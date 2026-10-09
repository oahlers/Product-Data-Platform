use std::sync::Arc;
use axum::{extract::{Path, State}, http::StatusCode, response::IntoResponse, routing::{get, post}, Json, Router};
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
        .layer(TraceLayer::new_for_http()).layer(CorsLayer::permissive()).with_state(state)
}

async fn health() -> Json<serde_json::Value> { Json(json!({"status":"ok"})) }
async fn products(State(s): State<Arc<AppState>>) -> impl IntoResponse {
    match s.store.products().await { Ok(v) => (StatusCode::OK, Json(json!(v))), Err(e) => error(e) }
}
async fn process_all(State(s): State<Arc<AppState>>) -> impl IntoResponse {
    match orchestrator::process_all(s).await { Ok(v) => (StatusCode::OK, Json(json!(v))), Err(e) => error(e) }
}
async fn process_sku(State(s): State<Arc<AppState>>, Path(sku): Path<String>) -> impl IntoResponse {
    match orchestrator::process_sku(s, &sku).await { Ok(v) => (StatusCode::OK, Json(json!(v))), Err(e) => error(e) }
}
async fn result(State(s): State<Arc<AppState>>, Path(sku): Path<String>) -> impl IntoResponse {
    match s.store.result(&sku).await { Ok(Some(v)) => (StatusCode::OK, Json(json!(v))), Ok(None) => (StatusCode::NOT_FOUND, Json(json!({"error":"result not found"}))), Err(e) => error(e) }
}
fn error(e: anyhow::Error) -> (StatusCode, Json<serde_json::Value>) { (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error":e.to_string()}))) }
