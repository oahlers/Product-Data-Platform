use std::sync::Arc;

use anyhow::{anyhow, Result};
use chrono::Utc;
use tokio::join;

use crate::{
    agents,
    models::result::SkuResult,
    state::AppState,
    status_engine,
};

pub async fn process_sku(
    state: Arc<AppState>,
    sku: &str,
) -> Result<SkuResult> {

    let product = state
        .product_cache
        .get(sku)
        .cloned()
        .ok_or_else(|| anyhow!(
            "SKU not found: {sku}"
        ))?;

    let (
        validation,
        text_qa,
        inci,
        translation
    ) = join!(
        agents::validator::run(&product),
        agents::text_qa::run(&product),
        agents::inci::run(&product),
        agents::translation::run(&product)
    );

    let labels =
        agents::labels::run(&product).await;

    let overall_status =
        status_engine::calculate(
            &validation,
            &text_qa,
            &inci
        );

    let result = SkuResult {
        sku: product.sku,
        validation,
        text_qa,
        inci,
        translation,
        labels,
        overall_status,
        processed_at: Utc::now(),
    };

    state
        .result_store
        .save_result(&result)
        .await?;

    Ok(result)
}

pub async fn process_all(
    state: Arc<AppState>,
) -> Result<Vec<SkuResult>> {

    let products: Vec<_> = state
        .product_cache
        .values()
        .cloned()
        .collect();

    let mut results =
        Vec::with_capacity(products.len());

    for product in products {
        results.push(
            process_sku(
                Arc::clone(&state),
                &product.sku,
            )
            .await?
        );
    }

    Ok(results)
}