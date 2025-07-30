use chrono::NaiveDateTime;
use diesel::prelude::*;

use crate::domain::benchmark::{Benchmark as DomainBenchmark, NewBenchmark as DomainNewBenchmark};
use crate::embedding::PromptEmbedding;

#[derive(Debug, Clone, Identifiable, Queryable)]
#[diesel(table_name = crate::schema::benchmarks)]
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
    pub embedding: Vec<u8>,
}

#[derive(Insertable)]
#[diesel(table_name = crate::schema::benchmarks)]
pub struct NewBenchmark<'a> {
    pub hub_id: i32,
    pub name: &'a str,
    pub sku: &'a str,
    pub category: &'a str,
    pub units: &'a str,
    pub price: f64,
    pub amount: f64,
    pub description: &'a str,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
    pub embedding: Vec<u8>,
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

impl PromptEmbedding for NewBenchmark<'_> {
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

impl From<Benchmark> for DomainBenchmark {
    fn from(benchmark: Benchmark) -> Self {
        Self {
            id: benchmark.id,
            hub_id: benchmark.hub_id,
            name: benchmark.name,
            sku: benchmark.sku,
            category: benchmark.category,
            units: benchmark.units,
            price: benchmark.price,
            amount: benchmark.amount,
            description: benchmark.description,
            created_at: benchmark.created_at,
            updated_at: benchmark.updated_at,
            embedding: benchmark
                .embedding
                .chunks_exact(4)
                .map(|b| f32::from_le_bytes([b[0], b[1], b[2], b[3]]))
                .collect(),
        }
    }
}

impl<'a> From<&'a DomainNewBenchmark> for NewBenchmark<'a> {
    fn from(benchmark: &'a DomainNewBenchmark) -> Self {
        Self {
            hub_id: benchmark.hub_id,
            name: benchmark.name.as_str(),
            sku: benchmark.sku.as_str(),
            category: benchmark.category.as_str(),
            units: benchmark.units.as_str(),
            price: benchmark.price,
            amount: benchmark.amount,
            description: benchmark.description.as_str(),
            created_at: benchmark.created_at,
            updated_at: benchmark.updated_at,
            embedding: benchmark
                .embedding
                .iter()
                .flat_map(|f| f.to_le_bytes())
                .collect::<Vec<u8>>(),
        }
    }
}
