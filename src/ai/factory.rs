use std::sync::Arc;

use crate::ai::{
    azure_client::AzureOpenAiClient,
    client::{AiClient, MockAiClient},
    config::AiConfig,
};

pub fn create_client(config: &AiConfig) -> Arc<dyn AiClient> {
    match config.provider.as_str() {
        "azure" => Arc::new(AzureOpenAiClient::new(
            config.endpoint.clone().unwrap(),
            config.api_key.clone().unwrap(),
            config.deployment.clone().unwrap(),
        )),

        _ => Arc::new(MockAiClient),
    }
}
