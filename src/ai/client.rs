pub struct MockAiClient;

#[async_trait::async_trait]
pub trait AiClient {
    async fn execute(
        &self,
        prompt: &str,
        input: &str,
    ) -> anyhow::Result<String>;
}

#[async_trait::async_trait]
impl AiClient for MockAiClient {
    async fn execute(
        &self,
        prompt: &str,
        input: &str,
    ) -> anyhow::Result<String> {

        Ok(format!(
            "PROMPT:\n{}\n\nINPUT:\n{}",
            prompt,
            input
        ))
    }
}