use anyhow::{Context, Result};

#[derive(Debug, Clone)]
pub struct InriverConfig {
    pub api_url: String,
    pub token: String,
    pub auth_scheme: String,
}

impl InriverConfig {
    pub fn from_env() -> Result<Self> {
        Ok(Self {
            api_url: std::env::var("INRIVER_API_URL").context("INRIVER_API_URL missing")?,
            token: std::env::var("INRIVER_TOKEN").context("INRIVER_TOKEN missing")?,
            auth_scheme: std::env::var("INRIVER_AUTH_SCHEME")
                .unwrap_or_else(|_| "Bearer".to_string()),
        })
    }
}
