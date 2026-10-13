use crate::{
    ai::{
        client::{AiClient, MockAiClient},
        prompt_loader,
    },
    models::{product::Product, result::ModuleResult, unified_product::UnifiedProduct},
    rules::{engine, result::Severity},
};

pub async fn run(product: &Product) -> ModuleResult {
    let mut findings = Vec::new();

    //
    // Deterministic rule engine
    //
    let rule_findings = engine::validate_product(product);

    let has_errors = rule_findings
        .iter()
        .any(|finding| matches!(finding.severity, Severity::Error));

    let has_warnings = rule_findings
        .iter()
        .any(|finding| matches!(finding.severity, Severity::Warning));

    for finding in rule_findings {
        let severity = match finding.severity {
            Severity::Error => "ERROR",
            Severity::Warning => "WARNING",
        };

        findings.push(format!(
            "[{}][{}] {}",
            severity, finding.rule_id, finding.message
        ));
    }

    //
    // Existing AI validation
    //
    let prompt = prompt_loader::load("product_validator.txt").unwrap_or_default();

    let input = format!(
        "
SKU: {}

Brand: {}

Product Name: {}

Description:
{}

Marketing Text:
{}

Warnings:
{}

Directions:
{}

INCI:
{}
",
        product.sku,
        product.brand,
        product.product_name,
        product.description,
        product.marketing_text,
        product.warnings,
        product.directions,
        product.inci
    );

    let client = MockAiClient;

    match client.execute(&prompt, &input).await {
        Ok(response) => {
            if !response.trim().is_empty() {
                findings.push(response);
            }

            let status = if has_errors {
                "FAIL"
            } else if has_warnings {
                "WARNING"
            } else {
                "PASS"
            };

            ModuleResult {
                status: status.into(),
                findings,
                provider: "rules+mock".into(),
            }
        }

        Err(error) => {
            findings.push(format!("AI validation failed: {}", error));

            ModuleResult {
                status: "FAIL".into(),
                findings,
                provider: "rules+mock".into(),
            }
        }
    }
}

pub fn validate_unified(product: &UnifiedProduct) -> ModuleResult {
    let rule_findings = crate::rules::unified::validate(product);

    let has_errors = rule_findings
        .iter()
        .any(|finding| matches!(finding.severity, Severity::Error));

    let has_warnings = rule_findings
        .iter()
        .any(|finding| matches!(finding.severity, Severity::Warning));

    let findings = rule_findings
        .into_iter()
        .map(|finding| {
            let severity = match finding.severity {
                Severity::Error => "ERROR",
                Severity::Warning => "WARNING",
            };

            format!("[{}][{}] {}", severity, finding.rule_id, finding.message)
        })
        .collect();

    let status = if has_errors {
        "FAIL"
    } else if has_warnings {
        "WARNING"
    } else {
        "PASS"
    };

    ModuleResult {
        status: status.into(),
        findings,
        provider: "unified-rules".into(),
    }
}
