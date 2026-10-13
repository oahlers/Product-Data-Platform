use std::sync::Arc;

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};

use serde::Deserialize;
use serde_json::json;

use tower_http::{cors::CorsLayer, trace::TraceLayer};

use crate::{orchestrator, state::AppState};

#[derive(Deserialize)]
struct SearchQuery {
    q: String,
}

pub fn router(state: Arc<AppState>) -> Router {
    Router::new()
        .route("/health", get(health))
        .route("/products", get(products))
        .route("/products/search", get(search_products))
        .route("/products/sku/{sku}", get(product_by_sku))
        .route("/products/ean/{ean}", get(product_by_ean))
        .route(
            "/products/unified/{item_number}",
            get(unified_product_by_sku),
        )
        .route(
            "/golden-record/{item_number}",
            get(golden_record_by_item_number),
        )
        .route("/process-all", post(process_all))
        .route("/process-modified", post(process_modified))
        .route("/process/item/{item_number}", post(process_item_number))
        .route("/process/{sku}", post(process_sku))
        .route("/results/{sku}", get(result))
        .layer(TraceLayer::new_for_http())
        .layer(CorsLayer::permissive())
        .with_state(state)
}

async fn health() -> Json<serde_json::Value> {
    Json(json!({
        "status": "ok"
    }))
}

async fn products(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    match state.product_source.products().await {
        Ok(products) => (StatusCode::OK, Json(json!(products))),
        Err(error) => internal_error(error),
    }
}

async fn search_products(
    State(state): State<Arc<AppState>>,
    Query(query): Query<SearchQuery>,
) -> impl IntoResponse {
    match state.product_source.search_products(&query.q).await {
        Ok(products) => (StatusCode::OK, Json(json!(products))),
        Err(error) => internal_error(error),
    }
}

async fn product_by_sku(
    State(state): State<Arc<AppState>>,
    Path(sku): Path<String>,
) -> impl IntoResponse {
    match state.product_source.product(&sku).await {
        Ok(Some(product)) => (StatusCode::OK, Json(json!(product))),

        Ok(None) => (
            StatusCode::NOT_FOUND,
            Json(json!({
                "error": format!("Product not found: {}", sku)
            })),
        ),

        Err(error) => internal_error(error),
    }
}

async fn product_by_ean(
    State(state): State<Arc<AppState>>,
    Path(ean): Path<String>,
) -> impl IntoResponse {
    match state.product_source.product_by_ean(&ean).await {
        Ok(Some(product)) => (StatusCode::OK, Json(json!(product))),

        Ok(None) => (
            StatusCode::NOT_FOUND,
            Json(json!({
                "error": format!("EAN not found: {}", ean)
            })),
        ),

        Err(error) => internal_error(error),
    }
}

async fn unified_product_by_sku(
    State(state): State<Arc<AppState>>,
    Path(item_number): Path<String>,
) -> impl IntoResponse {
    match orchestrator::get_unified_product(&state, &item_number).await {
        Ok(product) => (StatusCode::OK, Json(json!(product))),

        Err(error) if error.to_string().starts_with("Golden Record not found:") => (
            StatusCode::NOT_FOUND,
            Json(json!({
                "error": error.to_string()
            })),
        ),

        Err(error) if error.to_string().starts_with("Inriver product not found") => (
            StatusCode::NOT_FOUND,
            Json(json!({
                "error": error.to_string()
            })),
        ),

        Err(error) if error.to_string().contains("has no Inriver ProductID") => (
            StatusCode::NOT_FOUND,
            Json(json!({
                "error": error.to_string()
            })),
        ),

        Err(error) => internal_error(error),
    }
}

async fn golden_record_by_item_number(
    State(state): State<Arc<AppState>>,
    Path(item_number): Path<String>,
) -> impl IntoResponse {
    match state
        .golden_record_source
        .get_by_item_number(&item_number)
        .await
    {
        Ok(Some(product)) => (StatusCode::OK, Json(json!(product))),

        Ok(None) => (
            StatusCode::NOT_FOUND,
            Json(json!({
                "error": format!(
                    "Golden Record not found: {}",
                    item_number
                )
            })),
        ),

        Err(error) => internal_error(error),
    }
}

async fn process_all(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    match orchestrator::process_all(state).await {
        Ok(results) => (StatusCode::OK, Json(json!(results))),
        Err(error) => internal_error(error),
    }
}

async fn process_modified(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    match orchestrator::process_modified(state).await {
        Ok(results) => (StatusCode::OK, Json(json!(results))),
        Err(error) => internal_error(error),
    }
}

async fn process_item_number(
    State(state): State<Arc<AppState>>,
    Path(item_number): Path<String>,
) -> impl IntoResponse {
    match orchestrator::process_item_number(state, &item_number).await {
        Ok(result) => (StatusCode::OK, Json(json!(result))),

        Err(error) if error.to_string().starts_with("Golden Record not found:") => (
            StatusCode::NOT_FOUND,
            Json(json!({
                "error": error.to_string()
            })),
        ),

        Err(error) if error.to_string().starts_with("Inriver product not found") => (
            StatusCode::NOT_FOUND,
            Json(json!({
                "error": error.to_string()
            })),
        ),

        Err(error) if error.to_string().contains("has no Inriver ProductID") => (
            StatusCode::NOT_FOUND,
            Json(json!({
                "error": error.to_string()
            })),
        ),

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
            Json(json!({
                "error": error.to_string()
            })),
        ),

        Err(error) => internal_error(error),
    }
}

async fn result(State(state): State<Arc<AppState>>, Path(sku): Path<String>) -> impl IntoResponse {
    match state.result_store.result(&sku).await {
        Ok(Some(result)) => (StatusCode::OK, Json(json!(result))),

        Ok(None) => (
            StatusCode::NOT_FOUND,
            Json(json!({
                "error": "result not found"
            })),
        ),

        Err(error) => internal_error(error),
    }
}

fn internal_error(error: anyhow::Error) -> (StatusCode, Json<serde_json::Value>) {
    (
        StatusCode::INTERNAL_SERVER_ERROR,
        Json(json!({
            "error": error.to_string()
        })),
    )
}
