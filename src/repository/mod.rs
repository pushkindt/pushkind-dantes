use std::collections::HashMap;

use pushkind_common::db::{DbConnection, DbPool};
use pushkind_common::pagination::Pagination;
use pushkind_common::repository::errors::RepositoryResult;

use crate::domain::benchmark::{Benchmark, NewBenchmark};
use crate::domain::category::{Category, NewCategory};
use crate::domain::crawler::Crawler;
use crate::domain::product::Product;
use crate::domain::types::{
    BenchmarkId, CategoryId, CategoryName, CrawlerId, HubId, ProductId, SimilarityDistance,
};

pub mod benchmark;
pub mod category;
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
    pub crawler_id: Option<CrawlerId>,
    /// Filter by hub identifier.
    pub hub_id: Option<HubId>,
    /// Restrict to products associated with a benchmark.
    pub benchmark_id: Option<BenchmarkId>,
    /// Full-text search string.
    pub search: Option<String>,
    /// Pagination parameters.
    pub pagination: Option<Pagination>,
}

/// Query parameters for listing benchmarks belonging to a hub.
#[derive(Debug, Clone)]
pub struct BenchmarkListQuery {
    /// Hub identifier.
    pub hub_id: HubId,
    /// Pagination parameters.
    pub pagination: Option<Pagination>,
}

/// Query parameters for listing categories belonging to a hub.
#[derive(Debug, Clone)]
pub struct CategoryListQuery {
    /// Hub identifier.
    pub hub_id: HubId,
    /// Pagination parameters.
    pub pagination: Option<Pagination>,
}

impl CategoryListQuery {
    pub fn new(hub_id: HubId) -> Self {
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

impl BenchmarkListQuery {
    pub fn new(hub_id: HubId) -> Self {
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
    pub fn crawler(mut self, crawler_id: CrawlerId) -> Self {
        self.crawler_id = Some(crawler_id);
        self
    }
    pub fn hub_id(mut self, hub_id: HubId) -> Self {
        self.hub_id = Some(hub_id);
        self
    }
    pub fn benchmark(mut self, benchmark_id: BenchmarkId) -> Self {
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
    fn list_crawlers(&self, hub_id: HubId) -> RepositoryResult<Vec<Crawler>>;
    /// Retrieve a crawler by its identifier.
    fn get_crawler_by_id(&self, id: CrawlerId, hub_id: HubId) -> RepositoryResult<Option<Crawler>>;
}

pub trait CrawlerWriter {}

/// Read-only operations for product entities.
pub trait ProductReader {
    /// List products matching the supplied query parameters.
    fn list_products(&self, query: ProductListQuery) -> RepositoryResult<(usize, Vec<Product>)>;
    /// Return a mapping of product identifiers to similarity distances for a benchmark.
    fn list_distances(
        &self,
        benchmark_id: BenchmarkId,
    ) -> RepositoryResult<HashMap<ProductId, SimilarityDistance>>;
    /// Perform a full-text search for products.
    fn search_products(&self, query: ProductListQuery) -> RepositoryResult<(usize, Vec<Product>)>;
    /// Retrieve a product by its identifier.
    fn get_product_by_id(&self, id: ProductId) -> RepositoryResult<Option<Product>>;
}

pub trait ProductWriter {
    /// Set a manual category assignment for a product.
    fn set_product_category_manual(
        &self,
        product_id: ProductId,
        category_id: CategoryId,
    ) -> RepositoryResult<usize>;
    /// Clear manual category assignment and mark source as automatic.
    fn clear_product_category_manual(&self, product_id: ProductId) -> RepositoryResult<usize>;
}

/// Read-only operations for category entities.
pub trait CategoryReader {
    /// List categories using the supplied query options.
    fn list_categories(&self, query: CategoryListQuery)
    -> RepositoryResult<(usize, Vec<Category>)>;
    /// Retrieve a category by its identifier and hub.
    fn get_category_by_id(
        &self,
        id: CategoryId,
        hub_id: HubId,
    ) -> RepositoryResult<Option<Category>>;
}

/// Write operations for category entities.
pub trait CategoryWriter {
    /// Persist a new category.
    fn create_category(&self, category: &NewCategory) -> RepositoryResult<usize>;
    /// Update category name and embedding.
    fn update_category(
        &self,
        id: CategoryId,
        hub_id: HubId,
        name: &CategoryName,
        embedding: &[u8],
    ) -> RepositoryResult<usize>;
    /// Delete a category by id and hub.
    fn delete_category(&self, id: CategoryId, hub_id: HubId) -> RepositoryResult<usize>;
}

/// Read-only operations for benchmark entities.
pub trait BenchmarkReader {
    /// List benchmarks using the supplied query options.
    fn list_benchmarks(
        &self,
        query: BenchmarkListQuery,
    ) -> RepositoryResult<(usize, Vec<Benchmark>)>;
    /// Retrieve a benchmark by its identifier.
    fn get_benchmark_by_id(
        &self,
        id: BenchmarkId,
        hub_id: HubId,
    ) -> RepositoryResult<Option<Benchmark>>;
}

/// Write operations for benchmark entities and their associations.
pub trait BenchmarkWriter {
    /// Persist new benchmark records.
    fn create_benchmark(&self, benchmarks: &[NewBenchmark]) -> RepositoryResult<usize>;
    /// Remove an association between a benchmark and a product.
    fn remove_benchmark_association(
        &self,
        benchmark_id: BenchmarkId,
        product_id: ProductId,
    ) -> RepositoryResult<usize>;
    /// Create or update an association between a benchmark and a product with a similarity distance.
    fn set_benchmark_association(
        &self,
        benchmark_id: BenchmarkId,
        product_id: ProductId,
        distance: SimilarityDistance,
    ) -> RepositoryResult<usize>;
}
