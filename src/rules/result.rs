#[derive(Debug, Clone)]
pub enum Severity {
    Warning,
    Error,
}

#[derive(Debug, Clone)]
pub struct RuleFinding {
    pub rule_id: String,
    pub severity: Severity,
    pub message: String,
}
