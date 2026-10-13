use anyhow::{ensure, Context, Result};

#[derive(Debug, Clone)]
pub struct InriverConfig {
    pub api_url: String,
    pub token: String,
    pub auth_scheme: String,
    pub max_concurrent_imports: usize,
}

impl InriverConfig {
    pub fn from_env() -> Result<Self> {
        let max_concurrent_imports = match std::env::var("INRIVER_IMPORT_CONCURRENCY") {
            Ok(value) => value
                .parse::<usize>()
                .context("INRIVER_IMPORT_CONCURRENCY must be a positive integer")?,
            Err(std::env::VarError::NotPresent) => 3,
            Err(error) => return Err(error.into()),
        };
        ensure!(
            max_concurrent_imports > 0,
            "INRIVER_IMPORT_CONCURRENCY must be greater than zero"
        );

        Ok(Self {
            api_url: std::env::var("INRIVER_API_URL").context("INRIVER_API_URL missing")?,
            token: std::env::var("INRIVER_TOKEN").context("INRIVER_TOKEN missing")?,
            auth_scheme: std::env::var("INRIVER_AUTH_SCHEME")
                .unwrap_or_else(|_| "Bearer".to_string()),
            max_concurrent_imports,
        })
    }
}
