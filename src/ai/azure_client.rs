use anyhow::Result;

use super::client::AiClient;

pub struct AzureOpenAiClient {
    pub endpoint: String,
    pub api_key: String,
    pub deployment: String,
}

impl AzureOpenAiClient {
    pub fn new(
        endpoint: String,
        api_key: String,
        deployment: String,
    ) -> Self {
        Self {
            endpoint,
            api_key,
            deployment,
        }
    }
}

#[async_trait::async_trait]
impl AiClient for AzureOpenAiClient {
    async fn execute(
        &self,
        _prompt: &str,
        _input: &str,
    ) -> Result<String> {

        Ok(
            "Azure OpenAI not connected yet"
                .to_string()
        )
    }
}