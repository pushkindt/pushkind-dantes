use chrono::NaiveDateTime;
use serde::Serialize;

use crate::embedding::PromptEmbedding;

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
    pub embedding: Vec<f32>,
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
    pub embedding: Vec<f32>,
}

impl PromptEmbedding for Benchmark {
    fn prompt(&self) -> String {
        format!(
            "Name: {}\nSKU: {}\nCategory: {}\nUnits: {}\nPrice: {}\nAmount: {}\nDescription: {}",
            self.name,
            self.sku,
            self.category,
            self.units,
            self.price,
            self.amount,
            self.description
        )
    }
}

impl PromptEmbedding for NewBenchmark {
    fn prompt(&self) -> String {
        format!(
            "Name: {}\nSKU: {}\nCategory: {}\nUnits: {}\nPrice: {}\nAmount: {}\nDescription: {}",
            self.name,
            self.sku,
            self.category,
            self.units,
            self.price,
            self.amount,
            self.description
        )
    }
}
