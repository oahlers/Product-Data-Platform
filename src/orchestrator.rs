use std::{collections::HashMap, sync::Arc};

use anyhow::{anyhow, Result};
use chrono::Utc;
use futures::{stream, StreamExt, TryStreamExt};
use tokio::join;

use crate::{
    agents,
    models::{product::Product, result::SkuResult, unified_product::UnifiedProduct},
    state::AppState,
    status_engine,
    storage::{golden_record_source::GoldenRecordProduct, sync_checkpoint::SyncCheckpointStore},
};

const PROCESS_ALL_CONCURRENCY: usize = 3;
const PROCESS_MODIFIED_CONCURRENCY: usize = 3;

pub async fn get_unified_product(
    state: &Arc<AppState>,
    item_number: &str,
) -> Result<UnifiedProduct> {
    let golden_record = state
        .golden_record_source
        .get_by_item_number(item_number)
        .await?
        .ok_or_else(|| anyhow!("Golden Record not found: {}", item_number))?;

    let product_id = golden_record
        .product_id
        .clone()
        .ok_or_else(|| anyhow!("Golden Record {} has no Inriver ProductID", item_number))?;

    let product = state
        .product_source
        .product_by_product_id(&product_id)
        .await?
        .ok_or_else(|| anyhow!("Inriver product not found for ProductID: {}", product_id))?;

    Ok(UnifiedProduct::merge(product, Some(golden_record)))
}

async fn process_product_with_golden_record(
    state: Arc<AppState>,
    product: Product,
    golden_record: Option<GoldenRecordProduct>,
) -> Result<SkuResult> {
    let unified = UnifiedProduct::merge(product.clone(), golden_record);

    let (mut validation, text_qa, inci, translation) = join!(
        agents::validator::run(&product),
        agents::text_qa::run(&product),
        agents::inci::run(&product),
        agents::translation::run(&product)
    );

    let unified_validation = agents::validator::validate_unified(&unified);

    validation.findings.extend(unified_validation.findings);

    if unified_validation.status == "FAIL" {
        validation.status = "FAIL".into();
    } else if unified_validation.status == "WARNING" && validation.status == "PASS" {
        validation.status = "WARNING".into();
    }

    let labels = agents::labels::run(&product).await;

    let overall_status = status_engine::calculate(&validation, &text_qa, &inci);

    let result = SkuResult {
        sku: product.sku.clone(),
        validation,
        text_qa,
        inci,
        translation,
        labels,
        overall_status,
        processed_at: Utc::now(),
    };

    state.result_store.save_result(&result).await?;

    Ok(result)
}

pub async fn process_item_number(state: Arc<AppState>, item_number: &str) -> Result<SkuResult> {
    let golden_record = state
        .golden_record_source
        .get_by_item_number(item_number)
        .await?
        .ok_or_else(|| anyhow!("Golden Record not found: {}", item_number))?;

    let product_id = golden_record
        .product_id
        .clone()
        .ok_or_else(|| anyhow!("Golden Record {} has no Inriver ProductID", item_number))?;

    let product = state
        .product_source
        .product_by_product_id(&product_id)
        .await?
        .ok_or_else(|| anyhow!("Inriver product not found for ProductID: {}", product_id))?;

    process_product_with_golden_record(state, product, Some(golden_record)).await
}

pub async fn process_product(state: Arc<AppState>, product: Product) -> Result<SkuResult> {
    let golden_record = state
        .golden_record_source
        .get_by_product_id(&product.sku)
        .await?;

    process_product_with_golden_record(state, product, golden_record).await
}

pub async fn process_sku(state: Arc<AppState>, sku: &str) -> Result<SkuResult> {
    let product = state
        .product_source
        .product(sku)
        .await?
        .ok_or_else(|| anyhow!("SKU not found: {}", sku))?;

    process_product(state, product).await
}

pub async fn process_all(state: Arc<AppState>) -> Result<Vec<SkuResult>> {
    let products = state.product_source.products().await?;

    let golden_records: HashMap<String, GoldenRecordProduct> =
        state.golden_record_source.all_by_product_id().await?;

    let golden_records = Arc::new(golden_records);

    tracing::info!(
        product_count = products.len(),
        golden_record_count = golden_records.len(),
        concurrency = PROCESS_ALL_CONCURRENCY,
        "Starting full unified product processing"
    );

    let results = stream::iter(products)
        .map(|product| {
            let state = Arc::clone(&state);

            let golden_records = Arc::clone(&golden_records);

            async move {
                let golden_record = golden_records.get(&product.sku).cloned();

                process_product_with_golden_record(state, product, golden_record).await
            }
        })
        .buffer_unordered(PROCESS_ALL_CONCURRENCY)
        .try_collect::<Vec<SkuResult>>()
        .await?;

    tracing::info!(
        processed_count = results.len(),
        "Full unified product processing completed"
    );

    Ok(results)
}

pub async fn process_modified(state: Arc<AppState>) -> Result<Vec<SkuResult>> {
    let checkpoint_store = SyncCheckpointStore::new("data/inriver_last_sync.txt");

    let modified_since = checkpoint_store.load().await?;

    // Markér tidspunktet før opslaget.
    // Ændringer under kørslen bliver taget med
    // ved næste scheduler-kørsel.
    let scan_started_at = Utc::now();

    tracing::info!(
        modified_since =
            %modified_since.to_rfc3339(),
        "Starting incremental unified processing"
    );

    let modified_since_value = modified_since.to_rfc3339();

    let products = state
        .product_source
        .modified_products(&modified_since_value)
        .await?;

    let product_count = products.len();

    let golden_records: HashMap<String, GoldenRecordProduct> =
        state.golden_record_source.all_by_product_id().await?;

    let golden_records = Arc::new(golden_records);

    tracing::info!(
        product_count = product_count,
        golden_record_count = golden_records.len(),
        concurrency = PROCESS_MODIFIED_CONCURRENCY,
        "Processing modified unified products"
    );

    let results = stream::iter(products)
        .map(|product| {
            let state = Arc::clone(&state);

            let golden_records = Arc::clone(&golden_records);

            async move {
                let golden_record = golden_records.get(&product.sku).cloned();

                process_product_with_golden_record(state, product, golden_record).await
            }
        })
        .buffer_unordered(PROCESS_MODIFIED_CONCURRENCY)
        .try_collect::<Vec<SkuResult>>()
        .await?;

    // Flyt kun checkpointet, hvis hele kørslen lykkes.
    checkpoint_store.save(scan_started_at).await?;

    tracing::info!(
        product_count = product_count,
        processed_count = results.len(),
        checkpoint =
            %scan_started_at.to_rfc3339(),
        "Incremental unified processing completed"
    );

    Ok(results)
}
