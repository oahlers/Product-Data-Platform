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
            "translation.txt"
        )
        .unwrap_or_default();

    let input = format!(
        "Product: {}\n\nMarketing Text:\n{}",
        product.description,
        product.marketing_text
    );

    let client = MockAiClient;

    match client
        .execute(
            &prompt,
            &input,
        )
        .await
    {
        Ok(response) => ModuleResult {
            status: "PASS".into(),
            findings: vec![response],
        },

        Err(error) => ModuleResult {
            status: "FAIL".into(),
            findings: vec![
                error.to_string()
            ],
        },
    }
}