use crate::models::product::Product;

use super::result::{RuleFinding, Severity};

pub fn validate(product: &Product) -> Vec<RuleFinding> {
    let mut findings = Vec::new();

    if let (Some(items_per_shipper), Some(shippers_per_pallet), Some(items_per_pallet)) = (
        product.items_per_shipper,
        product.shippers_per_pallet,
        product.items_per_pallet,
    ) {
        let expected = items_per_shipper * shippers_per_pallet;

        if expected != items_per_pallet {
            findings.push(RuleFinding {
                rule_id: "PAL001".into(),
                severity: Severity::Error,
                message: format!(
                    "ItemsPerPallet ({}) differs from calculated value ({})",
                    items_per_pallet, expected
                ),
            });
        }
    }

    if let Some(height) = product.pallet_height_mm {
        if height > 1800.0 {
            findings.push(RuleFinding {
                rule_id: "PAL002".into(),
                severity: Severity::Warning,
                message: format!(
                    "Pallet height {:.2} mm exceeds recommended maximum of 1800 mm",
                    height
                ),
            });
        }
    }

    if product.pallet_gtin.is_none() {
        findings.push(RuleFinding {
            rule_id: "PAL003".into(),
            severity: Severity::Warning,
            message: "Pallet GTIN is missing".into(),
        });
    }

    findings
}
