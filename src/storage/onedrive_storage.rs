use anyhow::Result;
use async_trait::async_trait;

use crate::{
    models::product::Product,
    storage::product_source::ProductSource,
};

pub struct OneDriveStore;

#[async_trait]
impl ProductSource for OneDriveStore {
    async fn products(
        &self,
    ) -> Result<Vec<Product>> {

        anyhow::bail!(
            "OneDriveStore not implemented yet"
        )
    }
}