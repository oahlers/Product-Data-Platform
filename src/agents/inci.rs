use crate::models::{product::Product, result::ModuleResult};

pub async fn run(product: &Product) -> ModuleResult {
    if product.inci.trim().is_empty() {
        return ModuleResult { status: "HIGH".into(), findings: vec!["Missing INCI list".into()] };
    }
    let upper = product.inci.to_uppercase();
    let watch = ["PHENOXYETHANOL", "PARFUM", "LINALOOL", "LIMONENE", "CITRAL"];
    let findings: Vec<String> = watch.iter().filter(|x| upper.contains(**x)).map(|x| format!("Ingredient of interest: {x}")).collect();
    let status = match findings.len() { 0 => "LOW", 1..=2 => "MEDIUM", _ => "HIGH" };
    ModuleResult { status: status.into(), findings }
}
