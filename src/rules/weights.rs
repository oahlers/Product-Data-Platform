use crate::{
    models::product::Product,
    validation::weight_validator::{validate, WeightValidationInput},
};

use super::result::{RuleFinding, Severity};

pub fn validate_product(product: &Product) -> Vec<RuleFinding> {
    let findings = validate(&WeightValidationInput {
        net_weight_g: product.net_weight_g,
        gross_weight_g: product.gross_weight_g,

        piece_weight_g: product.piece_weight_g,
        case_weight_g: product.case_weight_g,
        carton_weight_g: product.carton_weight_g,

        pieces_per_case: product.pieces_per_case,
        cases_per_carton: product.cases_per_carton,
    });

    findings
        .into_iter()
        .map(|message| RuleFinding {
            rule_id: "WEIGHT".into(),
            severity: Severity::Error,
            message,
        })
        .collect()
}
