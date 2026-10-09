use crate::{
    ai::{
        client::{AiClient, MockAiClient},
        prompt_loader,
    },
    models::{
        product::Product,
        result::ModuleResult,
    },
};

pub async fn run(product: &Product) -> ModuleResult {

    let prompt =
        prompt_loader::load(
            "inci_review.txt"
        )
        .unwrap_or_default();

    let input = format!(
        "INCI:\n{}",
        product.inci
    );

    let client = MockAiClient;

    match client.execute(
        &prompt,
        &input,
    ).await {

        Ok(response) => ModuleResult {
            status: "PASS".into(),
            findings: vec![response],
        },

        Err(error) => ModuleResult {
            status: "FAIL".into(),
            findings: vec![error.to_string()],
        },
    }
}