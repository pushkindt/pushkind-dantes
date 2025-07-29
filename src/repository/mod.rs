use pushkind_common::repository::errors::RepositoryResult;

use crate::domain::benchmark::{Benchmark, NewBenchmark};
use crate::domain::crawler::Crawler;
use crate::domain::product::Product;

pub mod benchmark;
pub mod crawler;
pub mod product;

pub trait CrawlerReader {
    fn list(&self, hub_id: i32) -> RepositoryResult<Vec<Crawler>>;
    fn get_by_id(&self, id: i32) -> RepositoryResult<Option<Crawler>>;
}

pub trait CrawlerWriter {
    fn set_processing(&self, id: i32, status: bool) -> RepositoryResult<usize>;
}

pub trait ProductReader {
    fn list(&self, crawler_id: i32) -> RepositoryResult<Vec<Product>>;
}

pub trait ProductWriter {}

pub trait BenchmarkReader {
    fn list(&self, hub_id: i32) -> RepositoryResult<Vec<Benchmark>>;
}

pub trait BenchmarkWriter {
    fn create(&self, benchmarks: &[NewBenchmark]) -> RepositoryResult<usize>;
}
