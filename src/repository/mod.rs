use std::collections::HashMap;

use pushkind_common::db::{DbConnection, DbPool};
use pushkind_common::domain::benchmark::{Benchmark, NewBenchmark};
use pushkind_common::domain::crawler::Crawler;
use pushkind_common::domain::product::Product;
use pushkind_common::pagination::Pagination;
use pushkind_common::repository::errors::RepositoryResult;

pub mod benchmark;
pub mod crawler;
pub mod product;
#[cfg(test)]
pub mod test;

/// Repository implementation backed by Diesel and SQLite.
///
/// The underlying `r2d2::Pool` is cheap to clone, allowing the repository to
/// be passed around freely between handlers.
#[derive(Clone)]
pub struct DieselRepository {
    pool: DbPool, // r2d2::Pool is cheap to clone
}

impl DieselRepository {
    /// Create a new repository from an established database pool.
    pub fn new(pool: DbPool) -> Self {
        Self { pool }
    }

    /// Get a pooled database connection.
    fn conn(&self) -> RepositoryResult<DbConnection> {
        Ok(self.pool.get()?)
    }
}

/// Query parameters used when listing or searching products.
#[derive(Debug, Clone, Default)]
pub struct ProductListQuery {
    /// Filter by crawler identifier.
    pub crawler_id: Option<i32>,
    /// Filter by hub identifier.
    pub hub_id: Option<i32>,
    /// Restrict to products associated with a benchmark.
    pub benchmark_id: Option<i32>,
    /// Full-text search string.
    pub search: Option<String>,
    /// Pagination parameters.
    pub pagination: Option<Pagination>,
}

/// Query parameters for listing benchmarks belonging to a hub.
#[derive(Debug, Clone)]
pub struct BenchmarkListQuery {
    /// Hub identifier.
    pub hub_id: i32,
    /// Pagination parameters.
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

/// Read-only operations for crawler entities.
pub trait CrawlerReader {
    /// List all crawlers for a specific hub.
    fn list_crawlers(&self, hub_id: i32) -> RepositoryResult<Vec<Crawler>>;
    /// Retrieve a crawler by its identifier.
    fn get_crawler_by_id(&self, id: i32) -> RepositoryResult<Option<Crawler>>;
}

pub trait CrawlerWriter {}

/// Read-only operations for product entities.
pub trait ProductReader {
    /// List products matching the supplied query parameters.
    fn list_products(&self, query: ProductListQuery) -> RepositoryResult<(usize, Vec<Product>)>;
    /// Return a mapping of product identifiers to similarity distances for a benchmark.
    fn list_distances(&self, benchmark_id: i32) -> RepositoryResult<HashMap<i32, f32>>;
    /// Perform a full-text search for products.
    fn search_products(&self, query: ProductListQuery) -> RepositoryResult<(usize, Vec<Product>)>;
    /// Retrieve a product by its identifier.
    fn get_product_by_id(&self, id: i32) -> RepositoryResult<Option<Product>>;
}

pub trait ProductWriter {}

/// Read-only operations for benchmark entities.
pub trait BenchmarkReader {
    /// List benchmarks using the supplied query options.
    fn list_benchmarks(
        &self,
        query: BenchmarkListQuery,
    ) -> RepositoryResult<(usize, Vec<Benchmark>)>;
    /// Retrieve a benchmark by its identifier.
    fn get_benchmark_by_id(&self, id: i32) -> RepositoryResult<Option<Benchmark>>;
}

/// Write operations for benchmark entities and their associations.
pub trait BenchmarkWriter {
    /// Persist new benchmark records.
    fn create_benchmark(&self, benchmarks: &[NewBenchmark]) -> RepositoryResult<usize>;
    /// Remove an association between a benchmark and a product.
    fn remove_benchmark_association(
        &self,
        benchmark_id: i32,
        product_id: i32,
    ) -> RepositoryResult<usize>;
    /// Create or update an association between a benchmark and a product with a similarity distance.
    fn set_benchmark_association(
        &self,
        benchmark_id: i32,
        product_id: i32,
        distance: f32,
    ) -> RepositoryResult<usize>;
}
