use crate::models::{product::Product, result::ModuleResult};
use crate::ai::prompt_loader;

pub async fn run(_product: &Product) -> ModuleResult {
    let prompt =
    prompt_loader::load(
        "translation.txt"
    )
    .unwrap_or_default();

println!(
    "Translation prompt loaded: {} chars",
    prompt.len()
);
    ModuleResult { status: "PASS".into(), findings: vec!["MVP placeholder: connect Azure OpenAI translation prompt".into()] }
}
