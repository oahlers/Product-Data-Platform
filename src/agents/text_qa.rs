use crate::models::{product::Product, result::ModuleResult};

pub async fn run(product: &Product) -> ModuleResult {
    let text = format!("{} {}", product.description, product.marketing_text).to_lowercase();
    let risky = ["cures", "heals", "treats", "eliminates wrinkles", "100% sustainable", "zero environmental impact"];
    let findings: Vec<String> = risky.iter().filter(|x| text.contains(**x)).map(|x| format!("Flagged phrase: {x}")).collect();
    ModuleResult { status: if findings.is_empty() { "PASS" } else { "FAIL" }.into(), findings }
}
