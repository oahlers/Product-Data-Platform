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
            "product_validator.txt"
        )
        .unwrap_or_default();

    let input = format!(
        "Description: {}\n\
         Net Weight: {:?}\n\
         Gross Weight: {:?}\n\
         Width: {:?}\n\
         Depth: {:?}\n\
         Height: {:?}",
        product.description,
        product.net_weight_g,
        product.gross_weight_g,
        product.width_mm,
        product.depth_mm,
        product.height_mm
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