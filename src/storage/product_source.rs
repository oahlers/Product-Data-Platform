use anyhow::Result;
use async_trait::async_trait;

use crate::models::product::Product;

#[async_trait]
pub trait ProductSource: Send + Sync {
    async fn products(
        &self,
    ) -> Result<Vec<Product>>;
}