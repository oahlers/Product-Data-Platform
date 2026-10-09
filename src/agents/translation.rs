use crate::models::{product::Product, result::ModuleResult};

pub async fn run(_product: &Product) -> ModuleResult {
    ModuleResult { status: "PASS".into(), findings: vec!["MVP placeholder: connect Azure OpenAI translation prompt".into()] }
}
