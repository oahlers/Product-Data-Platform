use crate::models::result::ModuleResult;
pub fn calculate(validation: &ModuleResult, text_qa: &ModuleResult, inci: &ModuleResult) -> String {
    if validation.status == "FAIL" { "BLOCKED".into() }
    else if text_qa.status == "FAIL" || inci.status == "HIGH" { "REVIEW".into() }
    else { "READY".into() }
}
