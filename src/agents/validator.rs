use crate::{
    ai::{
        client::{AiClient, MockAiClient},
        prompt_loader,
    },
    models::{product::Product, result::ModuleResult},
};

pub async fn run(product: &Product) -> ModuleResult {
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
        Ok(response) => ModuleResult {
            status: "PASS".into(),
            findings: vec![response],
            provider: "mock".into(),
        },

        Err(error) => ModuleResult {
            status: "FAIL".into(),
            findings: vec![error.to_string()],
            provider: "mock".into(),
        },
    }
}
