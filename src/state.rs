use std::sync::Arc;
use crate::storage::json_store::JsonStore;

#[derive(Clone)]
pub struct AppState { pub store: Arc<JsonStore> }
impl AppState { pub fn new(store: Arc<JsonStore>) -> Self { Self { store } } }
