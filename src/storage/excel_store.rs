use std::{
    collections::HashMap,
    path::{Path, PathBuf},
};

use anyhow::{Context, Result};
use async_trait::async_trait;
use calamine::{open_workbook_auto, Data, Reader};
use chrono::Utc;
use tokio::task;
use tracing::{info, warn};

use super::product_source::ProductSource;
use crate::models::product::Product;

#[derive(Clone)]
pub struct ExcelStore {
    directory: PathBuf,
}

impl ExcelStore {
    pub fn new(directory: impl Into<PathBuf>) -> Self {
        Self {
            directory: directory.into(),
        }
    }

    pub async fn load_products(&self) -> Result<Vec<Product>> {
        let directory = self.directory.clone();

        task::spawn_blocking(move || load_directory(&directory))
            .await
            .context("Excel reader task failed")?
    }

    pub async fn product(&self, sku: &str) -> Result<Option<Product>> {
        Ok(self
            .load_products()
            .await?
            .into_iter()
            .find(|product| product.sku.eq_ignore_ascii_case(sku)))
    }
}

fn load_directory(directory: &Path) -> Result<Vec<Product>> {
    if !directory.exists() {
        anyhow::bail!("Excel directory does not exist: {}", directory.display());
    }

    let mut products = Vec::new();

    for entry in std::fs::read_dir(directory)? {
        let path = entry?.path();

        if !is_supported_excel_file(&path) {
            continue;
        }

        match load_workbook(&path) {
            Ok(mut workbook_products) => {
                info!(
                    file = %path.display(),
                    count = workbook_products.len(),
                    "Excel workbook loaded"
                );

                products.append(&mut workbook_products);
            }

            Err(error) => {
                warn!(
                    file = %path.display(),
                    %error,
                    "Excel workbook could not be loaded"
                );
            }
        }
    }

    deduplicate_products(&mut products);

    info!(count = products.len(), "Excel product ingestion completed");

    Ok(products)
}

fn load_workbook(path: &Path) -> Result<Vec<Product>> {
    let mut workbook =
        open_workbook_auto(path).with_context(|| format!("Cannot open {}", path.display()))?;

    let sheet_names = workbook.sheet_names().to_owned();
    let fallback_brand = brand_from_filename(path);
    let mut products = Vec::new();

    for sheet_name in sheet_names {
        let range = match workbook.worksheet_range(&sheet_name) {
            Ok(range) => range,

            Err(error) => {
                warn!(
                    sheet = %sheet_name,
                    %error,
                    "Worksheet could not be read"
                );

                continue;
            }
        };

        let rows: Vec<Vec<String>> = range
            .rows()
            .map(|row| row.iter().map(cell_to_string).collect())
            .collect();

        let Some(header_index) = find_header_row(&rows) else {
            continue;
        };

        let headers = build_header_map(&rows[header_index]);

        for row in rows.iter().skip(header_index + 1) {
            if let Some(product) = map_row(row, &headers, &fallback_brand) {
                products.push(product);
            }
        }
    }

    Ok(products)
}

fn find_header_row(rows: &[Vec<String>]) -> Option<usize> {
    rows.iter().take(100).enumerate().find_map(|(index, row)| {
        let normalized: Vec<String> = row.iter().map(|value| normalize(value)).collect();

        let has_sku = normalized.iter().any(|value| {
            matches!(
                value.as_str(),
                "sku" | "kendo sku" | "item number" | "reference-no" | "reference no"
            )
        });

        let has_product_name = normalized.iter().any(|value| {
            matches!(
                value.as_str(),
                "product name/ short description"
                    | "product name / short description"
                    | "item description"
                    | "description"
                    | "product name"
            )
        });

        (has_sku && has_product_name).then_some(index)
    })
}

fn build_header_map(headers: &[String]) -> HashMap<String, usize> {
    headers
        .iter()
        .enumerate()
        .filter_map(|(index, header)| {
            let normalized = normalize(header);

            (!normalized.is_empty()).then_some((normalized, index))
        })
        .collect()
}

fn map_row(
    row: &[String],
    headers: &HashMap<String, usize>,
    fallback_brand: &str,
) -> Option<Product> {
    let sku = first_value(
        row,
        headers,
        &[
            "sku",
            "kendo sku",
            "item number",
            "reference-no",
            "reference no",
        ],
    );

    if sku.is_empty() {
        return None;
    }

    let upc_code = parse_optional_string(first_value(
        row,
        headers,
        &["upc code", "upc", "ean", "ean code", "gtin", "barcode"],
    ));

    let brand = value_or_fallback(
        first_value(row, headers, &["brand name", "brand"]),
        fallback_brand,
    );

    let product_name = first_value(
        row,
        headers,
        &[
            "product name/ short description",
            "product name / short description",
            "item description",
            "description",
            "product name",
        ],
    );

    let description = normalize_optional_text(first_value(
        row,
        headers,
        &[
            "product name / long description",
            "long description en",
            "trade item description long description-en_gb-sephora",
            "sephora product long description-en_gb",
        ],
    ));

    let marketing_text = normalize_optional_text(first_value(
        row,
        headers,
        &["marketing message", "trade item marketing message-en_gb"],
    ));

    let inci = normalize_optional_text(first_value(
        row,
        headers,
        &["ingredients list", "ingredient statement-en_gb", "inci"],
    ));

    let warnings = normalize_optional_text(first_value(
        row,
        headers,
        &["precaution of use", "precautions", "warnings"],
    ));

    let directions = normalize_optional_text(first_value(
        row,
        headers,
        &[
            "application tips",
            "preparation instructions-en_gb",
            "directions",
            "directions for use",
        ],
    ));

    let net_weight_g = parse_number(first_value(
        row,
        headers,
        &["net weight (g)", "net weight", "net_weight"],
    ));

    let gross_weight_g = parse_number(first_value(
        row,
        headers,
        &[
            "gross weight(g)",
            "gross weight (g)",
            "gross weight",
            "gross_weight",
        ],
    ));

    let width_mm = parse_number(first_value(
        row,
        headers,
        &[
            "primary width (mm)",
            "item width (mm)",
            "width (mm)",
            "width",
        ],
    ));

    let depth_mm = parse_number(first_value(
        row,
        headers,
        &[
            "primary depth (mm)",
            "item depth (mm)",
            "item length (mm)",
            "item lenght (mm)",
            "depth (mm)",
            "depth",
        ],
    ));

    let height_mm = parse_number(first_value(
        row,
        headers,
        &[
            "primary height (mm)",
            "item height (mm)",
            "height (mm)",
            "height",
        ],
    ));

    let primary_volume_mm3 = calculate_volume(width_mm, depth_mm, height_mm);

    let secondary_width_mm = parse_number(first_value(
        row,
        headers,
        &["secondary width (mm)", "s. item width (mm)"],
    ));

    let secondary_depth_mm = parse_number(first_value(
        row,
        headers,
        &[
            "secondary depth (mm)",
            "secondary length (mm)",
            "s. item length (mm)",
        ],
    ));

    let secondary_height_mm = parse_number(first_value(
        row,
        headers,
        &["secondary height (mm)", "s. item height (mm)"],
    ));

    let secondary_volume_mm3 =
        calculate_volume(secondary_width_mm, secondary_depth_mm, secondary_height_mm);

    let piece_weight_g = gross_weight_g;

    let items_per_prepack = parse_u32(first_value(
        row,
        headers,
        &["items per pre-pack", "items per prepack"],
    ));

    let prepacks_per_shipper = parse_u32(first_value(
        row,
        headers,
        &["pre-packs per shipper", "prepacks per shipper"],
    ));

    let items_per_shipper = parse_u32(first_value(row, headers, &["items per shipper"]));

    let shipper_width_mm = parse_number(first_value(
        row,
        headers,
        &["shipper width (mm)", "case width (mm)"],
    ));

    let shipper_depth_mm = parse_number(first_value(
        row,
        headers,
        &[
            "shipper depth (mm)",
            "shipper length (mm)",
            "case depth (mm)",
            "case length (mm)",
        ],
    ));

    let shipper_height_mm = parse_number(first_value(
        row,
        headers,
        &["shipper height (mm)", "case height (mm)"],
    ));

    let shipper_weight_g = parse_number(first_value(
        row,
        headers,
        &["shipper weight (g)", "case weight (g)"],
    ));

    let shipper_volume_mm3 =
        calculate_volume(shipper_width_mm, shipper_depth_mm, shipper_height_mm);

    let case_weight_g = parse_number(first_value(
        row,
        headers,
        &["case weight (g)", "case weight"],
    ));

    let carton_weight_g = parse_number(first_value(
        row,
        headers,
        &["carton weight (g)", "carton weight"],
    ));

    let pieces_per_case = parse_u32(first_value(
        row,
        headers,
        &["pieces per case", "items per case"],
    ));

    let cases_per_carton = parse_u32(first_value(row, headers, &["cases per carton"]));

    let shippers_per_pallet = parse_u32(first_value(
        row,
        headers,
        &["shipper per pallet", "shippers per pallet"],
    ));

    let items_per_pallet = parse_u32(first_value(row, headers, &["items per pallet"]));

    let pallet_height_mm = parse_number(first_value(
        row,
        headers,
        &["pallet height (mm)", "pallet height"],
    ));

    let pallet_weight_g = parse_number(first_value(
        row,
        headers,
        &["pallet weight (g)", "pallet weight"],
    ));

    let pallet_gtin = parse_optional_string(first_value(
        row,
        headers,
        &["pallet gtin", "pallet barcode"],
    ));

    let cube = parse_number(first_value(row, headers, &["cube"]));

    let interspace_percent = parse_number(first_value(
        row,
        headers,
        &["interspace %", "interspace percent"],
    ));

    let packaging_material_layers = parse_u32(first_value(
        row,
        headers,
        &[
            "packaging material layers",
            "packaging material layers *not pallet",
        ],
    ));

    let paper_cardboard_g = parse_number(first_value(
        row,
        headers,
        &[
            "paper/cardboard (grams)",
            "paper / cardboard (grams)",
            "paper cardboard (grams)",
        ],
    ));

    let glass_g = parse_number(first_value(row, headers, &["glass (grams)"]));

    let textiles_wood_g = parse_number(first_value(
        row,
        headers,
        &[
            "textiles/wood (grams)",
            "textiles / wood (grams)",
            "textiles wood (grams)",
        ],
    ));

    let tinplate_g = parse_number(first_value(row, headers, &["tinplate (grams)"]));

    let aluminium_g = parse_number(first_value(
        row,
        headers,
        &["aluminium (grams)", "aluminum (grams)"],
    ));

    let plastic_g = parse_number(first_value(row, headers, &["plastic (grams)"]));

    Some(Product {
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
        secondary_width_mm,
        secondary_depth_mm,
        secondary_height_mm,
        secondary_volume_mm3,
        net_weight_g,
        gross_weight_g,
        piece_weight_g,
        items_per_prepack,
        prepacks_per_shipper,
        items_per_shipper,
        shipper_width_mm,
        shipper_depth_mm,
        shipper_height_mm,
        shipper_weight_g,
        shipper_volume_mm3,
        case_weight_g,
        carton_weight_g,
        pieces_per_case,
        cases_per_carton,
        shippers_per_pallet,
        items_per_pallet,
        pallet_height_mm,
        pallet_weight_g,
        pallet_gtin,
        cube,
        interspace_percent,
        packaging_material_layers,
        paper_cardboard_g,
        glass_g,
        textiles_wood_g,
        tinplate_g,
        aluminium_g,
        plastic_g,
        alcohol_percent: None,
        gender: None,

        total_packaging_g: None,

        cpnp_number: None,

        shelf_life: None,

        period_after_opening: None,

        dangerous_goods: None,
        danger_class: None,
        un_number: None,
        packing_group: None,
        flash_point: None,

        expiration_date: None,

        top_notes: String::new(),
        heart_notes: String::new(),
        base_notes: String::new(),
        modified_at: Utc::now(),
    })
}

fn first_value(row: &[String], headers: &HashMap<String, usize>, aliases: &[&str]) -> String {
    aliases
        .iter()
        .filter_map(|alias| headers.get(*alias))
        .filter_map(|index| row.get(*index))
        .map(|value| value.trim())
        .find(|value| !value.is_empty())
        .unwrap_or_default()
        .to_string()
}

fn normalize(value: &str) -> String {
    value
        .trim()
        .to_lowercase()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

fn normalize_optional_text(value: String) -> String {
    match value.trim().to_lowercase().as_str() {
        "" | "false" | "n/a" | "na" | "-" => String::new(),

        _ => value.trim().to_string(),
    }
}

fn parse_optional_string(value: String) -> Option<String> {
    let value = normalize_optional_text(value);

    if value.is_empty() {
        None
    } else {
        Some(value)
    }
}

fn parse_number(value: String) -> Option<f64> {
    let normalized = value.trim().replace(' ', "").replace(',', ".");

    if normalized.is_empty() {
        return None;
    }

    normalized.parse::<f64>().ok()
}

fn parse_u32(value: String) -> Option<u32> {
    let parsed = parse_number(value)?;

    if !parsed.is_finite() || parsed < 0.0 || parsed.fract() != 0.0 || parsed > u32::MAX as f64 {
        return None;
    }

    Some(parsed as u32)
}

fn calculate_volume(width: Option<f64>, depth: Option<f64>, height: Option<f64>) -> Option<f64> {
    match (width, depth, height) {
        (Some(width), Some(depth), Some(height))
            if width >= 0.0 && depth >= 0.0 && height >= 0.0 =>
        {
            Some(width * depth * height)
        }

        _ => None,
    }
}

fn value_or_fallback(value: String, fallback: &str) -> String {
    if value.trim().is_empty() {
        fallback.to_string()
    } else {
        value
    }
}

fn brand_from_filename(path: &Path) -> String {
    let filename = path
        .file_stem()
        .and_then(|name| name.to_str())
        .unwrap_or_default()
        .to_lowercase();

    if filename.contains("clarins") {
        "Clarins".into()
    } else if filename.contains("ole henriksen") {
        "Ole Henriksen".into()
    } else if filename.contains("fenty beauty") {
        "Fenty Beauty".into()
    } else if filename.contains("fenty skin") {
        "Fenty Skin".into()
    } else if filename.contains("fenty hair") {
        "Fenty Hair".into()
    } else {
        "Unknown".into()
    }
}

fn deduplicate_products(products: &mut Vec<Product>) {
    products.sort_by_key(|p| p.sku.to_lowercase());

    products.dedup_by(|left, right| left.sku.eq_ignore_ascii_case(&right.sku));
}

fn cell_to_string(cell: &Data) -> String {
    match cell {
        Data::Empty => String::new(),

        Data::String(value) => value.trim().to_string(),

        Data::Float(value) => {
            if value.fract() == 0.0 {
                format!("{value:.0}")
            } else {
                value.to_string()
            }
        }

        Data::Int(value) => value.to_string(),

        Data::Bool(value) => value.to_string(),

        Data::Error(error) => format!("{error:?}"),

        Data::DateTime(value) => value.to_string(),

        Data::DateTimeIso(value) => value.clone(),

        Data::DurationIso(value) => value.clone(),
    }
}

fn is_supported_excel_file(path: &Path) -> bool {
    path.extension()
        .and_then(|extension| extension.to_str())
        .map(|extension| {
            matches!(
                extension.to_lowercase().as_str(),
                "xlsx" | "xls" | "xlsm" | "xlsb" | "ods"
            )
        })
        .unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_integer_values() {
        assert_eq!(parse_u32("12".to_string()), Some(12));
    }

    #[test]
    fn rejects_fractional_integer_values() {
        assert_eq!(parse_u32("12.5".to_string()), None);
    }

    #[test]
    fn calculates_volume_from_dimensions() {
        assert_eq!(
            calculate_volume(Some(10.0), Some(20.0), Some(30.0),),
            Some(6000.0)
        );
    }

    #[tokio::test]
    async fn loads_excel_products() {
        let store = ExcelStore::new("data/excel");

        let products = store
            .load_products()
            .await
            .expect("Excel products should load");

        println!("Loaded {} products", products.len());

        for product in products.iter().take(10) {
            println!(
                "{} | {} | {}",
                product.sku, product.brand, product.product_name
            );
        }

        assert!(!products.is_empty(), "No products were loaded from Excel");
    }
}

#[async_trait]
impl ProductSource for ExcelStore {
    async fn products(&self) -> Result<Vec<Product>> {
        self.load_products().await
    }

    async fn product(&self, sku: &str) -> Result<Option<Product>> {
        Ok(self
            .load_products()
            .await?
            .into_iter()
            .find(|product| product.sku.eq_ignore_ascii_case(sku)))
    }

    async fn product_by_ean(&self, ean: &str) -> Result<Option<Product>> {
        Ok(self
            .load_products()
            .await?
            .into_iter()
            .find(|product| product.upc_code.as_deref() == Some(ean)))
    }

    async fn search_products(&self, search_term: &str) -> Result<Vec<Product>> {
        let search_term = search_term.trim().to_lowercase();

        if search_term.is_empty() {
            return Ok(Vec::new());
        }

        let products = self.load_products().await?;

        Ok(products
            .into_iter()
            .filter(|product| {
                product.sku.to_lowercase().contains(&search_term)
                    || product.brand.to_lowercase().contains(&search_term)
                    || product.product_name.to_lowercase().contains(&search_term)
                    || product
                        .upc_code
                        .as_deref()
                        .unwrap_or_default()
                        .to_lowercase()
                        .contains(&search_term)
            })
            .collect())
    }
}
