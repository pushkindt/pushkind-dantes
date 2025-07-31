use chrono::NaiveDateTime;
use serde::Serialize;

#[derive(Serialize)]
pub struct Benchmark {
    pub id: i32,
    pub hub_id: i32,
    pub name: String,
    pub sku: String,
    pub category: String,
    pub units: String,
    pub price: f64,
    pub amount: f64,
    pub description: String,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
}

pub struct NewBenchmark {
    pub hub_id: i32,
    pub name: String,
    pub sku: String,
    pub category: String,
    pub units: String,
    pub price: f64,
    pub amount: f64,
    pub description: String,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
}
