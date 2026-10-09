use crate::models::product::Product;

pub fn build_validation_context(
    product: &Product,
) -> String {
    format!(
        "
SKU: {}
Brand: {}
Product Name: {}

Description:
{}

Marketing Text:
{}

Warnings:
{}

Directions:
{}

INCI:
{}
",
        product.sku,
        product.brand,
        product.product_name,
        product.description,
        product.marketing_text,
        product.warnings,
        product.directions,
        product.inci,
    )
}

pub fn build_translation_context(
    product: &Product,
) -> String {
    format!(
        "
Product Name:
{}

Description:
{}

Marketing Text:
{}

Warnings:
{}

Directions:
{}
",
        product.product_name,
        product.description,
        product.marketing_text,
        product.warnings,
        product.directions,
    )
}

pub fn build_inci_context(
    product: &Product,
) -> String {
    format!(
        "
SKU: {}

INCI:
{}
",
        product.sku,
        product.inci,
    )
}