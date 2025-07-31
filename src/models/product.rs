use chrono::NaiveDateTime;
use diesel::prelude::*;

use crate::domain::product::Product as DomainProduct;

#[derive(Debug, Clone, Identifiable, Queryable)]
#[diesel(table_name = crate::schema::products)]
pub struct Product {
    pub id: i32,
    pub crawler_id: i32,
    pub name: String,
    pub sku: String,
    pub category: Option<String>,
    pub units: Option<String>,
    pub price: f64,
    pub amount: Option<f64>,
    pub description: Option<String>,
    pub url: String,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
    pub embedding: Option<Vec<u8>>,
}

impl From<Product> for DomainProduct {
    fn from(product: Product) -> Self {
        Self {
            id: product.id,
            crawler_id: product.crawler_id,
            name: product.name,
            sku: product.sku,
            category: product.category,
            units: product.units,
            price: product.price,
            amount: product.amount,
            description: product.description,
            url: product.url,
            created_at: product.created_at,
            updated_at: product.updated_at,
        }
    }
}
