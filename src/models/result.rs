use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModuleResult {
    pub status: String,

    #[serde(default)]
    pub findings: Vec<String>,

    #[serde(default)]
    pub recommendations: Vec<String>,

    #[serde(default)]
    pub provider: String,

    #[serde(default)]
    pub score: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Labels {
    #[serde(default)]
    pub da: String,

    #[serde(default)]
    pub sv: String,

    #[serde(default)]
    pub no: String,

    #[serde(default)]
    pub fi: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkuResult {
    pub sku: String,

    pub validation: ModuleResult,

    pub text_qa: ModuleResult,

    pub inci: ModuleResult,

    pub translation: ModuleResult,

    pub sustainability: ModuleResult,

    pub labels: Labels,

    pub overall_status: String,

    #[serde(default)]
    pub source_provider: String,

    pub processed_at: DateTime<Utc>,
}