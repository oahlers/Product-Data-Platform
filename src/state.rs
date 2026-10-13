use std::sync::Arc;

use crate::{
    ai::config::AiConfig,
    storage::{
        golden_record_source::GoldenRecordSource, json_store::JsonStore,
        product_source::ProductSource,
    },
};

#[derive(Clone)]
pub struct AppState {
    pub result_store: Arc<JsonStore>,
    pub product_source: Arc<dyn ProductSource>,
    pub golden_record_source: Arc<GoldenRecordSource>,
    pub ai_config: Arc<AiConfig>,
}

impl AppState {
    pub fn new(
        result_store: Arc<JsonStore>,
        product_source: Arc<dyn ProductSource>,
        golden_record_source: Arc<GoldenRecordSource>,
        ai_config: Arc<AiConfig>,
    ) -> Self {
        Self {
            result_store,
            product_source,
            golden_record_source,
            ai_config,
        }
    }
}
