use crate::models::product::Product;

use super::result::{RuleFinding, Severity};

pub fn validate(product: &Product) -> Vec<RuleFinding> {
    let mut findings = Vec::new();

    if let (Some(width), Some(depth)) = (product.width_mm, product.depth_mm) {
        if width < depth {
            findings.push(RuleFinding {
                rule_id: "DIM001".into(),
                severity: Severity::Warning,
                message: format!(
                    "Width ({:.2} mm) is smaller than depth ({:.2} mm)",
                    width, depth
                ),
            });
        }
    }

    findings
}
