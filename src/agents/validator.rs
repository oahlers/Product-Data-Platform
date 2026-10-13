use crate::{
    ai::{
        client::{AiClient, MockAiClient},
        prompt_loader,
    },
    models::{product::Product, result::ModuleResult},
    rules::engine,
};

pub async fn run(product: &Product) -> ModuleResult {
    let mut findings = Vec::new();

    //
    // Deterministic rule engine
    //
    let rule_findings = engine::validate_product(product);

    for finding in rule_findings {
        findings.push(format!("[{}] {}", finding.rule_id, finding.message));
    }

    //
    // Existing AI validation
    //
    let prompt = prompt_loader::load("validator.txt").unwrap_or_default();

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
            findings.push(response);

            ModuleResult {
                status: if findings.is_empty() {
                    "PASS".into()
                } else {
                    "WARNING".into()
                },
                findings,
                provider: "rules+mock".into(),
            }
        }

        Err(error) => {
            findings.push(error.to_string());

            ModuleResult {
                status: "FAIL".into(),
                findings,
                provider: "rules+mock".into(),
            }
        }
    }
}
