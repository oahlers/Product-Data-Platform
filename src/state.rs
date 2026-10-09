use std::sync::Arc;

use crate::storage::{excel_store::ExcelStore, json_store::JsonStore};

#[derive(Clone)]
pub struct AppState {
    pub product_store: Arc<ExcelStore>,
    pub result_store: Arc<JsonStore>,
}

impl AppState {
    pub fn new(
        product_store: Arc<ExcelStore>,
        result_store: Arc<JsonStore>,
    ) -> Self {
        Self {
            product_store,
            result_store,
        }
    }
}
