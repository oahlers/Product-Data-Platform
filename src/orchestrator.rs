use std::sync::Arc;

use anyhow::{anyhow, Result};
use chrono::Utc;
use futures::{stream, StreamExt, TryStreamExt};
use tokio::join;

use crate::{
    agents,
    models::{product::Product, result::SkuResult},
    state::AppState,
    status_engine,
    storage::sync_checkpoint::SyncCheckpointStore,
};

const PROCESS_ALL_CONCURRENCY: usize = 3;
const PROCESS_MODIFIED_CONCURRENCY: usize = 3;

pub async fn process_product(state: Arc<AppState>, product: Product) -> Result<SkuResult> {
    let (validation, text_qa, inci, translation) = join!(
        agents::validator::run(&product),
        agents::text_qa::run(&product),
        agents::inci::run(&product),
        agents::translation::run(&product)
    );

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

pub async fn process_sku(state: Arc<AppState>, sku: &str) -> Result<SkuResult> {
    let product = state
        .product_source
        .product(sku)
        .await?
        .ok_or_else(|| anyhow!("SKU not found: {sku}"))?;

    process_product(state, product).await
}

pub async fn process_all(state: Arc<AppState>) -> Result<Vec<SkuResult>> {
    let products = state.product_source.products().await?;

    tracing::info!(
        product_count = products.len(),
        concurrency = PROCESS_ALL_CONCURRENCY,
        "Starting full product processing"
    );

    let results = stream::iter(products)
        .map(|product| {
            let state = Arc::clone(&state);

            async move { process_product(state, product).await }
        })
        .buffer_unordered(PROCESS_ALL_CONCURRENCY)
        .try_collect::<Vec<SkuResult>>()
        .await?;

    tracing::info!(
        processed_count = results.len(),
        "Full product processing completed"
    );

    Ok(results)
}

pub async fn process_modified(state: Arc<AppState>) -> Result<Vec<SkuResult>> {
    let checkpoint_store = SyncCheckpointStore::new("data/inriver_last_sync.txt");

    let modified_since = checkpoint_store.load().await?;

    // Checkpointet markeres før opslaget.
    // Ændringer under kørslen bliver dermed taget med næste gang.
    let scan_started_at = Utc::now();

    tracing::info!(
        modified_since = %modified_since.to_rfc3339(),
        "Starting incremental Inriver processing"
    );

    let modified_since_value = modified_since.to_rfc3339();

    let products = state
        .product_source
        .modified_products(&modified_since_value)
        .await?;

    let product_count = products.len();

    tracing::info!(
        product_count = product_count,
        concurrency = PROCESS_MODIFIED_CONCURRENCY,
        "Processing modified Inriver products"
    );

    let results = stream::iter(products)
        .map(|product| {
            let state = Arc::clone(&state);

            async move { process_product(state, product).await }
        })
        .buffer_unordered(PROCESS_MODIFIED_CONCURRENCY)
        .try_collect::<Vec<SkuResult>>()
        .await?;

    // Checkpointet må først flyttes, når hele behandlingen lykkes.
    checkpoint_store.save(scan_started_at).await?;

    tracing::info!(
        product_count = product_count,
        processed_count = results.len(),
        checkpoint = %scan_started_at.to_rfc3339(),
        "Incremental Inriver processing completed"
    );

    Ok(results)
}
