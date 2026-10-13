use crate::models::result::ModuleResult;

pub fn calculate(validation: &ModuleResult, text_qa: &ModuleResult, inci: &ModuleResult) -> String {
    if validation.status == "FAIL" {
        return "BLOCKED".into();
    }

    if validation.status == "WARNING"
        || text_qa.status == "FAIL"
        || text_qa.status == "WARNING"
        || inci.status == "FAIL"
        || inci.status == "HIGH"
        || inci.status == "WARNING"
    {
        return "REVIEW".into();
    }

    "READY".into()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn module_result(status: &str) -> ModuleResult {
        ModuleResult {
            status: status.into(),
            findings: Vec::new(),
            provider: "test".into(),
        }
    }

    #[test]
    fn blocks_validation_failure() {
        let status = calculate(
            &module_result("FAIL"),
            &module_result("PASS"),
            &module_result("PASS"),
        );

        assert_eq!(status, "BLOCKED");
    }

    #[test]
    fn reviews_validation_warning() {
        let status = calculate(
            &module_result("WARNING"),
            &module_result("PASS"),
            &module_result("PASS"),
        );

        assert_eq!(status, "REVIEW");
    }

    #[test]
    fn reviews_text_qa_failure() {
        let status = calculate(
            &module_result("PASS"),
            &module_result("FAIL"),
            &module_result("PASS"),
        );

        assert_eq!(status, "REVIEW");
    }

    #[test]
    fn reviews_inci_failure() {
        let status = calculate(
            &module_result("PASS"),
            &module_result("PASS"),
            &module_result("FAIL"),
        );

        assert_eq!(status, "REVIEW");
    }

    #[test]
    fn returns_ready_when_all_modules_pass() {
        let status = calculate(
            &module_result("PASS"),
            &module_result("PASS"),
            &module_result("PASS"),
        );

        assert_eq!(status, "READY");
    }
}
