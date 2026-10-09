use std::{path::{Path, PathBuf}, sync::Arc};
use anyhow::{Context, Result};
use tokio::{fs, sync::Mutex};
use crate::models::{product::Product, result::SkuResult};

#[derive(Clone)]
pub struct JsonStore {
    products_path: PathBuf,
    results_dir: PathBuf,
    write_lock: Arc<Mutex<()>>,
}

impl JsonStore {
    pub fn new(products_path: impl Into<PathBuf>, results_dir: impl Into<PathBuf>) -> Self {
        Self { products_path: products_path.into(), results_dir: results_dir.into(), write_lock: Arc::new(Mutex::new(())) }
    }

    pub async fn ensure_files(&self) -> Result<()> {
        if let Some(parent) = self.products_path.parent() { fs::create_dir_all(parent).await?; }
        fs::create_dir_all(&self.results_dir).await?;
        if !Path::new(&self.products_path).exists() { fs::write(&self.products_path, "[]").await?; }
        Ok(())
    }

    pub async fn products(&self) -> Result<Vec<Product>> {
        let bytes = fs::read(&self.products_path).await.context("read products.json")?;
        Ok(serde_json::from_slice(&bytes).context("parse products.json")?)
    }

    pub async fn product(&self, sku: &str) -> Result<Option<Product>> {
        Ok(self.products().await?.into_iter().find(|p| p.sku.eq_ignore_ascii_case(sku)))
    }

    pub async fn save_result(&self, result: &SkuResult) -> Result<()> {
        let _guard = self.write_lock.lock().await;
        let path = self.results_dir.join(format!("{}.json", result.sku));
        fs::write(path, serde_json::to_vec_pretty(result)?).await?;
        Ok(())
    }

    pub async fn result(&self, sku: &str) -> Result<Option<SkuResult>> {
        let path = self.results_dir.join(format!("{}.json", sku));
        match fs::read(path).await {
            Ok(bytes) => Ok(Some(serde_json::from_slice(&bytes)?)),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(e) => Err(e.into()),
        }
    }
}
