use crate::models::product::Product;

use super::result::{RuleFinding, Severity};

pub fn validate(product: &Product) -> Vec<RuleFinding> {
    let mut findings = Vec::new();

    let inci = product.inci.to_uppercase();

    //
    // Demo / proof of concept
    // Senere erstattes af egentligt regelsæt
    //
    let restricted_ingredients = ["FORMALDEHYDE", "HYDROQUINONE", "METHYL EUGENOL"];

    for ingredient in restricted_ingredients {
        if inci.contains(ingredient) {
            findings.push(RuleFinding {
                rule_id: "INCI001".into(),
                severity: Severity::Error,
                message: format!("Potential restricted ingredient detected: '{}'", ingredient),
            });
        }
    }

    findings
}
