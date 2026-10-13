use serde::Serialize;

use crate::{models::product::Product, storage::golden_record_source::GoldenRecordProduct};

#[derive(Debug, Clone, Serialize)]
pub struct UnifiedProduct {
    // ---------------------------------------------------------
    // IDENTIFIERS
    // ---------------------------------------------------------
    pub sku: String,
    pub ean: Option<String>,
    pub product_id: Option<String>,
    pub item_number: Option<String>,

    // ---------------------------------------------------------
    // INRIVER PRODUCT DATA
    // ---------------------------------------------------------
    pub brand: String,
    pub product_name: String,
    pub description: String,
    pub marketing_text: String,
    pub inci: String,
    pub warnings: String,
    pub directions: String,

    pub gender: Option<String>,

    pub top_notes: String,
    pub heart_notes: String,
    pub base_notes: String,

    // ---------------------------------------------------------
    // M3 / GOLDEN RECORD
    // ---------------------------------------------------------
    pub m3_description: Option<String>,
    pub m3_status: Option<String>,
    pub m3_item_type: Option<String>,
    pub m3_item_group: Option<String>,
    pub m3_item_class: Option<String>,
    pub m3_unit: Option<String>,
    pub m3_country: Option<String>,

    // Keep these raw until the M3 dimension semantics/units
    // have been verified.
    pub m3_dimension_1: Option<f64>,
    pub m3_dimension_2: Option<f64>,
    pub m3_dimension_3: Option<f64>,
    pub m3_gross_weight: Option<f64>,
    pub m3_net_weight: Option<f64>,
    pub m3_volume: Option<f64>,

    // ---------------------------------------------------------
    // EXISTING NORMALIZED PRODUCT FIELDS
    // ---------------------------------------------------------
    pub width_mm: Option<f64>,
    pub depth_mm: Option<f64>,
    pub height_mm: Option<f64>,

    pub primary_volume_mm3: Option<f64>,

    pub net_weight_g: Option<f64>,
    pub gross_weight_g: Option<f64>,

    pub dangerous_goods: Option<String>,
    pub danger_class: Option<String>,
    pub un_number: Option<String>,
    pub packing_group: Option<String>,
    pub flash_point: Option<f64>,

    pub alcohol_percent: Option<f64>,

    pub pallet_gtin: Option<String>,
    pub items_per_pallet: Option<u32>,
    pub shippers_per_pallet: Option<u32>,
    pub pallet_height_mm: Option<f64>,
    pub pallet_weight_g: Option<f64>,

    // ---------------------------------------------------------
    // SOURCE STATUS
    // ---------------------------------------------------------
    pub has_inriver_data: bool,
    pub has_golden_record_data: bool,
}

impl UnifiedProduct {
    pub fn merge(product: Product, golden_record: Option<GoldenRecordProduct>) -> Self {
        let golden = golden_record.as_ref();

        Self {
            // Identifiers
            sku: product.sku.clone(),

            ean: product
                .upc_code
                .clone()
                .or_else(|| golden.and_then(|g| g.ean.clone())),

            product_id: golden.and_then(|g| g.product_id.clone()),

            item_number: golden.map(|g| g.item_number.clone()),

            // Inriver
            brand: product.brand.clone(),
            product_name: product.product_name.clone(),
            description: product.description.clone(),
            marketing_text: product.marketing_text.clone(),
            inci: product.inci.clone(),
            warnings: product.warnings.clone(),
            directions: product.directions.clone(),

            gender: product.gender.clone(),

            top_notes: product.top_notes.clone(),
            heart_notes: product.heart_notes.clone(),
            base_notes: product.base_notes.clone(),

            // M3
            m3_description: golden.and_then(|g| g.m3_description.clone()),

            m3_status: golden.and_then(|g| g.m3_status.clone()),

            m3_item_type: golden.and_then(|g| g.m3_item_type.clone()),

            m3_item_group: golden.and_then(|g| g.m3_item_group.clone()),

            m3_item_class: golden.and_then(|g| g.m3_item_class.clone()),

            m3_unit: golden.and_then(|g| g.m3_unit.clone()),

            m3_country: golden.and_then(|g| g.m3_country.clone()),

            m3_dimension_1: golden.and_then(|g| g.dimension_1),

            m3_dimension_2: golden.and_then(|g| g.dimension_2),

            m3_dimension_3: golden.and_then(|g| g.dimension_3),

            m3_gross_weight: golden.and_then(|g| g.gross_weight),

            m3_net_weight: golden.and_then(|g| g.net_weight),

            m3_volume: golden.and_then(|g| g.volume),

            // Existing normalized fields
            width_mm: product.width_mm,
            depth_mm: product.depth_mm,
            height_mm: product.height_mm,

            primary_volume_mm3: product.primary_volume_mm3,

            net_weight_g: product.net_weight_g,
            gross_weight_g: product.gross_weight_g,

            dangerous_goods: product.dangerous_goods.clone(),
            danger_class: product.danger_class.clone(),
            un_number: product.un_number.clone(),
            packing_group: product.packing_group.clone(),
            flash_point: product.flash_point,

            alcohol_percent: product.alcohol_percent,

            pallet_gtin: product.pallet_gtin.clone(),
            items_per_pallet: product.items_per_pallet,
            shippers_per_pallet: product.shippers_per_pallet,
            pallet_height_mm: product.pallet_height_mm,
            pallet_weight_g: product.pallet_weight_g,

            // Sources
            has_inriver_data: true,
            has_golden_record_data: golden.is_some(),
        }
    }
}
