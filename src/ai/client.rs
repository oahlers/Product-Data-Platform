#[async_trait::async_trait]
pub trait AiClient {

    async fn execute(
        &self,
        prompt: &str,
        input: &str,
    ) -> anyhow::Result<String>;
}