use anyhow::Result;
use async_trait::async_trait;

use crate::models::product::Product;

#[async_trait]
pub trait ProductSource: Send + Sync {
    async fn products(&self) -> Result<Vec<Product>>;

    async fn modified_products(&self, _modified_since: &str) -> Result<Vec<Product>> {
        self.products().await
    }

    async fn product(&self, sku: &str) -> Result<Option<Product>>;

    async fn product_by_ean(&self, ean: &str) -> Result<Option<Product>>;

    async fn product_by_product_id(&self, _product_id: &str) -> Result<Option<Product>> {
        Ok(None)
    }

    async fn search_products(&self, search_term: &str) -> Result<Vec<Product>>;
}
