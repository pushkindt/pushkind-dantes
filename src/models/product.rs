use chrono::NaiveDateTime;
use diesel::prelude::*;

use crate::domain::product::{NewProduct as DomainNewProduct, Product as DomainProduct};

/// Diesel model representing the `products` table.
#[derive(Debug, Clone, Identifiable, Queryable, QueryableByName)]
#[diesel(table_name = crate::schema::products)]
#[diesel(foreign_derive)]
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

/// Insertable/patchable form of [`Product`].
#[derive(Debug, Insertable, AsChangeset)]
#[diesel(table_name = crate::schema::products)]
pub struct NewProduct {
    pub crawler_id: i32,
    pub name: String,
    pub sku: String,
    pub category: Option<String>,
    pub units: Option<String>,
    pub price: f64,
    pub amount: Option<f64>,
    pub description: Option<String>,
    pub url: String,
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
            embedding: product.embedding,
            images: vec![],
        }
    }
}

impl From<DomainNewProduct> for NewProduct {
    fn from(product: DomainNewProduct) -> Self {
        Self {
            crawler_id: product.crawler_id,
            name: product.name,
            sku: product.sku,
            category: product.category,
            units: product.units,
            price: product.price,
            amount: product.amount,
            description: product.description,
            url: product.url,
        }
    }
}
