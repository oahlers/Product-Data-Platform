use crate::models::product::Product;

use super::result::{RuleFinding, Severity};

pub fn validate(product: &Product) -> Vec<RuleFinding> {
    let mut findings = Vec::new();

    let text = format!(
        "{} {} {}",
        product.product_name, product.description, product.marketing_text,
    )
    .to_lowercase();

    //
    // Test rule
    //
    //
    // Medicinal claims
    //
    let medicinal_claims = [
        "cure",
        "cures",
        "heal",
        "heals",
        "treat",
        "treats",
        "helbreder",
        "behandler",
        "lægelig",
        "medicinsk",
        "läker",
        "behandlar",
        "helbreder",
        "behandler",
        "medisinsk",
    ];

    for claim in medicinal_claims {
        if text.contains(claim) {
            findings.push(RuleFinding {
                rule_id: "TXT001".into(),
                severity: Severity::Error,
                message: format!("Potential medicinal claim detected: '{}'", claim),
            });
        }
    }

    //
    // Greenwashing claims
    //
    let greenwashing_claims = [
        "eco friendly",
        "eco-friendly",
        "green product",
        "environmentally friendly",
        "climate neutral",
        "100% natural",
        "miljøvenlig",
        "miljøvennlig",
        "miljövänlig",
        "bæredygtig",
        "bærekraftig",
        "hållbar",
    ];

    for claim in greenwashing_claims {
        if text.contains(claim) {
            findings.push(RuleFinding {
                rule_id: "TXT002".into(),
                severity: Severity::Warning,
                message: format!("Potential greenwashing claim detected: '{}'", claim),
            });
        }
    }

    findings
}
