use std::sync::Arc;

use crate::{
    cache::product_cache::ProductCache,
    storage::json_store::JsonStore,
};

#[derive(Clone)]
pub struct AppState {
    pub product_cache: Arc<ProductCache>,
    pub result_store: Arc<JsonStore>,
}

impl AppState {
    pub fn new(
        product_cache: Arc<ProductCache>,
        result_store: Arc<JsonStore>,
    ) -> Self {

        Self {
            product_cache,
            result_store,
        }
    }
}