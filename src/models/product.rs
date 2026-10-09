use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Product {
    pub sku: String,
    pub brand: String,
    pub product_name: String,
    #[serde(default)] pub description: String,
    #[serde(default)] pub marketing_text: String,
    #[serde(default)] pub inci: String,
    #[serde(default)] pub warnings: String,
    #[serde(default)] pub directions: String,
    pub net_weight_g: Option<f64>,
    pub gross_weight_g: Option<f64>,
    pub width_mm: Option<f64>,
    pub depth_mm: Option<f64>,
    pub height_mm: Option<f64>,
    #[serde(default = "Utc::now")]
    pub modified_at: DateTime<Utc>,
}
