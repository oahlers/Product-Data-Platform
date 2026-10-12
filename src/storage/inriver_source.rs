use anyhow::{bail, Context, Result};
use reqwest::{
    header::{HeaderMap, HeaderValue, ACCEPT, AUTHORIZATION},
    Client,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::inriver_config::InriverConfig;

#[derive(Debug, Clone)]
pub struct InriverSource {
    client: Client,
    config: InriverConfig,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct QueryRequest {
    system_criteria: Vec<SystemCriterion>,

    #[serde(skip_serializing_if = "Option::is_none")]
    objects: Option<String>,
}

#[derive(Debug, Serialize)]
struct SystemCriterion {
    #[serde(rename = "type")]
    criterion_type: String,

    value: String,
    operator: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct QueryResponse {
    #[serde(default)]
    pub count: usize,

    #[serde(default)]
    pub entity_ids: Vec<i64>,
}

impl InriverSource {
    pub fn new(config: InriverConfig) -> Result<Self> {
        let client = Client::builder()
            .default_headers(build_headers(&config)?)
            .build()
            .context("Could not create Inriver HTTP client")?;

        Ok(Self { client, config })
    }

    pub async fn query_entity_ids(
        &self,
        entity_type: &str,
        modified_since: Option<&str>,
    ) -> Result<QueryResponse> {
        let mut criteria = vec![SystemCriterion {
            criterion_type: "EntityTypeId".to_string(),
            value: entity_type.to_string(),
            operator: "Equal".to_string(),
        }];

        if let Some(value) = modified_since {
            criteria.push(SystemCriterion {
                criterion_type: "LastModified".to_string(),
                value: value.to_string(),
                operator: "GreaterThanOrEqual".to_string(),
            });
        }

        let request = QueryRequest {
            system_criteria: criteria,
            objects: None,
        };

        let url = format!("{}/api/v1.0.0/query", self.config.api_url);

        let response = self
            .client
            .post(&url)
            .json(&request)
            .send()
            .await
            .context("Inriver query request failed")?;

        let status = response.status();
        let body = response
            .text()
            .await
            .context("Could not read Inriver query response")?;

        if !status.is_success() {
            bail!(
                "Inriver query failed with HTTP {}: {}",
                status,
                body
            );
        }

        serde_json::from_str(&body).with_context(|| {
            format!("Could not parse Inriver query response: {body}")
        })
    }

    pub async fn get_entity_summary(
        &self,
        entity_id: i64,
    ) -> Result<Value> {
        let url = format!(
            "{}/api/v1.0.0/entities/{}/summary",
            self.config.api_url,
            entity_id
        );

        self.get_json(&url).await
    }

    pub async fn get_entity_fields(
        &self,
        entity_id: i64,
    ) -> Result<Value> {
        let url = format!(
            "{}/api/v1.0.0/entities/{}/summary/fields",
            self.config.api_url,
            entity_id
        );

        self.get_json(&url).await
    }

    pub async fn get_entity_bundle(
        &self,
        entity_id: i64,
    ) -> Result<Value> {
        let url = format!(
            "{}/api/v1.0.0/entities/{}/linksandfields",
            self.config.api_url,
            entity_id
        );

        self.get_json(&url).await
    }

    async fn get_json(&self, url: &str) -> Result<Value> {
        let response = self
            .client
            .get(url)
            .send()
            .await
            .with_context(|| format!("Inriver GET request failed: {url}"))?;

        let status = response.status();
        let body = response
            .text()
            .await
            .context("Could not read Inriver response")?;

        if !status.is_success() {
            bail!(
                "Inriver request failed with HTTP {}: {}",
                status,
                body
            );
        }

        serde_json::from_str(&body).with_context(|| {
            format!("Could not parse Inriver JSON response: {body}")
        })
    }
}

fn build_headers(config: &InriverConfig) -> Result<HeaderMap> {
    let mut headers = HeaderMap::new();

    headers.insert(
        ACCEPT,
        HeaderValue::from_static("application/json"),
    );

    if config.auth_scheme.trim().is_empty() {
        return Ok(headers);
    }

    let authorization = format!(
        "{} {}",
        config.auth_scheme.trim(),
        config.token.trim()
    );

    headers.insert(
        AUTHORIZATION,
        HeaderValue::from_str(&authorization)
            .context("Invalid Inriver authorization header")?,
    );

    Ok(headers)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn creates_quer*_payload() {
        let request =*QueryRequest {
            system_*riteria: vec![SystemCriterion {
                criterion_type: "EntityTypeId".to_string(),
            *   value: "Product".to_string(),
 *              operator: "Equal".to*string(),
            }],
        *   objects: None,
        };

    *   let json = serde_json::to_value*request)
            .expect("Quer* request should serialize");

    *   assert_eq!(
            json["systemCriteria"][0]["type"],
       *    "EntityTypeId"
        );

   *    assert_eq!(
            json["systemCriteria"][0]["value"],
     *      "Product"
        );
    }
}*

#[cfg(test)]
mod integration_*ests {
    use super::*;

    #[to*io::test]
    #[ignore = "requires*live Inriver credentials"]
    asy*c fn queries_products_and_fetches_*irst_bundle() {
        dotenvy::d*tenv().ok();

        let config =*InriverConfig::from_env()
        *   .expect("Inriver environment va*iables must exist");

        let *ource = InriverSource::new(config)*            .expect("Inriver clien* should be created");

        let*query = source
            .query_*ntity_ids("Product", None)
       *    .await
            .expect("Pr*duct query should succeed");

    *   println!("Inriver product count* {}", query.count);

        let f*rst_id = query
            .entity*ids
            .first()
         *  .copied()
            .expect("Q*ery should return at least one ent*ty ID");

        println!("First *ntity ID: {first_id}");

        l*t bundle = source
            .get*entity_bundle(first_id)
          * .await
            .expect("Bundl* request should succeed");

      * println!(
            "{}",
     *      serde_json::to_string_pretty*&bundle)
                .expect("*undle should serialize")
        )*
    }
}