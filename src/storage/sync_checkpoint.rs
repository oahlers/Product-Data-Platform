use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use chrono::{DateTime, Duration, Utc};
use tokio::fs;

#[derive(Debug, Clone)]
pub struct SyncCheckpointStore {
    path: PathBuf,
}

impl SyncCheckpointStore {
    pub fn new(path: impl AsRef<Path>) -> Self {
        Self {
            path: path.as_ref().to_path_buf(),
        }
    }

    pub async fn load(&self) -> Result<DateTime<Utc>> {
        match fs::read_to_string(&self.path).await {
            Ok(value) => {
                let timestamp = DateTime::parse_from_rfc3339(value.trim())
                    .context("Invalid Inriver checkpoint timestamp")?
                    .with_timezone(&Utc);

                Ok(timestamp)
            }

            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                Ok(Utc::now() - Duration::hours(24))
            }

            Err(error) => Err(error).context("Failed to read Inriver checkpoint"),
        }
    }

    pub async fn save(&self, timestamp: DateTime<Utc>) -> Result<()> {
        if let Some(parent) = self.path.parent() {
            fs::create_dir_all(parent)
                .await
                .context("Failed to create checkpoint directory")?;
        }

        let temporary_path = self.path.with_extension("tmp");

        fs::write(&temporary_path, timestamp.to_rfc3339())
            .await
            .context("Failed to write temporary checkpoint")?;

        fs::rename(&temporary_path, &self.path)
            .await
            .context("Failed to replace Inriver checkpoint")?;

        Ok(())
    }
}
