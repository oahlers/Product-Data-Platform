#[derive(Clone)]
pub struct AiConfig {
    pub provider: String,

    pub endpoint: Option<String>,
    pub deployment: Option<String>,
    pub api_key: Option<String>,
}