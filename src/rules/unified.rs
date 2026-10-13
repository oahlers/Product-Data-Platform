use crate::models::unified_product::UnifiedProduct;

use super::result::{RuleFinding, Severity};

pub fn validate(product: &UnifiedProduct) -> Vec<RuleFinding> {
    let mut findings = Vec::new();

    validate_sources(product, &mut findings);
    validate_identifiers(product, &mut findings);
    validate_dimensions(product, &mut findings);
    validate_volume(product, &mut findings);
    validate_weights(product, &mut findings);

    findings
}

fn validate_sources(product: &UnifiedProduct, findings: &mut Vec<RuleFinding>) {
    if !product.has_inriver_data {
        findings.push(RuleFinding {
            rule_id: "UNI001".into(),
            severity: Severity::Error,
            message: "Product has no Inriver data".into(),
        });
    }

    if !product.has_golden_record_data {
        findings.push(RuleFinding {
            rule_id: "UNI002".into(),
            severity: Severity::Warning,
            message: "Product has no Golden Record / M3 data".into(),
        });
    }
}

fn validate_identifiers(product: &UnifiedProduct, findings: &mut Vec<RuleFinding>) {
    if product.item_number.is_none() {
        findings.push(RuleFinding {
            rule_id: "UNI003".into(),
            severity: Severity::Warning,
            message: "Product has no M3 ItemNumber".into(),
        });
    }

    if product.product_id.is_none() {
        findings.push(RuleFinding {
            rule_id: "UNI004".into(),
            severity: Severity::Warning,
            message: "Product has no Inriver ProductID in Golden Record".into(),
        });
    }

    if product.ean.is_none() {
        findings.push(RuleFinding {
            rule_id: "UNI005".into(),
            severity: Severity::Warning,
            message: "Product has no EAN".into(),
        });
    }
}

fn validate_dimensions(product: &UnifiedProduct, findings: &mut Vec<RuleFinding>) {
    if product.width_mm.is_none() || product.depth_mm.is_none() || product.height_mm.is_none() {
        findings.push(RuleFinding {
            rule_id: "UNI010".into(),
            severity: Severity::Warning,
            message: "One or more normalized Inriver dimensions are missing".into(),
        });
    }

    if product.has_golden_record_data
        && (product.m3_dimension_1.is_none()
            || product.m3_dimension_2.is_none()
            || product.m3_dimension_3.is_none())
    {
        findings.push(RuleFinding {
            rule_id: "UNI011".into(),
            severity: Severity::Warning,
            message: "One or more M3 dimensions are missing".into(),
        });
    }
}

fn validate_volume(product: &UnifiedProduct, findings: &mut Vec<RuleFinding>) {
    if let (Some(width), Some(depth), Some(height), Some(volume)) = (
        product.width_mm,
        product.depth_mm,
        product.height_mm,
        product.primary_volume_mm3,
    ) {
        let calculated = width * depth * height;

        if !approximately_equal(calculated, volume, 0.01) {
            findings.push(RuleFinding {
                rule_id: "UNI020".into(),
                severity: Severity::Warning,
                message: format!(
                    "Normalized volume mismatch: dimensions calculate to {:.2} mm3 but stored volume is {:.2} mm3",
                    calculated, volume
                ),
            });
        }
    }

    if product.has_golden_record_data && product.m3_volume.is_none() {
        findings.push(RuleFinding {
            rule_id: "UNI021".into(),
            severity: Severity::Warning,
            message: "M3 volume is missing".into(),
        });
    }
}

fn validate_weights(product: &UnifiedProduct, findings: &mut Vec<RuleFinding>) {
    if let (Some(net), Some(gross)) = (product.net_weight_g, product.gross_weight_g) {
        if net > gross {
            findings.push(RuleFinding {
                rule_id: "UNI030".into(),
                severity: Severity::Error,
                message: format!(
                    "Inriver net weight ({:.2} g) exceeds gross weight ({:.2} g)",
                    net, gross
                ),
            });
        }
    }

    if let (Some(net), Some(gross)) = (product.m3_net_weight, product.m3_gross_weight) {
        if net > gross {
            findings.push(RuleFinding {
                rule_id: "UNI031".into(),
                severity: Severity::Error,
                message: format!(
                    "M3 net weight ({:.2}) exceeds M3 gross weight ({:.2})",
                    net, gross
                ),
            });
        }
    }
}

fn approximately_equal(left: f64, right: f64, tolerance: f64) -> bool {
    if left == right {
        return true;
    }

    let largest = left.abs().max(right.abs());

    if largest == 0.0 {
        return true;
    }

    ((left - right).abs() / largest) <= tolerance
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_missing_golden_record() {
        let product = UnifiedProduct {
            sku: "TEST".into(),
            ean: None,
            product_id: None,
            item_number: None,

            brand: "TEST".into(),
            product_name: "Test Product".into(),
            description: String::new(),
            marketing_text: String::new(),
            inci: String::new(),
            warnings: String::new(),
            directions: String::new(),

            gender: None,

            top_notes: String::new(),
            heart_notes: String::new(),
            base_notes: String::new(),

            m3_description: None,
            m3_status: None,
            m3_item_type: None,
            m3_item_group: None,
            m3_item_class: None,
            m3_unit: None,
            m3_country: None,

            m3_dimension_1: None,
            m3_dimension_2: None,
            m3_dimension_3: None,
            m3_gross_weight: None,
            m3_net_weight: None,
            m3_volume: None,

            width_mm: None,
            depth_mm: None,
            height_mm: None,
            primary_volume_mm3: None,

            net_weight_g: None,
            gross_weight_g: None,

            dangerous_goods: None,
            danger_class: None,
            un_number: None,
            packing_group: None,
            flash_point: None,

            alcohol_percent: None,

            pallet_gtin: None,
            items_per_pallet: None,
            shippers_per_pallet: None,
            pallet_height_mm: None,
            pallet_weight_g: None,

            has_inriver_data: true,
            has_golden_record_data: false,
        };

        let findings = validate(&product);

        assert!(findings.iter().any(|finding| finding.rule_id == "UNI002"));
    }
}
