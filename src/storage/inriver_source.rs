use anyhow::{bail, Context, Result};
use chrono::{DateTime, NaiveDateTime, Utc};
use reqwest::{
    header::{HeaderMap, HeaderName, HeaderValue, ACCEPT, RETRY_AFTER},
    Client,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::time::Duration;

use crate::models::product::Product;

use super::inriver_config::InriverConfig;

const INRIVER_API_KEY_HEADER: &str = "x-inriver-apikey";

#[derive(Debug, Clone)]
pub struct InriverSource {
    client: Client,
    config: InriverConfig,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct QueryRequest {
    system_criteria: Vec<SystemCriterion>,

    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    data_criteria: Vec<DataCriterion>,

    #[serde(skip_serializing_if = "Option::is_none")]
    objects: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct DataCriterion {
    field_type_id: String,
    operator: String,
    value: String,
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
            .context("Failed to create Inriver HTTP client")?;

        Ok(Self { client, config })
    }

    pub async fn query_products(&self) -> Result<QueryResponse> {
        self.query_entities("Product", None).await
    }

    pub async fn query_modified_products(&self, modified_since: &str) -> Result<QueryResponse> {
        self.query_entities("Product", Some(modified_since)).await
    }

    pub async fn query_entities(
        &self,
        entity_type: &str,
        modified_since: Option<&str>,
    ) -> Result<QueryResponse> {
        let mut criteria = vec![SystemCriterion {
            criterion_type: "EntityTypeId".to_string(),
            value: entity_type.to_string(),
            operator: "Equal".to_string(),
        }];

        if let Some(modified_since) = modified_since {
            criteria.push(SystemCriterion {
                criterion_type: "LastModified".to_string(),
                value: modified_since.to_string(),
                operator: "GreaterThanOrEqual".to_string(),
            });
        }

        let request = QueryRequest {
            system_criteria: criteria,
            data_criteria: Vec::new(),
            objects: None,
        };

        self.post_query(&request).await
    }

    pub async fn query_products_by_field(
        &self,
        field_type_id: &str,
        operator: &str,
        value: &str,
    ) -> Result<QueryResponse> {
        let request = QueryRequest {
            system_criteria: vec![SystemCriterion {
                criterion_type: "EntityTypeId".to_string(),
                value: "Product".to_string(),
                operator: "Equal".to_string(),
            }],

            data_criteria: vec![DataCriterion {
                field_type_id: field_type_id.to_string(),
                operator: operator.to_string(),
                value: value.to_string(),
            }],

            objects: None,
        };

        self.post_query(&request).await
    }

    pub async fn get_entity_bundle(&self, entity_id: i64) -> Result<Value> {
        let url = format!(
            "{}/api/v1.0.0/entities/{}/linksandfields",
            self.config.api_url.trim_end_matches('/'),
            entity_id
        );

        self.get_json(&url).await
    }

    pub async fn get_product(&self, entity_id: i64) -> Result<Product> {
        let bundle = self.get_entity_bundle(entity_id).await?;

        map_inriver_entity(&bundle)
    }

    pub async fn get_product_by_product_id(&self, product_id: &str) -> Result<Option<Product>> {
        let result = self
            .query_products_by_field("ProductID", "Equal", product_id)
            .await?;

        let Some(entity_id) = result.entity_ids.first().copied() else {
            return Ok(None);
        };

        let product = self.get_product(entity_id).await?;

        Ok(Some(product))
    }

    pub async fn get_product_by_sku(&self, sku: &str) -> Result<Option<Product>> {
        let result = self.query_products().await?;

        for entity_id in result.entity_ids {
            let bundle = self.get_entity_bundle(entity_id).await?;

            let product = map_inriver_entity(&bundle)?;

            if product.sku == sku {
                return Ok(Some(product));
            }
        }

        Ok(None)
    }

    pub async fn get_product_by_ean(&self, ean: &str) -> Result<Option<Product>> {
        let result = self.query_products().await?;

        for entity_id in result.entity_ids {
            let bundle = self.get_entity_bundle(entity_id).await?;

            let matching_item = find_linked_item_by_ean(&bundle, ean);

            if matching_item.is_none() {
                continue;
            }

            let product = map_inriver_entity_with_item_ean(&bundle, Some(ean))?;

            return Ok(Some(product));
        }

        Ok(None)
    }

    pub async fn search_products_by_term(&self, search_term: &str) -> Result<Vec<Product>> {
        use futures::{stream, StreamExt, TryStreamExt};

        let result = self
            .query_products_by_field("ProductName", "Contains", search_term)
            .await?;

        let concurrency = self.config.max_concurrent_imports;

        let source = self.clone();

        let products = stream::iter(result.entity_ids)
            .map(move |entity_id| {
                let source = source.clone();

                async move {
                    let bundle = source.get_entity_bundle(entity_id).await?;

                    let product = map_inriver_entity(&bundle)?;

                    Ok::<Product, anyhow::Error>(product)
                }
            })
            .buffer_unordered(concurrency)
            .try_collect::<Vec<Product>>()
            .await?;

        Ok(products)
    }

    async fn get_json(&self, url: &str) -> Result<Value> {
        const MAX_ATTEMPTS: u32 = 3;

        for attempt in 1..=MAX_ATTEMPTS {
            let response = self
                .client
                .get(url)
                .send()
                .await
                .with_context(|| format!("Inriver GET request failed: {url}"))?;

            let status = response.status();

            // Skal aflæses før response.text(), fordi text()
            // forbruger response-objektet.
            let delay_seconds = retry_delay_seconds(response.headers(), attempt);

            let body = response
                .text()
                .await
                .context("Failed to read Inriver response")?;

            if status.is_success() {
                return serde_json::from_str(&body)
                    .with_context(|| format!("Failed to parse Inriver JSON response: {body}"));
            }

            if status.as_u16() == 429 && attempt < MAX_ATTEMPTS {
                tracing::warn!(
                    attempt = attempt,
                    max_attempts = MAX_ATTEMPTS,
                    delay_seconds = delay_seconds,
                    url = url,
                    "Inriver throttled GET request - retrying"
                );

                tokio::time::sleep(Duration::from_secs(delay_seconds)).await;

                continue;
            }

            bail!("Inriver request failed with HTTP {}: {}", status, body);
        }

        unreachable!("Inriver GET retry loop exited unexpectedly")
    }

    async fn post_query(&self, request: &QueryRequest) -> Result<QueryResponse> {
        const MAX_ATTEMPTS: u32 = 3;

        let url = format!(
            "{}/api/v1.0.0/query",
            self.config.api_url.trim_end_matches('/')
        );

        for attempt in 1..=MAX_ATTEMPTS {
            let response = self
                .client
                .post(&url)
                .json(request)
                .send()
                .await
                .context("Failed to query Inriver")?;

            let status = response.status();

            // Skal aflæses før response.text().
            let delay_seconds = retry_delay_seconds(response.headers(), attempt);

            let body = response
                .text()
                .await
                .context("Failed to read Inriver query response")?;

            if status.is_success() {
                return serde_json::from_str(&body)
                    .with_context(|| format!("Failed to parse Inriver query response: {body}"));
            }

            if status.as_u16() == 429 && attempt < MAX_ATTEMPTS {
                tracing::warn!(
                    attempt = attempt,
                    max_attempts = MAX_ATTEMPTS,
                    delay_seconds = delay_seconds,
                    url = %url,
                    "Inriver throttled query request - retrying"
                );

                tokio::time::sleep(Duration::from_secs(delay_seconds)).await;

                continue;
            }

            bail!("Inriver query failed with HTTP {}: {}", status, body);
        }

        unreachable!("Inriver query retry loop exited unexpectedly")
    }
}

fn retry_delay_seconds(headers: &HeaderMap, attempt: u32) -> u64 {
    let fallback_seconds = 2_u64.pow(attempt - 1);

    let Some(value) = headers.get(RETRY_AFTER) else {
        return fallback_seconds;
    };

    let Ok(value) = value.to_str() else {
        return fallback_seconds;
    };

    // Retry-After som antal sekunder.
    if let Ok(seconds) = value.parse::<u64>() {
        return seconds;
    }

    // Retry-After som HTTP-dato.
    if let Ok(retry_at) = chrono::DateTime::parse_from_rfc2822(value) {
        let now = Utc::now();

        let seconds = retry_at
            .with_timezone(&Utc)
            .signed_duration_since(now)
            .num_seconds();

        return seconds.max(0) as u64;
    }

    fallback_seconds
}

pub fn map_inriver_entity(json: &Value) -> Result<Product> {
    map_inriver_entity_with_item_ean(json, None)
}

fn map_inriver_entity_with_item_ean(json: &Value, requested_ean: Option<&str>) -> Result<Product> {
    let summary = json
        .get("summary")
        .context("Inriver bundle is missing root summary")?;

    let linked_item = match requested_ean {
        Some(ean) => find_linked_item_by_ean(json, ean),
        None => find_linked_entity(json, "Item"),
    };

    if requested_ean.is_some() && linked_item.is_none() {
        bail!(
            "No linked Item matched EAN {}",
            requested_ean.unwrap_or_default()
        );
    }

    let linked_item_summary = linked_item.and_then(|entity| entity.get("summary"));

    let mut fields = Vec::new();

    // Product-felter fra root-entity
    if let Some(root_fields) = json.get("fields") {
        collect_fields(root_fields, &mut fields);
    }

    // Item-felter kun fra det valgte linked Item.
    // Vi samler ikke længere felter fra samtlige linked Items.
    if let Some(item) = linked_item {
        if let Some(item_fields) = item.get("fields") {
            let mut selected_item_fields = Vec::new();

            collect_fields(item_fields, &mut selected_item_fields);

            // Item-felter kommer først, så de vinder over
            // eventuelle felter med samme ID på Product.
            selected_item_fields.extend(fields);
            fields = selected_item_fields;
        }
    }

    let sku = summary_string(summary, "displayDescription");

    if sku.is_empty() {
        bail!("Inriver entity is missing SKU/displayDescription");
    }

    let linked_item_name = linked_item_summary
        .and_then(|item_summary| non_empty_string(item_summary, "displayName"))
        .unwrap_or_default();

    let product_name = first_non_empty(&[
        field_string(&fields, "ItemWebname"),
        linked_item_name,
        summary_string(summary, "displayName"),
    ]);

    let description = first_non_empty(&[
        locale_field(&fields, "ItemBodyTx", "en"),
        locale_field(&fields, "ItemTeaserTx", "en"),
    ]);

    let marketing_text = first_non_empty(&[
        locale_field(&fields, "ItemTeaserTx", "en"),
        locale_field(&fields, "ItemBodyTx", "en"),
    ]);

    let directions = first_non_empty(&[
        locale_field(&fields, "ItemHowToTx", "en"),
        field_string(&fields, "ItemHowToTx"),
    ]);

    let warnings = first_non_empty(&[
        locale_field(&fields, "ItemWarningsTx", "en"),
        locale_field(&fields, "ItemWarningTx", "en"),
        locale_field(&fields, "ItemPrecautionsTx", "en"),
        field_string(&fields, "ItemWarnings"),
        field_string(&fields, "ItemWarning"),
    ]);

    let inci = field_string(&fields, "ItemInci");

    let brand = first_non_empty(&[
        field_string(&fields, "ItemBrand"),
        field_string(&fields, "ProductBrand"),
        field_string(&fields, "Brand"),
    ]);

    let width_mm = parse_cm_as_mm_field(&fields, "ItemWidth");

    let height_mm = parse_cm_as_mm_field(&fields, "ItemHeight");

    let depth_mm = parse_cm_as_mm_field(&fields, "ItemLength");

    let primary_volume_mm3 = calculate_volume(width_mm, depth_mm, height_mm);

    let paper_cardboard_g = parse_f64_field(&fields, "ItemPaper");

    let glass_g = parse_f64_field(&fields, "ItemPackagingGlass");

    let plastic_g = parse_f64_field(&fields, "ItemPackagingPlastic");

    let alcohol_percent = parse_f64_field(&fields, "ItemAlcoholPercent");

    let gender = optional_field_string(&fields, "ItemGender");

    let total_packaging_g = parse_f64_field(&fields, "ItemTotalPackaging");

    let net_weight_g = parse_f64_field(&fields, "ItemMLGR");

    let dangerous_goods = optional_field_string(&fields, "ItemDangerGoods");

    let danger_class = optional_field_string(&fields, "ItemDangerClass");

    let un_number = optional_field_string(&fields, "ItemUNNumber");

    let packing_group = optional_field_string(&fields, "ItemPackingGroup");

    let flash_point = parse_f64_field(&fields, "ItemFlashPoint");

    let cpnp_number = optional_field_string(&fields, "CPNPNumber");

    let shelf_life = optional_field_string(&fields, "ItemShelfLife");

    let period_after_opening = optional_field_string(&fields, "ItemFirstOpening");

    let expiration_date = optional_field_string(&fields, "ItemExpirationDate");

    let top_notes = locale_field(&fields, "ItemTopNotes", "en");

    let heart_notes = locale_field(&fields, "ItemHeartNotes", "en");

    let base_notes = locale_field(&fields, "ItemBaseNotes", "en");

    let aluminium_g = parse_f64_field(&fields, "ItemPackagingAluminium");

    let tinplate_g = parse_f64_field(&fields, "ItemPackagingMetal");

    let upc_code = {
        let ean = field_string(&fields, "ItemEAN");

        if !ean.is_empty() {
            Some(ean)
        } else {
            linked_item_summary
                .and_then(|item_summary| non_empty_string(item_summary, "displayDescription"))
        }
    };

    let modified_at = parse_modified_at(summary).unwrap_or_else(Utc::now);

    Ok(Product {
        sku,
        upc_code,
        brand,
        product_name,
        description,
        marketing_text,
        inci,
        warnings,
        directions,

        width_mm,
        depth_mm,
        height_mm,
        primary_volume_mm3,

        secondary_width_mm: None,
        secondary_depth_mm: None,
        secondary_height_mm: None,
        secondary_volume_mm3: None,

        net_weight_g,
        gross_weight_g: None,
        piece_weight_g: None,

        items_per_prepack: None,
        prepacks_per_shipper: None,
        items_per_shipper: None,

        shipper_width_mm: None,
        shipper_depth_mm: None,
        shipper_height_mm: None,
        shipper_weight_g: None,
        shipper_volume_mm3: None,

        case_weight_g: None,
        carton_weight_g: None,
        pieces_per_case: None,
        cases_per_carton: None,

        shippers_per_pallet: None,
        items_per_pallet: None,
        pallet_height_mm: None,
        pallet_weight_g: None,
        pallet_gtin: None,

        cube: None,
        interspace_percent: None,

        packaging_material_layers: None,

        paper_cardboard_g,
        glass_g,
        textiles_wood_g: None,
        tinplate_g,
        aluminium_g,
        plastic_g,

        alcohol_percent,
        gender,
        total_packaging_g,

        dangerous_goods,
        danger_class,
        un_number,
        packing_group,
        flash_point,

        top_notes,
        heart_notes,
        base_notes,

        cpnp_number,

        shelf_life,

        period_after_opening,

        expiration_date,

        modified_at,
    })
}

pub fn dump_fields(value: &Value) {
    match value {
        Value::Object(object) => {
            if object.contains_key("fieldTypeId") {
                let field_type_id = object
                    .get("fieldTypeId")
                    .and_then(Value::as_str)
                    .unwrap_or("");

                let display_name = object
                    .get("fieldTypeDisplayName")
                    .and_then(Value::as_str)
                    .unwrap_or("");

                let field_data_type = object
                    .get("fieldDataType")
                    .and_then(Value::as_str)
                    .unwrap_or("");

                let value = object.get("value").unwrap_or(&Value::Null);

                println!(
                    "{} | {} | {} | {}",
                    field_type_id,
                    display_name,
                    field_data_type,
                    compact_json(value)
                );
            }

            for child in object.values() {
                dump_fields(child);
            }
        }

        Value::Array(values) => {
            for child in values {
                dump_fields(child);
            }
        }

        _ => {}
    }
}

pub fn field_dump_lines(value: &Value) -> Vec<String> {
    let mut lines = Vec::new();

    collect_field_dump_lines(value, &mut lines);

    lines.sort();
    lines.dedup();

    lines
}

fn collect_field_dump_lines(value: &Value, output: &mut Vec<String>) {
    match value {
        Value::Object(object) => {
            if object.contains_key("fieldTypeId") {
                let field_type_id = object
                    .get("fieldTypeId")
                    .and_then(Value::as_str)
                    .unwrap_or("");

                let display_name = object
                    .get("fieldTypeDisplayName")
                    .and_then(Value::as_str)
                    .unwrap_or("");

                let field_data_type = object
                    .get("fieldDataType")
                    .and_then(Value::as_str)
                    .unwrap_or("");

                let field_value = object.get("value").unwrap_or(&Value::Null);

                output.push(format!(
                    "{} | {} | {} | {}",
                    field_type_id,
                    display_name,
                    field_data_type,
                    compact_json(field_value)
                ));
            }

            for child in object.values() {
                collect_field_dump_lines(child, output);
            }
        }

        Value::Array(values) => {
            for child in values {
                collect_field_dump_lines(child, output);
            }
        }

        _ => {}
    }
}

fn compact_json(value: &Value) -> String {
    serde_json::to_string(value).unwrap_or_else(|_| "null".to_string())
}

fn build_headers(config: &InriverConfig) -> Result<HeaderMap> {
    let mut headers = HeaderMap::new();

    headers.insert(ACCEPT, HeaderValue::from_static("application/json"));

    let api_key_header = HeaderName::from_static(INRIVER_API_KEY_HEADER);

    let api_key = HeaderValue::from_str(config.token.trim())
        .context("Invalid Inriver API key header value")?;

    headers.insert(api_key_header, api_key);

    Ok(headers)
}

fn collect_fields<'a>(value: &'a Value, output: &mut Vec<&'a Value>) {
    match value {
        Value::Object(object) => {
            if object.contains_key("fieldTypeId") && object.contains_key("value") {
                output.push(value);
            }

            for child in object.values() {
                collect_fields(child, output);
            }
        }

        Value::Array(values) => {
            for child in values {
                collect_fields(child, output);
            }
        }

        _ => {}
    }
}

fn find_linked_entity<'a>(value: &'a Value, entity_type_id: &str) -> Option<&'a Value> {
    match value {
        Value::Object(object) => {
            if let Some(summary) = object.get("summary") {
                let current_type = summary.get("entityTypeId").and_then(Value::as_str);

                if current_type == Some(entity_type_id) {
                    return Some(value);
                }
            }

            for child in object.values() {
                if let Some(entity) = find_linked_entity(child, entity_type_id) {
                    return Some(entity);
                }
            }

            None
        }

        Value::Array(values) => {
            for child in values {
                if let Some(entity) = find_linked_entity(child, entity_type_id) {
                    return Some(entity);
                }
            }

            None
        }

        _ => None,
    }
}

fn find_linked_item_by_ean<'a>(value: &'a Value, requested_ean: &str) -> Option<&'a Value> {
    match value {
        Value::Object(object) => {
            let is_item = object
                .get("summary")
                .and_then(|summary| summary.get("entityTypeId"))
                .and_then(Value::as_str)
                == Some("Item");

            if is_item {
                let mut fields = Vec::new();

                if let Some(item_fields) = object.get("fields") {
                    collect_fields(item_fields, &mut fields);
                }

                let item_ean = field_string(&fields, "ItemEAN");

                let summary_ean = object
                    .get("summary")
                    .and_then(|summary| non_empty_string(summary, "displayDescription"));

                if item_ean == requested_ean || summary_ean.as_deref() == Some(requested_ean) {
                    return Some(value);
                }
            }

            for child in object.values() {
                if let Some(item) = find_linked_item_by_ean(child, requested_ean) {
                    return Some(item);
                }
            }

            None
        }

        Value::Array(values) => {
            for child in values {
                if let Some(item) = find_linked_item_by_ean(child, requested_ean) {
                    return Some(item);
                }
            }

            None
        }

        _ => None,
    }
}

fn field<'a>(fields: &[&'a Value], field_type_id: &str) -> Option<&'a Value> {
    fields
        .iter()
        .copied()
        .find(|field| field.get("fieldTypeId").and_then(Value::as_str) == Some(field_type_id))
}

fn field_string(fields: &[&Value], field_type_id: &str) -> String {
    let Some(field) = field(fields, field_type_id) else {
        return String::new();
    };

    if let Some(value) = field.get("value").and_then(Value::as_str) {
        if !value.trim().is_empty() {
            return value.trim().to_string();
        }
    }

    field
        .get("displayValue")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or_default()
        .to_string()
}

fn optional_field_string(fields: &[&Value], field_type_id: &str) -> Option<String> {
    let value = field_string(fields, field_type_id);

    if value.trim().is_empty() {
        None
    } else {
        Some(value)
    }
}

fn locale_field(fields: &[&Value], field_type_id: &str, preferred_language: &str) -> String {
    let Some(field) = field(fields, field_type_id) else {
        return String::new();
    };

    let Some(value) = field.get("value") else {
        return String::new();
    };

    if let Some(text) = value.as_str() {
        return text.trim().to_string();
    }

    let Some(locales) = value.as_object() else {
        return String::new();
    };

    if let Some(text) = locales
        .get(preferred_language)
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        return text.to_string();
    }

    if let Some(text) = locales
        .get("en")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        return text.to_string();
    }

    locales
        .values()
        .filter_map(Value::as_str)
        .map(str::trim)
        .find(|value| !value.is_empty())
        .unwrap_or_default()
        .to_string()
}

fn summary_string(summary: &Value, property: &str) -> String {
    summary
        .get(property)
        .and_then(Value::as_str)
        .map(str::trim)
        .unwrap_or_default()
        .to_string()
}

fn non_empty_string(value: &Value, property: &str) -> Option<String> {
    value
        .get(property)
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToString::to_string)
}

fn first_non_empty(values: &[String]) -> String {
    values
        .iter()
        .find(|value| !value.trim().is_empty())
        .cloned()
        .unwrap_or_default()
}

fn parse_f64_field(fields: &[&Value], field_type_id: &str) -> Option<f64> {
    let field = field(fields, field_type_id)?;

    let value = field.get("value")?;

    if let Some(number) = value.as_f64() {
        return Some(number);
    }

    if let Some(text) = value.as_str() {
        let text = text.trim();

        if text.is_empty() {
            return None;
        }

        return text.replace(',', ".").parse::<f64>().ok();
    }

    None
}

fn parse_cm_as_mm_field(fields: &[&Value], field_type_id: &str) -> Option<f64> {
    parse_f64_field(fields, field_type_id).map(|v| v * 10.0)
}

fn calculate_volume(
    width_mm: Option<f64>,
    depth_mm: Option<f64>,
    height_mm: Option<f64>,
) -> Option<f64> {
    match (width_mm, depth_mm, height_mm) {
        (Some(w), Some(d), Some(h)) => Some(w * d * h),
        _ => None,
    }
}

fn parse_modified_at(summary: &Value) -> Option<DateTime<Utc>> {
    let value = summary.get("modifiedDate").and_then(Value::as_str)?;

    if let Ok(date) = DateTime::parse_from_rfc3339(value) {
        return Some(date.with_timezone(&Utc));
    }

    let naive = NaiveDateTime::parse_from_str(value, "%Y-%m-%dT%H:%M:%S%.f").ok()?;

    Some(DateTime::<Utc>::from_naive_utc_and_offset(naive, Utc))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn creates_query_request() {
        let request = QueryRequest {
            system_criteria: vec![SystemCriterion {
                criterion_type: "EntityTypeId".to_string(),
                value: "Product".to_string(),
                operator: "Equal".to_string(),
            }],

            data_criteria: Vec::new(),

            objects: None,
        };

        let json = serde_json::to_value(request).expect("Query request should serialize");

        assert_eq!(json["systemCriteria"][0]["type"], "EntityTypeId");

        assert_eq!(json["systemCriteria"][0]["value"], "Product");
    }

    #[test]
    fn maps_inriver_bundle_to_product() {
        let bundle = serde_json::json!({
                    "summary": {
                        "displayDescription": "10700103",
                        "displayName": "AZZARO Chrome Eau de toilette tester",
                        "entityTypeId": "Product",
                        "modifiedDate": "2026-08-21T08:00:00.0000000Z"
                    },
                    "linkedEntities": [
            {
                "summary": {
                    "displayDescription": "3614273962384",
                    "displayName": "Chrome United eau de toilette 100 ml",
                    "entityTypeId": "Item"
                },
                "fields": [
                    {
                        "fieldTypeId": "ItemEAN",
                        "fieldTypeDisplayName": "EAN",
                        "fieldDataType": "String",
                        "value": "3614273962384"
                    },
                    {
                        "fieldTypeId": "ItemWebname",
                        "fieldTypeDisplayName": "Webname",
                        "fieldDataType": "String",
                        "value": "Chrome United eau de toilette 100 ml"
                    },
                    {
                        "fieldTypeId": "ItemInci",
                        "fieldTypeDisplayName": "INCI",
                        "fieldDataType": "String",
                        "value": "ALCOHOL, PARFUM, AQUA"
                    },
                    {
                        "fieldTypeId": "ItemWidth",
                        "fieldTypeDisplayName": "Width",
                        "fieldDataType": "String",
                        "value": "9.4"
                    },
                    {
                        "fieldTypeId": "ItemLength",
                        "fieldTypeDisplayName": "Length",
                        "fieldDataType": "String",
                        "value": "3.9"
                    },
                    {
                        "fieldTypeId": "ItemHeight",
                        "fieldTypeDisplayName": "Height",
                        "fieldDataType": "String",
                        "value": "12.1"
                    },
                    {
                        "fieldTypeId": "ItemPaper",
                        "fieldTypeDisplayName": "Paper packaging",
                        "fieldDataType": "String",
                        "value": "21.13"
                    },
                    {
                        "fieldTypeId": "ItemPackagingGlass",
                        "fieldTypeDisplayName": "Glass packaging",
                        "fieldDataType": "String",
                        "value": "181.1"
                    },
                    {
                        "fieldTypeId": "ItemPackagingPlastic",
                        "fieldTypeDisplayName": "Plastic packaging",
                        "fieldDataType": "String",
                        "value": "17.99"
                    },
                    {
                        "fieldTypeId": "ItemPackagingMetal",
                        "fieldTypeDisplayName": "Metal packaging",
                        "fieldDataType": "String",
                        "value": "2.21"
                    },
                    {
                        "fieldTypeId": "ItemBodyTx",
                        "fieldTypeDisplayName": "Body",
                        "fieldDataType": "LocaleString",
                        "value": {
                            "da": "",
                            "en": "English product description",
                            "fi": "",
                            "no": "",
                            "sv": ""
                        }
                    },
                    {
                        "fieldTypeId": "ItemHowToTx",
                        "fieldTypeDisplayName": "How to",
                        "fieldDataType": "LocaleString",
                        "value": {
                            "da": "",
                            "en": "Apply as directed",
                            "fi": "",
                            "no": "",
                            "sv": ""
                        }
                    }
                ]
            }
        ]
                });

        let product = map_inriver_entity(&bundle).expect("Bundle should map to Product");

        assert_eq!(product.sku, "10700103");

        assert_eq!(product.upc_code.as_deref(), Some("3614273962384"));

        assert_eq!(product.product_name, "Chrome United eau de toilette 100 ml");

        assert_eq!(product.description, "English product description");

        assert_eq!(product.inci, "ALCOHOL, PARFUM, AQUA");

        assert_eq!(product.directions, "Apply as directed");

        assert_eq!(product.width_mm, Some(94.0));
        assert_eq!(product.depth_mm, Some(39.0));
        assert_eq!(product.height_mm, Some(121.0));

        assert_eq!(product.paper_cardboard_g, Some(21.13));

        assert_eq!(product.glass_g, Some(181.1));

        assert_eq!(product.plastic_g, Some(17.99));

        assert_eq!(product.tinplate_g, Some(2.21));
    }

    #[test]
    fn maps_requested_linked_item_by_ean() {
        let bundle = serde_json::json!({
            "summary": {
                "displayDescription": "PRODUCT-123",
                "displayName": "Test Product",
                "entityTypeId": "Product",
                "modifiedDate": "2026-08-21T08:00:00Z"
            },
            "linkedEntities": [
                {
                    "summary": {
                        "displayDescription": "1111111111111",
                        "displayName": "Item A",
                        "entityTypeId": "Item"
                    },
                    "fields": [
                        {
                            "fieldTypeId": "ItemEAN",
                            "value": "1111111111111"
                        },
                        {
                            "fieldTypeId": "ItemWebname",
                            "value": "Item A"
                        },
                        {
                            "fieldTypeId": "ItemWidth",
                            "value": "1"
                        },
                        {
                            "fieldTypeId": "ItemLength",
                            "value": "2"
                        },
                        {
                            "fieldTypeId": "ItemHeight",
                            "value": "3"
                        }
                    ]
                },
                {
                    "summary": {
                        "displayDescription": "2222222222222",
                        "displayName": "Item B",
                        "entityTypeId": "Item"
                    },
                    "fields": [
                        {
                            "fieldTypeId": "ItemEAN",
                            "value": "2222222222222"
                        },
                        {
                            "fieldTypeId": "ItemWebname",
                            "value": "Item B"
                        },
                        {
                            "fieldTypeId": "ItemWidth",
                            "value": "4"
                        },
                        {
                            "fieldTypeId": "ItemLength",
                            "value": "5"
                        },
                        {
                            "fieldTypeId": "ItemHeight",
                            "value": "6"
                        }
                    ]
                }
            ]
        });

        let product = map_inriver_entity_with_item_ean(&bundle, Some("2222222222222"))
            .expect("Requested Item should map");

        assert_eq!(product.upc_code.as_deref(), Some("2222222222222"));

        assert_eq!(product.product_name, "Item B");

        assert_eq!(product.width_mm, Some(40.0));

        assert_eq!(product.depth_mm, Some(50.0));

        assert_eq!(product.height_mm, Some(60.0));
    }

    #[test]
    fn creates_field_dump_lines() {
        let bundle = serde_json::json!({
            "fields": [
                {
                    "fieldTypeId": "ItemInci",
                    "fieldTypeDisplayName": "INCI",
                    "fieldDataType": "String",
                    "value": "ALCOHOL, AQUA"
                }
            ]
        });

        let lines = field_dump_lines(&bundle);

        assert_eq!(lines.len(), 1);

        assert!(lines[0].contains("ItemInci"));

        assert!(lines[0].contains("ALCOHOL"));
    }
}

#[cfg(test)]
mod integration_tests {
    use std::fs;

    use super::*;

    #[tokio::test]
    #[ignore = "requires live Inriver credentials"]
    async fn queries_products_and_fetches_first_bundle() {
        dotenvy::dotenv().ok();

        let config = InriverConfig::from_env().expect("Missing Inriver config");

        let source = InriverSource::new(config).expect("Could not create Inriver source");

        let result = source.query_products().await.expect("Query failed");

        println!("Found {} products", result.count);

        let first_id = result
            .entity_ids
            .first()
            .copied()
            .expect("No entity IDs returned");

        println!("Using entity ID {first_id}");

        let bundle = source
            .get_entity_bundle(first_id)
            .await
            .expect("Bundle request failed");

        let field_lines = field_dump_lines(&bundle);

        let field_dump = if field_lines.is_empty() {
            String::new()
        } else {
            format!("{}\n", field_lines.join("\n"))
        };

        fs::write("inriver_field_dump.txt", field_dump).expect("Field dump should be written");

        println!(
            "Wrote {} field definitions to inriver_field_dump.txt",
            field_lines.len()
        );

        dump_fields(&bundle);

        let product = map_inriver_entity(&bundle).expect("Bundle should map to Product");

        println!(
            "Mapped product:\n{}",
            serde_json::to_string_pretty(&product,).expect("Product should serialize",)
        );
    }

    #[tokio::test]
    #[ignore = "requires live Inriver credentials"]
    async fn finds_product_by_sku() {
        dotenvy::dotenv().ok();

        let config = InriverConfig::from_env().expect("Missing config");

        let source = InriverSource::new(config).expect("Source");

        let product = source
            .get_product_by_sku("10700103")
            .await
            .expect("Lookup failed");

        assert!(product.is_some());

        println!("{}", serde_json::to_string_pretty(&product).unwrap());
    }

    #[tokio::test]
    #[ignore = "requires live Inriver credentials"]
    async fn finds_product_by_ean() {
        dotenvy::dotenv().ok();

        let config = InriverConfig::from_env().expect("Missing config");

        let source = InriverSource::new(config).expect("Source");

        let product = source
            .get_product_by_ean("3351500020409")
            .await
            .expect("Lookup failed");

        assert!(product.is_some());

        println!("{}", serde_json::to_string_pretty(&product).unwrap());
    }

    #[tokio::test]
    #[ignore = "requires live Inriver credentials"]
    async fn searches_products() {
        dotenvy::dotenv().ok();

        let config = InriverConfig::from_env().expect("Missing config");

        let source = InriverSource::new(config).expect("Source");

        let products = source
            .search_products_by_term("chrome")
            .await
            .expect("Search failed");

        println!("Found {} products", products.len());

        assert!(!products.is_empty());

        assert!(products.iter().all(|product| {
            product.product_name.to_lowercase().contains("chrome")
                || product.brand.to_lowercase().contains("chrome")
                || product.sku.to_lowercase().contains("chrome")
        }));

        println!("{}", serde_json::to_string_pretty(&products).unwrap());
    }

    #[tokio::test]
    #[ignore = "requires live Inriver credentials"]
    async fn queries_products_by_name() {
        dotenvy::dotenv().ok();

        let config = InriverConfig::from_env().expect("Missing Inriver config");

        let source = InriverSource::new(config).expect("Could not create Inriver source");

        let result = source
            .query_products_by_field("ProductName", "Contains", "Chrome")
            .await
            .expect("Product name query failed");

        println!("Found {} matching product entities", result.count);

        println!("Entity IDs: {:?}", result.entity_ids);

        assert!(
            !result.entity_ids.is_empty(),
            "No Chrome products were returned"
        );
    }

    #[test]
    fn parses_inriver_modified_date_without_timezone() {
        let summary = serde_json::json!({
            "modifiedDate": "2026-08-21T08:00:00.0000000"
        });

        let modified_at = parse_modified_at(&summary).expect("Modified date should parse");

        assert_eq!(modified_at.to_rfc3339(), "2026-08-21T08:00:00+00:00");
    }
}

use async_trait::async_trait;

use crate::storage::product_source::ProductSource;

#[async_trait]
impl ProductSource for InriverSource {
    async fn products(&self) -> Result<Vec<Product>> {
        use futures::{stream, StreamExt, TryStreamExt};

        let started = std::time::Instant::now();

        let result = self.query_products().await?;

        let entity_count = result.entity_ids.len();

        let concurrency = self.config.max_concurrent_imports;

        let source = self.clone();

        tracing::info!(
            entity_count = entity_count,
            concurrency = concurrency,
            "Loading all Inriver products"
        );

        let products = stream::iter(result.entity_ids)
            .map(move |entity_id| {
                let source = source.clone();

                async move {
                    let bundle = source.get_entity_bundle(entity_id).await?;

                    let product = map_inriver_entity(&bundle)?;

                    Ok::<Product, anyhow::Error>(product)
                }
            })
            .buffer_unordered(concurrency)
            .try_collect::<Vec<Product>>()
            .await?;

        tracing::info!(
            entity_count = entity_count,
            loaded_count = products.len(),
            concurrency = concurrency,
            elapsed_ms = started.elapsed().as_millis(),
            "Loaded all Inriver products"
        );

        Ok(products)
    }

    async fn modified_products(&self, modified_since: &str) -> Result<Vec<Product>> {
        use futures::{stream, StreamExt, TryStreamExt};

        let started = std::time::Instant::now();

        let result = self.query_modified_products(modified_since).await?;

        let entity_count = result.entity_ids.len();

        let concurrency = self.config.max_concurrent_imports;

        let source = self.clone();

        tracing::info!(
            modified_since = modified_since,
            entity_count = entity_count,
            concurrency = concurrency,
            "Loading modified Inriver products"
        );

        let products = stream::iter(result.entity_ids)
            .map(move |entity_id| {
                let source = source.clone();

                async move {
                    let bundle = source.get_entity_bundle(entity_id).await?;

                    let product = map_inriver_entity(&bundle)?;

                    Ok::<Product, anyhow::Error>(product)
                }
            })
            .buffer_unordered(concurrency)
            .try_collect::<Vec<Product>>()
            .await?;

        tracing::info!(
            modified_since = modified_since,
            entity_count = entity_count,
            loaded_count = products.len(),
            concurrency = concurrency,
            elapsed_ms = started.elapsed().as_millis(),
            "Loaded modified Inriver products"
        );

        Ok(products)
    }

    async fn product(&self, sku: &str) -> Result<Option<Product>> {
        self.get_product_by_sku(sku).await
    }

    async fn product_by_ean(&self, ean: &str) -> Result<Option<Product>> {
        self.get_product_by_ean(ean).await
    }

    async fn product_by_product_id(&self, product_id: &str) -> Result<Option<Product>> {
        self.get_product_by_product_id(product_id).await
    }

    async fn search_products(&self, search_term: &str) -> Result<Vec<Product>> {
        self.search_products_by_term(search_term).await
    }
}
