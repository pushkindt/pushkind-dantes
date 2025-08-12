use std::collections::HashMap;

use pushkind_common::domain::{crawler::Crawler, product::Product};
use pushkind_common::repository::errors::RepositoryResult;

use crate::repository::{CrawlerReader, ProductListQuery, ProductReader};

/// Simple in-memory repository used for unit tests.
#[derive(Default)]
pub struct TestRepository {
    crawlers: HashMap<i32, Crawler>,
    products: Vec<Product>,
}

impl TestRepository {
    pub fn new(crawlers: Vec<Crawler>, products: Vec<Product>) -> Self {
        Self {
            crawlers: crawlers.into_iter().map(|c| (c.id, c)).collect(),
            products,
        }
    }

    fn clone_crawler(c: &Crawler) -> Crawler {
        Crawler {
            id: c.id,
            hub_id: c.hub_id,
            name: c.name.clone(),
            url: c.url.clone(),
            selector: c.selector.clone(),
            processing: c.processing,
            updated_at: c.updated_at,
            num_products: c.num_products,
        }
    }

    fn clone_product(p: &Product) -> Product {
        Product {
            id: p.id,
            crawler_id: p.crawler_id,
            name: p.name.clone(),
            sku: p.sku.clone(),
            category: p.category.clone(),
            units: p.units.clone(),
            price: p.price,
            amount: p.amount,
            description: p.description.clone(),
            url: p.url.clone(),
            created_at: p.created_at,
            updated_at: p.updated_at,
            embedding: p.embedding.clone(),
        }
    }
}

impl CrawlerReader for TestRepository {
    fn list_crawlers(&self, hub_id: i32) -> RepositoryResult<Vec<Crawler>> {
        Ok(
            self
                .crawlers
                .values()
                .filter(|c| c.hub_id == hub_id)
                .map(Self::clone_crawler)
                .collect(),
        )
    }

    fn get_crawler_by_id(&self, id: i32) -> RepositoryResult<Option<Crawler>> {
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

    fn list_distances(&self, _benchmark_id: i32) -> RepositoryResult<HashMap<i32, f32>> {
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

    fn get_product_by_id(&self, id: i32) -> RepositoryResult<Option<Product>> {
        Ok(self
            .products
            .iter()
            .find(|p| p.id == id)
            .map(Self::clone_product))
    }
}

