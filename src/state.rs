use std::sync::Arc;

use crate::{
    ai::config::AiConfig,
    storage::{json_store::JsonStore, product_source::ProductSource},
};

#[derive(Clone)]
pub struct AppState {
    pub result_store: Arc<JsonStore>,
    pub product_source: Arc<dyn ProductSource>,
    pub ai_config: Arc<AiConfig>,
}

impl AppState {
    pub fn new(
        result_store: Arc<JsonStore>,
        product_source: Arc<dyn ProductSource>,
        ai_config: Arc<AiConfig>,
    ) -> Self {
        Self {
            result_store,
            product_source,
            ai_config,
        }
    }
}
