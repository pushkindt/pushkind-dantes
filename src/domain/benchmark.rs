use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize};

use crate::domain::types::{
    BenchmarkId, BenchmarkName, BenchmarkSku, CategoryName, HubId, ProductAmount, ProductCount,
    ProductDescription, ProductPrice, ProductUnits,
};

/// A benchmark reference product used for price comparisons.
///
/// This domain struct mirrors the `benchmarks` table and is
/// independent from any persistence layer representation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Benchmark {
    pub id: BenchmarkId,
    pub hub_id: HubId,
    pub name: BenchmarkName,
    pub sku: BenchmarkSku,
    pub category: CategoryName,
    pub units: ProductUnits,
    pub price: ProductPrice,
    pub amount: ProductAmount,
    pub description: ProductDescription,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
    pub embedding: Option<Vec<u8>>,
    pub processing: bool,
    pub num_products: ProductCount,
}

/// Data required to insert a new [`Benchmark`].
///
/// This struct is typically deserialized from incoming requests
/// before being converted into a database model.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct NewBenchmark {
    pub hub_id: HubId,
    pub name: BenchmarkName,
    pub sku: BenchmarkSku,
    pub category: CategoryName,
    pub units: ProductUnits,
    pub price: ProductPrice,
    pub amount: ProductAmount,
    pub description: ProductDescription,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
}
