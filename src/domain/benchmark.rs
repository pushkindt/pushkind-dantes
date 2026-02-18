use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize};

/// A benchmark reference product used for price comparisons.
///
/// This domain struct mirrors the `benchmarks` table and is
/// independent from any persistence layer representation.
#[derive(Debug, Clone, Serialize, Deserialize)]
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
    pub embedding: Option<Vec<u8>>,
    pub processing: bool,
    pub num_products: i32,
}

/// Data required to insert a new [`Benchmark`].
///
/// This struct is typically deserialized from incoming requests
/// before being converted into a database model.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
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
