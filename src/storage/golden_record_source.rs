use anyhow::{Context, Result};
use azure_core::credentials::Secret;
use azure_data_cosmos::{
    feed::FeedScope, options::Region, AccountEndpoint, AccountReference, CosmosClient,
    RoutingStrategy,
};
use futures::TryStreamExt;
use serde_json::Value;
use std::collections::HashMap;

#[derive(Clone)]
pub struct GoldenRecordSource {
    client: CosmosClient,
    database_name: String,
    container_name: String,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct GoldenRecordProduct {
    pub item_number: String,
    pub product_id: Option<String>,
    pub ean: Option<String>,
    pub product_name: Option<String>,

    pub m3_description: Option<String>,
    pub m3_status: Option<String>,
    pub m3_item_type: Option<String>,
    pub m3_item_group: Option<String>,
    pub m3_item_class: Option<String>,
    pub m3_unit: Option<String>,
    pub m3_country: Option<String>,

    pub dimension_1: Option<f64>,
    pub dimension_2: Option<f64>,
    pub dimension_3: Option<f64>,

    pub gross_weight: Option<f64>,
    pub net_weight: Option<f64>,
    pub volume: Option<f64>,
}

impl GoldenRecordSource {
    pub async fn from_env() -> Result<Self> {
        let endpoint = std::env::var("COSMOS_ENDPOINT").context("COSMOS_ENDPOINT is missing")?;

        let key = std::env::var("COSMOS_KEY").context("COSMOS_KEY is missing")?;

        let endpoint: AccountEndpoint = endpoint.parse().context("Invalid COSMOS_ENDPOINT")?;

        let account = AccountReference::with_authentication_key(endpoint, Secret::from(key));

        let client = CosmosClient::builder()
            .build(account, RoutingStrategy::ProximityTo(Region::WEST_EUROPE))
            .await
            .context("Failed to create Cosmos client")?;

        Ok(Self {
            client,
            database_name: "GoldenRecord".to_string(),
            container_name: "Record".to_string(),
        })
    }

    pub async fn get_by_item_number(
        &self,
        item_number: &str,
    ) -> Result<Option<GoldenRecordProduct>> {
        let database = self.client.database_client(&self.database_name);

        let container = database
            .container_client(&self.container_name, None)
            .await
            .context("Failed to create Golden Record container client")?;

        let escaped = item_number.replace('\'', "''");

        let query = format!("SELECT * FROM c WHERE c.id = '{}'", escaped);

        let mut items = container
            .query_items::<Value>(&query, FeedScope::full_container(), None)
            .await
            .context("Failed to query Golden Record")?;

        let Some(record) = items.try_next().await? else {
            return Ok(None);
        };

        Ok(Some(parse_golden_record(&record)?))
    }

    pub async fn get_by_product_id(&self, product_id: &str) -> Result<Option<GoldenRecordProduct>> {
        let database = self.client.database_client(&self.database_name);

        let container = database
            .container_client(&self.container_name, None)
            .await
            .context("Failed to create Golden Record container client")?;

        let query = r#"
            SELECT *
            FROM c
            WHERE IS_DEFINED(c.PimData.Data)
              AND NOT IS_NULL(c.PimData.Data)
        "#;

        let mut items = container
            .query_items::<Value>(query, FeedScope::full_container(), None)
            .await
            .context("Failed to query Golden Record by ProductID")?;

        while let Some(record) = items.try_next().await? {
            let parsed = parse_golden_record(&record)?;

            if parsed.product_id.as_deref() == Some(product_id) {
                return Ok(Some(parsed));
            }
        }

        Ok(None)
    }
    pub async fn all_by_product_id(&self) -> Result<HashMap<String, GoldenRecordProduct>> {
        let database = self.client.database_client(&self.database_name);

        let container = database
            .container_client(&self.container_name, None)
            .await
            .context("Failed to create Golden Record container client")?;

        let query = r#"
            SELECT *
            FROM c
            WHERE IS_DEFINED(c.PimData.Data)
              AND NOT IS_NULL(c.PimData.Data)
        "#;

        let mut items = container
            .query_items::<Value>(query, FeedScope::full_container(), None)
            .await
            .context("Failed to query Golden Records")?;

        let mut records: HashMap<String, GoldenRecordProduct> = HashMap::new();

        while let Some(record) = items.try_next().await? {
            let parsed = parse_golden_record(&record)?;

            let Some(product_id) = parsed.product_id.clone() else {
                continue;
            };

            records.insert(product_id, parsed);
        }

        Ok(records)
    }
}

fn parse_golden_record(record: &Value) -> Result<GoldenRecordProduct> {
    let record_id = value_string(record.get("id"));

    let erp = record.pointer("/ErpData/Data");

    let pim = record.pointer("/PimData/Data/inRiverJson");

    let m3_item_number = value_string_at(erp, "Mmitno");

    let item_number = m3_item_number
        .or(record_id)
        .context("Golden Record has no item number")?;

    let product_id = find_inriver_field(pim, "ProductID");

    let ean = find_inriver_field(pim, "ItemEAN");

    let product_name = find_inriver_field(pim, "ProductName");

    Ok(GoldenRecordProduct {
        item_number,

        product_id,
        ean,
        product_name,

        m3_description: value_string_at(erp, "Mmitds"),

        m3_status: value_string_at(erp, "Mmstat"),

        m3_item_type: value_string_at(erp, "Mmitty"),

        m3_item_group: value_string_at(erp, "Mmitgr"),

        m3_item_class: value_string_at(erp, "Mmitcl"),

        m3_unit: value_string_at(erp, "Mmpuun"),

        m3_country: value_string_at(erp, "M9orco"),

        dimension_1: parse_f64_at(erp, "Mmdim1"),

        dimension_2: parse_f64_at(erp, "Mmdim2"),

        dimension_3: parse_f64_at(erp, "Mmdim3"),

        gross_weight: parse_f64_at(erp, "Mmgrwe"),

        net_weight: parse_f64_at(erp, "Mmnewe"),

        volume: parse_f64_at(erp, "Mmvol3"),
    })
}

fn value_string(value: Option<&Value>) -> Option<String> {
    let value = value?.as_str()?.trim();

    if value.is_empty() {
        None
    } else {
        Some(value.to_string())
    }
}

fn value_string_at(parent: Option<&Value>, field: &str) -> Option<String> {
    value_string(parent?.get(field))
}

fn parse_f64_at(parent: Option<&Value>, field: &str) -> Option<f64> {
    let value = value_string_at(parent, field)?;

    value.replace(',', ".").parse::<f64>().ok()
}

fn find_inriver_field(inriver_json: Option<&Value>, field_id: &str) -> Option<String> {
    let object = inriver_json?.as_object()?;

    for fields in object.values() {
        let Some(fields) = fields.as_array() else {
            continue;
        };

        for field in fields {
            let id = field.get("@ID").and_then(Value::as_str);

            if id != Some(field_id) {
                continue;
            }

            let value = field.get("@VALUE").and_then(Value::as_str)?.trim();

            if !value.is_empty() {
                return Some(value.to_string());
            }
        }
    }

    None
}

#[cfg(test)]
mod integration_tests {
    use super::*;

    #[tokio::test]
    #[ignore = "requires live Cosmos DB credentials"]
    async fn finds_golden_record_by_item_number() {
        dotenvy::dotenv().expect("Failed to load .env");

        let source = GoldenRecordSource::from_env()
            .await
            .expect("Failed to create GoldenRecordSource");

        let product = source
            .get_by_item_number("1014070112")
            .await
            .expect("Golden Record query failed")
            .expect("Product 1014070112 was not found");

        println!("{product:#?}");

        assert_eq!(product.item_number, "1014070112");

        assert_eq!(product.product_id.as_deref(), Some("1010197NS1014070112"));

        assert_eq!(
            product.product_name.as_deref(),
            Some("ØKONOMI ØKONOMI Matas generisk display")
        );

        assert_eq!(
            product.m3_description.as_deref(),
            Some("MATAS GENERISK DISPLAY")
        );
    }

    #[tokio::test]
    #[ignore = "requires live Cosmos DB credentials"]
    async fn finds_golden_record_by_product_id() {
        dotenvy::dotenv().expect("Failed to load .env");

        let source = GoldenRecordSource::from_env()
            .await
            .expect("Failed to create GoldenRecordSource");

        let product = source
            .get_by_product_id("1010197NS1014070112")
            .await
            .expect("Golden Record ProductID query failed")
            .expect("ProductID 1010197NS1014070112 was not found");

        println!("{product:#?}");

        assert_eq!(product.item_number, "1014070112");

        assert_eq!(product.product_id.as_deref(), Some("1010197NS1014070112"));
    }
}
