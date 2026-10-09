use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModuleResult {
    pub status: String,
    pub findings: Vec<String>,
    pub provider: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Labels {
    pub da: String,
    pub sv: String,
    pub no: String,
    pub fi: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkuResult {
    pub sku: String,
    pub validation: ModuleResult,
    pub text_qa: ModuleResult,
    pub inci: ModuleResult,
    pub translation: ModuleResult,
    pub labels: Labels,
    pub overall_status: String,
    pub processed_at: DateTime<Utc>,
}
