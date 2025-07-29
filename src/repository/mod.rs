use pushkind_common::pagination::Pagination;
use pushkind_common::repository::errors::RepositoryResult;

use crate::domain::benchmark::{Benchmark, NewBenchmark};
use crate::domain::crawler::Crawler;
use crate::domain::product::Product;

pub mod benchmark;
pub mod crawler;
pub mod product;

#[derive(Debug, Clone)]
pub struct ProductListQuery {
    pub crawler_id: i32,
    pub pagination: Option<Pagination>,
}

#[derive(Debug, Clone)]
pub struct BenchmarkListQuery {
    pub hub_id: i32,
    pub pagination: Option<Pagination>,
}

impl BenchmarkListQuery {
    pub fn new(hub_id: i32) -> Self {
        Self {
            hub_id,
            pagination: None,
        }
    }
    pub fn paginate(mut self, page: usize, per_page: usize) -> Self {
        self.pagination = Some(Pagination { page, per_page });
        self
    }
}

impl ProductListQuery {
    pub fn new(crawler_id: i32) -> Self {
        Self {
            crawler_id,
            pagination: None,
        }
    }
    pub fn paginate(mut self, page: usize, per_page: usize) -> Self {
        self.pagination = Some(Pagination { page, per_page });
        self
    }
}

pub trait CrawlerReader {
    fn list(&self, hub_id: i32) -> RepositoryResult<Vec<Crawler>>;
    fn get_by_id(&self, id: i32) -> RepositoryResult<Option<Crawler>>;
}

pub trait CrawlerWriter {
    fn set_processing(&self, id: i32, status: bool) -> RepositoryResult<usize>;
}

pub trait ProductReader {
    fn list(&self, query: ProductListQuery) -> RepositoryResult<(usize, Vec<Product>)>;
}

pub trait ProductWriter {}

pub trait BenchmarkReader {
    fn list(&self, query: BenchmarkListQuery) -> RepositoryResult<(usize, Vec<Benchmark>)>;
}

pub trait BenchmarkWriter {
    fn create(&self, benchmarks: &[NewBenchmark]) -> RepositoryResult<usize>;
}
