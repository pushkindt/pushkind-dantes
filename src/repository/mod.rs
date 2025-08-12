use std::collections::HashMap;

use pushkind_common::db::DbPool;
use pushkind_common::domain::benchmark::{Benchmark, NewBenchmark};
use pushkind_common::domain::crawler::Crawler;
use pushkind_common::domain::product::Product;
use pushkind_common::pagination::Pagination;
use pushkind_common::repository::errors::RepositoryResult;

pub mod benchmark;
pub mod crawler;
pub mod product;

pub struct DieselRepository<'a> {
    pub pool: &'a DbPool,
}

impl<'a> DieselRepository<'a> {
    pub fn new(pool: &'a DbPool) -> Self {
        Self { pool }
    }
}

#[derive(Debug, Clone, Default)]
pub struct ProductListQuery {
    pub crawler_id: Option<i32>,
    pub hub_id: Option<i32>,
    pub benchmark_id: Option<i32>,
    pub search: Option<String>,
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
    pub fn crawler(mut self, crawler_id: i32) -> Self {
        self.crawler_id = Some(crawler_id);
        self
    }
    pub fn hub_id(mut self, hub_id: i32) -> Self {
        self.hub_id = Some(hub_id);
        self
    }
    pub fn benchmark(mut self, benchmark_id: i32) -> Self {
        self.benchmark_id = Some(benchmark_id);
        self
    }
    pub fn search(mut self, search: impl Into<String>) -> Self {
        self.search = Some(search.into());
        self
    }
    pub fn paginate(mut self, page: usize, per_page: usize) -> Self {
        self.pagination = Some(Pagination { page, per_page });
        self
    }
}

pub trait CrawlerReader {
    fn list_crawlers(&self, hub_id: i32) -> RepositoryResult<Vec<Crawler>>;
    fn get_crawler_by_id(&self, id: i32) -> RepositoryResult<Option<Crawler>>;
}

pub trait CrawlerWriter {}

pub trait ProductReader {
    fn list_products(&self, query: ProductListQuery) -> RepositoryResult<(usize, Vec<Product>)>;
    fn list_distances(&self, benchmark_id: i32) -> RepositoryResult<HashMap<i32, f32>>;
    fn search_products(&self, query: ProductListQuery) -> RepositoryResult<(usize, Vec<Product>)>;
    fn get_product_by_id(&self, id: i32) -> RepositoryResult<Option<Product>>;
}

pub trait ProductWriter {}

pub trait BenchmarkReader {
    fn list_benchmarks(
        &self,
        query: BenchmarkListQuery,
    ) -> RepositoryResult<(usize, Vec<Benchmark>)>;
    fn get_benchmark_by_id(&self, id: i32) -> RepositoryResult<Option<Benchmark>>;
}

pub trait BenchmarkWriter {
    fn create_benchmark(&self, benchmarks: &[NewBenchmark]) -> RepositoryResult<usize>;
    fn remove_benchmark_association(
        &self,
        benchmark_id: i32,
        product_id: i32,
    ) -> RepositoryResult<usize>;
    fn set_benchmark_association(
        &self,
        benchmark_id: i32,
        product_id: i32,
        distance: f32,
    ) -> RepositoryResult<usize>;
}
