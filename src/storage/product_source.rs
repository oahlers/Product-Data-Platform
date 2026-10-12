use anyhow::Result;
use async_trait::async_trait;

use crate::models::product::Product;

#[async_trait]
pub trait ProductSource: Send + Sync {
    async fn products(&self) -> Result<Vec<Product>>;

    async fn product(&self, sku: &str) -> Result<Option<Product>>;

    async fn product_by_ean(&self, ean: &str) -> Result<Option<Product>>;

    async fn search_products(&self, search_term: &str) -> Result<Vec<Product>>;
}
