use std::collections::HashMap;

use pushkind_common::repository::errors::RepositoryResult;

use crate::domain::benchmark::NewBenchmark;
use crate::domain::types::{BenchmarkId, CrawlerId, HubId, ProductId, SimilarityDistance};
use crate::domain::{benchmark::Benchmark, crawler::Crawler, product::Product};
use crate::repository::{
    BenchmarkListQuery, BenchmarkReader, BenchmarkWriter, CrawlerReader, ProductListQuery,
    ProductReader,
};

/// Simple in-memory repository used for unit tests.
#[derive(Default)]
pub struct TestRepository {
    crawlers: HashMap<CrawlerId, Crawler>,
    products: Vec<Product>,
    benchmarks: Vec<Benchmark>,
}

impl TestRepository {
    pub fn new(crawlers: Vec<Crawler>, products: Vec<Product>, benchmarks: Vec<Benchmark>) -> Self {
        Self {
            crawlers: crawlers.into_iter().map(|c| (c.id, c)).collect(),
            products,
            benchmarks,
        }
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
