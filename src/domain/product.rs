use chrono::NaiveDateTime;
use serde::Serialize;

#[derive(Serialize)]
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
}
