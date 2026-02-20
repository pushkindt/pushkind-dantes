use std::collections::HashMap;

use pushkind_common::repository::errors::RepositoryResult;

use crate::domain::benchmark::NewBenchmark;
use crate::domain::category::Category;
use crate::domain::types::{
    BenchmarkId, CategoryId, CategoryName, CrawlerId, HubId, ProductId, SimilarityDistance,
};
use crate::domain::{benchmark::Benchmark, crawler::Crawler, product::Product};
use crate::repository::{
    BenchmarkListQuery, BenchmarkReader, BenchmarkWriter, CategoryListQuery, CategoryReader,
    CategoryWriter, CrawlerReader, ProcessingStateReader, ProductListQuery, ProductReader,
    ProductWriter,
};

/// Simple in-memory repository used for unit tests.
#[derive(Default)]
pub struct TestRepository {
    crawlers: HashMap<CrawlerId, Crawler>,
    products: Vec<Product>,
    benchmarks: Vec<Benchmark>,
    categories: Vec<Category>,
}

impl TestRepository {
    pub fn new(crawlers: Vec<Crawler>, products: Vec<Product>, benchmarks: Vec<Benchmark>) -> Self {
        Self {
            crawlers: crawlers.into_iter().map(|c| (c.id, c)).collect(),
            products,
            benchmarks,
            categories: vec![],
        }
    }

    pub fn with_categories(mut self, categories: Vec<Category>) -> Self {
        self.categories = categories;
        self
    }

    fn clone_crawler(c: &Crawler) -> Crawler {
        c.clone()
    }

    fn clone_product(p: &Product) -> Product {
        p.clone()
    }

    fn clone_benchmark(b: &Benchmark) -> Benchmark {
        b.clone()
    }

    fn clone_category(c: &Category) -> Category {
        c.clone()
    }
}

impl CrawlerReader for TestRepository {
    fn list_crawlers(&self, hub_id: HubId) -> RepositoryResult<Vec<Crawler>> {
        Ok(self
            .crawlers
            .values()
            .filter(|c| c.hub_id == hub_id)
            .map(Self::clone_crawler)
            .collect())
    }

    fn get_crawler_by_id(
        &self,
        id: CrawlerId,
        _hub_id: HubId,
    ) -> RepositoryResult<Option<Crawler>> {
        Ok(self.crawlers.get(&id).map(Self::clone_crawler))
    }
}

impl ProcessingStateReader for TestRepository {
    fn has_active_processing(&self, hub_id: HubId) -> RepositoryResult<bool> {
        let crawler_processing = self
            .crawlers
            .values()
            .any(|crawler| crawler.hub_id == hub_id && crawler.processing);

        if crawler_processing {
            return Ok(true);
        }

        let benchmark_processing = self
            .benchmarks
            .iter()
            .any(|benchmark| benchmark.hub_id == hub_id && benchmark.processing);

        Ok(benchmark_processing)
    }
}

impl ProductReader for TestRepository {
    fn list_products(&self, query: ProductListQuery) -> RepositoryResult<(usize, Vec<Product>)> {
        let mut items: Vec<Product> = self.products.iter().map(Self::clone_product).collect();
        if let Some(crawler_id) = query.crawler_id {
            items.retain(|p| p.crawler_id == crawler_id);
        }
        let total = items.len();
        Ok((total, items))
    }

    fn list_distances(
        &self,
        _benchmark_id: BenchmarkId,
    ) -> RepositoryResult<HashMap<ProductId, SimilarityDistance>> {
        Ok(HashMap::new())
    }

    fn search_products(&self, query: ProductListQuery) -> RepositoryResult<(usize, Vec<Product>)> {
        let mut items: Vec<Product> = self.products.iter().map(Self::clone_product).collect();
        if let Some(crawler_id) = query.crawler_id {
            items.retain(|p| p.crawler_id == crawler_id);
        }
        if let Some(search) = query.search {
            let search = search.to_lowercase();
            items.retain(|p| p.name.to_lowercase().contains(&search));
        }
        let total = items.len();
        Ok((total, items))
    }

    fn get_product_by_id(&self, id: ProductId) -> RepositoryResult<Option<Product>> {
        Ok(self
            .products
            .iter()
            .find(|p| p.id == id)
            .map(Self::clone_product))
    }
}

impl ProductWriter for TestRepository {
    fn set_product_category_manual(
        &self,
        _product_id: ProductId,
        _category_id: CategoryId,
    ) -> RepositoryResult<usize> {
        Ok(1)
    }

    fn clear_product_category_manual(&self, _product_id: ProductId) -> RepositoryResult<usize> {
        Ok(1)
    }
}

impl BenchmarkReader for TestRepository {
    fn list_benchmarks(
        &self,
        query: BenchmarkListQuery,
    ) -> RepositoryResult<(usize, Vec<Benchmark>)> {
        let mut items: Vec<Benchmark> = self.benchmarks.iter().map(Self::clone_benchmark).collect();
        items.retain(|b| b.hub_id == query.hub_id);
        let total = items.len();
        Ok((total, items))
    }

    fn get_benchmark_by_id(
        &self,
        id: BenchmarkId,
        _hub_id: HubId,
    ) -> RepositoryResult<Option<Benchmark>> {
        Ok(self
            .benchmarks
            .iter()
            .find(|b| b.id == id)
            .map(Self::clone_benchmark))
    }
}

impl BenchmarkWriter for TestRepository {
    fn create_benchmark(&self, benchmarks: &[NewBenchmark]) -> RepositoryResult<usize> {
        Ok(benchmarks.len())
    }

    fn remove_benchmark_association(
        &self,
        _benchmark_id: BenchmarkId,
        _product_id: ProductId,
    ) -> RepositoryResult<usize> {
        Ok(1)
    }

    fn set_benchmark_association(
        &self,
        _benchmark_id: BenchmarkId,
        _product_id: ProductId,
        _distance: SimilarityDistance,
    ) -> RepositoryResult<usize> {
        Ok(1)
    }
}

impl CategoryReader for TestRepository {
    fn list_categories(
        &self,
        query: CategoryListQuery,
    ) -> RepositoryResult<(usize, Vec<Category>)> {
        let mut items: Vec<Category> = self
            .categories
            .iter()
            .filter(|c| c.hub_id == query.hub_id)
            .map(Self::clone_category)
            .collect();
        let total = items.len();
        if let Some(pagination) = query.pagination {
            let start = (pagination.page.saturating_sub(1)) * pagination.per_page;
            items = items
                .into_iter()
                .skip(start)
                .take(pagination.per_page)
                .collect();
        }
        Ok((total, items))
    }

    fn get_category_by_id(
        &self,
        id: CategoryId,
        hub_id: HubId,
    ) -> RepositoryResult<Option<Category>> {
        Ok(self
            .categories
            .iter()
            .find(|c| c.id == id && c.hub_id == hub_id)
            .map(Self::clone_category))
    }
}

impl CategoryWriter for TestRepository {
    fn create_category(
        &self,
        _category: &crate::domain::category::NewCategory,
    ) -> RepositoryResult<usize> {
        Ok(1)
    }

    fn update_category(
        &self,
        _id: CategoryId,
        _hub_id: HubId,
        _name: &CategoryName,
        _embedding: Option<&[u8]>,
    ) -> RepositoryResult<usize> {
        Ok(1)
    }

    fn delete_category(&self, _id: CategoryId, _hub_id: HubId) -> RepositoryResult<usize> {
        Ok(1)
    }
}
