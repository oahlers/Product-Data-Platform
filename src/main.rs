mod agents;
mod ai;
mod api;
mod models;
mod orchestrator;
mod scheduler;
mod state;
mod status_engine;
mod storage;

use anyhow::Result;
use state::AppState;
use std::{net::SocketAddr, sync::Arc};
use storage::{
    inriver_config::InriverConfig, inriver_source::InriverSource, json_store::JsonStore,
    product_source::ProductSource,
};
use tokio::net::TcpListener;
use tracing::info;

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive("product_data_platform=info".parse()?),
        )
        .init();

    dotenvy::dotenv().ok();

    let inriver_config = InriverConfig::from_env()?;

    let inriver_source = InriverSource::new(inriver_config)?;

    let product_source: Arc<dyn ProductSource> = Arc::new(inriver_source);

    let result_store = Arc::new(JsonStore::new("data/products.json", "results"));

    result_store.ensure_files().await?;

    let state = Arc::new(AppState::new(result_store, product_source));

    let mut scheduler = scheduler::start(Arc::clone(&state)).await?;
    let app = api::router(state);
    let address = SocketAddr::from(([0, 0, 0, 0], 3000));
    let listener = TcpListener::bind(address).await?;
    info!(%address, "Product Data Platform started");

    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await?;

    scheduler.shutdown().await?;
    Ok(())
}

async fn shutdown_signal() {
    let _ = tokio::signal::ctrl_c().await;
}
