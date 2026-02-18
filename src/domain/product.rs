use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize};

/// A product extracted from a crawler run.
#[derive(Debug, Clone, Serialize, Deserialize)]
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
    pub images: Vec<String>,
}

/// Information required to create a new [`Product`].
#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
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
    pub images: Vec<String>,
}
