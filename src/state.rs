use std::sync::Arc;

use crate::{
    cache::product_cache::ProductCache,
    storage::{json_store::JsonStore, product_source::ProductSource},
};

#[derive(Clone)]
pub struct AppState {
    pub product_cache: Arc<ProductCache>,
    pub result_store: Arc<JsonStore>,
    pub product_source: Arc<dyn ProductSource>,
}

impl AppState {
    pub fn new(
        product_cache: Arc<ProductCache>,
        result_store: Arc<JsonStore>,
        product_source: Arc<dyn ProductSource>,
    ) -> Self {
        Self {
            product_cache,
            result_store,
            product_source,
        }
    }
}
