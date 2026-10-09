use std::sync::Arc;

use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use serde_json::json;
use tower_http::{cors::CorsLayer, trace::TraceLayer};

use crate::{orchestrator, state::AppState};

pub fn router(state: Arc<AppState>) -> Router {
    Router::new()
        .route("/health", get(health))
        .route("/products", get(products))
        .route("/process-all", post(process_all))
        .route("/process/{sku}", post(process_sku))
        .route("/results/{sku}", get(result))
        .layer(TraceLayer::new_for_http())
        .layer(CorsLayer::permissive())
        .with_state(state)
}

async fn health() -> Json<serde_json::Value> {
    Json(json!({"status": "ok"}))
}

async fn products(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    match state.product_store.products().await {
        Ok(products) => (StatusCode::OK, Json(json!(products))),
        Err(error) => internal_error(error),
    }
}

async fn process_all(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    match orchestrator::process_all(state).await {
        Ok(results) => (StatusCode::OK, Json(json!(results))),
        Err(error) => internal_error(error),
    }
}

async fn process_sku(
    State(state): State<Arc<AppState>>,
    Path(sku): Path<String>,
) -> impl IntoResponse {
    match orchestrator::process_sku(state, &sku).await {
        Ok(result) => (StatusCode::OK, Json(json!(result))),
        Err(error) if error.to_string().starts_with("SKU not found:") => (
            StatusCode::NOT_FOUND,
            Json(json!({"error": error.to_string()})),
        ),
        Err(error) => internal_error(error),
    }
}

async fn result(
    State(state): State<Arc<AppState>>,
    Path(sku): Path<String>,
) -> impl IntoResponse {
    match state.result_store.result(&sku).await {
        Ok(Some(result)) => (StatusCode::OK, Json(json!(result))),
        Ok(None) => (
            StatusCode::NOT_FOUND,
            Json(json!({"error": "result not found"})),
        ),
        Err(error) => internal_error(error),
    }
}

fn internal_error(error: anyhow::Error) -> (StatusCode, Json<serde_json::Value>) {
    (
        StatusCode::INTERNAL_SERVER_ERROR,
        Json(json!({"error": error.to_string()})),
    )
}
