use chrono::NaiveDateTime;
use diesel::prelude::*;

use crate::domain::benchmark::{Benchmark as DomainBenchmark, NewBenchmark as DomainNewBenchmark};
use crate::domain::types::{
    BenchmarkName, BenchmarkSku, CategoryName, ProductAmount, ProductCount, ProductDescription,
    ProductPrice, ProductUnits, TypeConstraintError,
};

/// Diesel model representing a row in the `benchmarks` table.
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
    pub embedding: Option<Vec<u8>>,
    pub processing: bool,
    pub num_products: i32,
}

/// Insertable form of [`Benchmark`] used for creating new rows.
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
}

impl TryFrom<Benchmark> for DomainBenchmark {
    type Error = TypeConstraintError;

    fn try_from(benchmark: Benchmark) -> Result<Self, Self::Error> {
        Ok(Self {
            id: benchmark.id.try_into()?,
            hub_id: benchmark.hub_id.try_into()?,
            name: BenchmarkName::new(benchmark.name)?,
            sku: BenchmarkSku::new(benchmark.sku)?,
            category: CategoryName::new(benchmark.category)?,
            units: ProductUnits::new(benchmark.units)?,
            price: ProductPrice::new(benchmark.price)?,
            amount: ProductAmount::new(benchmark.amount)?,
            description: ProductDescription::new(benchmark.description)?,
            created_at: benchmark.created_at,
            updated_at: benchmark.updated_at,
            embedding: benchmark.embedding,
            processing: benchmark.processing,
            num_products: ProductCount::new(benchmark.num_products)?,
        })
    }
}

impl<'a> From<&'a DomainNewBenchmark> for NewBenchmark<'a> {
    fn from(benchmark: &'a DomainNewBenchmark) -> Self {
        Self {
            hub_id: benchmark.hub_id.get(),
            name: benchmark.name.as_str(),
            sku: benchmark.sku.as_str(),
            category: benchmark.category.as_str(),
            units: benchmark.units.as_str(),
            price: benchmark.price.get(),
            amount: benchmark.amount.get(),
            description: benchmark.description.as_str(),
            created_at: benchmark.created_at,
            updated_at: benchmark.updated_at,
        }
    }
}
