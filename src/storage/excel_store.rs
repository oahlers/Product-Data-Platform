use std::{
    collections::HashMap,
    path::{Path, PathBuf},
};

use anyhow::{Context, Result};
use calamine::{open_workbook_auto, Data, Reader};
use chrono::Utc;
use tokio::task;
use tracing::{info, warn};

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

    pub async fn products(&self) -> Result<Vec<Product>> {
        let directory = self.directory.clone();

        task::spawn_blocking(move || load_directory(&directory))
            .await
            .context("Excel reader task failed")?
    }

    pub async fn product(&self, sku: &str) -> Result<Option<Product>> {
        Ok(self
            .products()
            .await?
            .into_iter()
            .find(|product| product.sku.eq_ignore_ascii_case(sku)))
    }
}

fn load_directory(directory: &Path) -> Result<Vec<Product>> {
    if !directory.exists() {
        anyhow::bail!(
            "Excel directory does not exist: {}",
            directory.display()
        );
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

    info!(
        count = products.len(),
        "Excel product ingestion completed"
    );

    Ok(products)
}

fn load_workbook(path: &Path) -> Result<Vec<Product>> {
    let mut workbook = open_workbook_auto(path)
        .with_context(|| format!("Cannot open {}", path.display()))?;

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
    rows.iter()
        .take(100)
        .enumerate()
        .find_map(|(index, row)| {
            let normalized: Vec<String> =
                row.iter().map(|value| normalize(value)).collect();

            let has_sku = normalized.iter().any(|value| {
                matches!(
                    value.as_str(),
                    "sku"
                        | "kendo sku"
                        | "item number"
                        | "reference-no"
                        | "reference no"
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

    let description = first_value(
        row,
        headers,
        &[
            "product name / long description",
            "long description en",
            "trade item description long description-en_gb-sephora",
            "sephora product long description-en_gb",
        ],
    );

    let marketing_text = first_value(
        row,
        headers,
        &["marketing message", "trade item marketing message-en_gb"],
    );

    let inci = first_value(
        row,
        headers,
        &["ingredients list", "ingredient statement-en_gb", "inci"],
    );

    let warnings = first_value(
        row,
        headers,
        &["precaution of use", "precautions", "warnings"],
    );

    let directions = first_value(
        row,
        headers,
        &[
            "application tips",
            "preparation instructions-en_gb",
            "directions",
            "directions for use",
        ],
    );

    Some(Product {
        sku,
        brand,
        product_name,
        description,
        marketing_text,
        inci,
        warnings,
        directions,
        net_weight_g: parse_number(first_value(
            row,
            headers,
            &["net weight (g)", "net weight", "net_weight"],
        )),
        gross_weight_g: parse_number(first_value(
            row,
            headers,
            &["gross weight(g)", "gross weight", "gross_weight"],
        )),
        width_mm: parse_number(first_value(
            row,
            headers,
            &[
                "primary width (mm)",
                "secondary width (mm)",
                "item width (m)",
                "width",
            ],
        )),
        depth_mm: parse_number(first_value(
            row,
            headers,
            &[
                "primary depth (mm)",
                "secondary depth (mm)",
                "item lenght (m)",
                "item length (m)",
                "depth",
            ],
        )),
        height_mm: parse_number(first_value(
            row,
            headers,
            &[
                "primary height (mm)",
                "secondary height (mm)",
                "item height (m)",
                "height",
            ],
        )),
        modified_at: Utc::now(),
    })
}

fn first_value(
    row: &[String],
    headers: &HashMap<String, usize>,
    aliases: &[&str],
) -> String {
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

fn parse_number(value: String) -> Option<f64> {
    let normalized = value.trim().replace(' ', "").replace(',', ".");

    if normalized.is_empty() {
        return None;
    }

    normalized.parse::<f64>().ok()
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
    products.sort_by(|left, right| {
        left.sku
            .to_lowercase()
            .cmp(&right.sku.to_lowercase())
    });

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

    #[tokio::test]
    async fn loads_excel_products() {
        let store = ExcelStore::new("data/excel");

        let products = store
            .products()
            .await
            .expect("Excel products should load");

        println!("Loaded {} products", products.len());

        for product in products.iter().take(10) {
            println!(
                "{} | {} | {}",
                product.sku, product.brand, product.product_name
            );
        }

        assert!(
            !products.is_empty(),
            "No products were loaded from Excel"
        );
    }
}
