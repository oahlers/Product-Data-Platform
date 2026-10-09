use crate::models::{product::Product, result::ModuleResult};
use crate::ai::prompt_loader;

pub async fn run(product: &Product) -> ModuleResult {
    let prompt =
    prompt_loader::load(
        "text_qa.txt"
    )
    .unwrap_or_default();

println!(
    "Text QA prompt loaded: {} chars",
    prompt.len()
);
    let text = format!("{} {}", product.description, product.marketing_text).to_lowercase();
    let risky = ["cures", "heals", "treats", "eliminates wrinkles", "100% sustainable", "zero environmental impact"];
    let findings: Vec<String> = risky.iter().filter(|x| text.contains(**x)).map(|x| format!("Flagged phrase: {x}")).collect();
    ModuleResult { status: if findings.is_empty() { "PASS" } else { "FAIL" }.into(), findings }
}
