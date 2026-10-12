use std::sync::Arc;

use crate::storage::{json_store::JsonStore, product_source::ProductSource};

#[derive(Clone)]
pub struct AppState {
    pub result_store: Arc<JsonStore>,
    pub product_source: Arc<dyn ProductSource>,
}

impl AppState {
    pub fn new(result_store: Arc<JsonStore>, product_source: Arc<dyn ProductSource>) -> Self {
        Self {
            result_store,
            product_source,
        }
    }
}
