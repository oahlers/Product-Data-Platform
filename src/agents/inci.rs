use crate::models::{product::Product, result::ModuleResult};
use crate::ai::prompt_loader;

pub async fn run(product: &Product) -> ModuleResult {

    let prompt =
        prompt_loader::load(
            "inci_review.txt"
        )
        .unwrap_or_default();
