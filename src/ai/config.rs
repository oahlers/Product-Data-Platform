#[derive(Clone)]
pub struct AiConfig {
    pub provider: String,

    pub endpoint: Option<String>,
    pub deployment: Option<String>,
    pub api_key: Option<String>,
}

impl AiConfig {
    pub fn from_env() -> Self {
        Self {
            provider: std::env::var("AI_PROVIDER").unwrap_or_else(|_| "mock".into()),

            endpoint: std::env::var("AZURE_OPENAI_ENDPOINT").ok(),

            deployment: std::env::var("AZURE_OPENAI_DEPLOYMENT").ok(),

            api_key: std::env::var("AZURE_OPENAI_API_KEY").ok(),
        }
    }
}
