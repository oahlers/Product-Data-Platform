use crate::models::{product::Product, result::Labels};

pub async fn run(product: &Product) -> Labels {
    fn label(lang: &str, p: &Product) -> String {
        format!("{lang}\nProduct: {}\nDirections: {}\nWarnings: {}\nIngredients: {}",
            p.product_name,
            if p.directions.is_empty() { "Not available" } else { &p.directions },
            if p.warnings.is_empty() { "Not available" } else { &p.warnings },
            if p.inci.is_empty() { "Not available" } else { &p.inci })
    }
    Labels { da: label("DANSK", product), sv: label("SVENSKA", product), no: label("NORSK", product), fi: label("SUOMI", product) }
}
