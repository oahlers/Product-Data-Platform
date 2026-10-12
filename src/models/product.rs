use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Product {
    pub sku: String,

    #[serde(default)]
    pub upc_code: Option<String>,

    #[serde(default)]
    pub brand: String,

    #[serde(default)]
    pub product_name: String,

    #[serde(default)]
    pub description: String,

    #[serde(default)]
    pub marketing_text: String,

    #[serde(default)]
    pub inci: String,

    #[serde(default)]
    pub warnings: String,

    #[serde(default)]
    pub directions: String,

    #[serde(default)]
    pub width_mm: Option<f64>,

    #[serde(default)]
    pub depth_mm: Option<f64>,

    #[serde(default)]
    pub height_mm: Option<f64>,

    #[serde(default)]
    pub primary_volume_mm3: Option<f64>,

    #[serde(default)]
    pub secondary_width_mm: Option<f64>,

    #[serde(default)]
    pub secondary_depth_mm: Option<f64>,

    #[serde(default)]
    pub secondary_height_mm: Option<f64>,

    #[serde(default)]
    pub secondary_volume_mm3: Option<f64>,

    #[serde(default)]
    pub net_weight_g: Option<f64>,

    #[serde(default)]
    pub gross_weight_g: Option<f64>,

    #[serde(default)]
    pub piece_weight_g: Option<f64>,

    #[serde(default)]
    pub items_per_prepack: Option<u32>,

    #[serde(default)]
    pub prepacks_per_shipper: Option<u32>,

    #[serde(default)]
    pub items_per_shipper: Option<u32>,

    #[serde(default)]
    pub shipper_width_mm: Option<f64>,

    #[serde(default)]
    pub shipper_depth_mm: Option<f64>,

    #[serde(default)]
    pub shipper_height_mm: Option<f64>,

    #[serde(default)]
    pub shipper_weight_g: Option<f64>,

    #[serde(default)]
    pub shipper_volume_mm3: Option<f64>,

    #[serde(default)]
    pub case_weight_g: Option<f64>,

    #[serde(default)]
    pub carton_weight_g: Option<f64>,

    #[serde(default)]
    pub pieces_per_case: Option<u32>,

    #[serde(default)]
    pub cases_per_carton: Option<u32>,

    #[serde(default)]
    pub shippers_per_pallet: Option<u32>,

    #[serde(default)]
    pub items_per_pallet: Option<u32>,

    #[serde(default)]
    pub pallet_height_mm: Option<f64>,

    #[serde(default)]
    pub pallet_weight_g: Option<f64>,

    #[serde(default)]
    pub pallet_gtin: Option<String>,

    #[serde(default)]
    pub cube: Option<f64>,

    #[serde(default)]
    pub interspace_percent: Option<f64>,

    #[serde(default)]
    pub packaging_material_layers: Option<u32>,

    #[serde(default)]
    pub paper_cardboard_g: Option<f64>,

    #[serde(default)]
    pub glass_g: Option<f64>,

    #[serde(default)]
    pub textiles_wood_g: Option<f64>,

    #[serde(default)]
    pub tinplate_g: Option<f64>,

    #[serde(default)]
    pub aluminium_g: Option<f64>,

    #[serde(default)]
    pub plastic_g: Option<f64>,

    #[serde(default = "Utc::now")]
    pub modified_at: DateTime<Utc>,
}
