pub struct WeightValidationInput {
    pub net_weight_g: Option<f64>,
    pub gross_weight_g: Option<f64>,

    pub piece_weight_g: Option<f64>,
    pub case_weight_g: Option<f64>,
    pub carton_weight_g: Option<f64>,

    pub pieces_per_case: Option<u32>,
    pub cases_per_carton: Option<u32>,
}

pub fn validate(
    input: &WeightValidationInput,
) -> Vec<String> {

    let mut findings = Vec::new();

    //
    // Net vs Gross
    //
    if let (
        Some(net),
        Some(gross),
    ) = (
        input.net_weight_g,
        input.gross_weight_g,
    ) {
        if net > gross {
            findings.push(
                format!(
                    "Net weight ({:.2} g) exceeds gross weight ({:.2} g)",
                    net,
                    gross
                )
            );
        }
    }

    //
    // STK -> COL
    //
    if let (
        Some(piece_weight),
        Some(case_weight),
        Some(pieces_per_case),
    ) = (
        input.piece_weight_g,
        input.case_weight_g,
        input.pieces_per_case,
    ) {
        let expected =
            piece_weight *
            pieces_per_case as f64;

        if case_weight < expected {
            findings.push(
                format!(
                    "Case weight ({:.2} g) is lower than expected minimum ({:.2} g)",
                    case_weight,
                    expected
                )
            );
        }

        if case_weight > expected * 1.5 {
            findings.push(
                format!(
                    "Case weight ({:.2} g) appears unusually high compared to expected ({:.2} g)",
                    case_weight,
                    expected
                )
            );
        }
    }

    //
    // COL -> KAS
    //
    if let (
        Some(case_weight),
        Some(carton_weight),
        Some(cases_per_carton),
    ) = (
        input.case_weight_g,
        input.carton_weight_g,
        input.cases_per_carton,
    ) {
        let expected =
            case_weight *
            cases_per_carton as f64;

        if carton_weight < expected {
            findings.push(
                format!(
                    "Carton weight ({:.2} g) is lower than expected minimum ({:.2} g)",
                    carton_weight,
                    expected
                )
            );
        }

        if carton_weight > expected * 1.5 {
            findings.push(
                format!(
                    "Carton weight ({:.2} g) appears unusually high compared to expected ({:.2} g)",
                    carton_weight,
                    expected
                )
            );
        }
    }

    findings
}

#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    fn detects_invalid_net_weight() {

        let input = WeightValidationInput {
            net_weight_g: Some(150.0),
            gross_weight_g: Some(100.0),

            piece_weight_g: None,
            case_weight_g: None,
            carton_weight_g: None,

            pieces_per_case: None,
            cases_per_carton: None,
        };

        let findings = validate(&input);

        assert!(
            !findings.is_empty()
        );
    }

    #[test]
    fn accepts_valid_net_weight() {

        let input = WeightValidationInput {
            net_weight_g: Some(100.0),
            gross_weight_g: Some(150.0),

            piece_weight_g: None,
            case_weight_g: None,
            carton_weight_g: None,

            pieces_per_case: None,
            cases_per_carton: None,
        };

        let findings = validate(&input);

        assert!(
            findings.is_empty()
        );
    }

    #[test]
    fn detects_invalid_case_weight() {

        let input = WeightValidationInput {
            net_weight_g: None,
            gross_weight_g: None,

            piece_weight_g: Some(100.0),
            case_weight_g: Some(500.0),
            carton_weight_g: None,

            pieces_per_case: Some(6),
            cases_per_carton: None,
        };

        let findings = validate(&input);

        assert!(
            !findings.is_empty()
        );
    }

    #[test]
    fn detects_invalid_carton_weight() {

        let input = WeightValidationInput {
            net_weight_g: None,
            gross_weight_g: None,

            piece_weight_g: None,
            case_weight_g: Some(600.0),
            carton_weight_g: Some(5000.0),

            pieces_per_case: None,
            cases_per_carton: Some(12),
        };

        let findings = validate(&input);

        assert!(
            !findings.is_empty()
        );
    }
}