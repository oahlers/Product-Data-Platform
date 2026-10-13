use crate::models::product::Product;

use super::{dimensions, inci, pallet, result::RuleFinding, text_claims, weights};

pub fn validate_product(product: &Product) -> Vec<RuleFinding> {
    let mut findings = Vec::new();

    findings.extend(dimensions::validate(product));

    findings.extend(pallet::validate(product));

    findings.extend(weights::validate_product(product));

    findings.extend(text_claims::validate(product));

    findings.extend(inci::validate(product));

    findings
}
