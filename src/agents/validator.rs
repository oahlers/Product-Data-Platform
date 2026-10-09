use crate::models::{product::Product, result::ModuleResult};
use crate::ai::prompt_loader;

pub async fn run(product: &Product) -> ModuleResult {
    let prompt =
    prompt_loader::load(
        "product_validator.txt"
    )
    .unwrap_or_default();

println!(
    "Validator prompt loaded: {} chars",
    prompt.len()
);
    let mut findings = Vec::new();
    if let (Some(net), Some(gross)) = (product.net_weight_g, product.gross_weight_g) {
        if net > gross { findings.push("Net weight exceeds gross weight".into()); }
    }
    for (name, value) in [("width", product.width_mm), ("depth", product.depth_mm), ("height", product.height_mm)] {
        if value.is_some_and(|v| v <= 0.0) { findings.push(format!("Invalid {name}")); }
    }
    ModuleResult { status: if findings.is_empty() { "PASS" } else { "FAIL" }.into(), findings }
}
