use std::collections::HashMap;

use log::error;
use pushkind_common::domain::{benchmark::Benchmark, crawler::Crawler, product::Product};
use pushkind_common::models::auth::AuthenticatedUser;
use pushkind_common::pagination::{DEFAULT_ITEMS_PER_PAGE, Paginated};
use pushkind_common::routes::ensure_role;

use crate::repository::{
    BenchmarkListQuery, BenchmarkReader, CrawlerReader, ProductListQuery, ProductReader,
};

use super::errors::{ServiceError, ServiceResult};

/// Core business logic for rendering the benchmarks page.
///
/// Validates the `parser` role and fetches paginated benchmarks for the
/// user's hub. Repository errors are translated into [`ServiceError`] so the
/// HTTP route can remain a thin wrapper.
pub fn show_benchmarks<R>(repo: &R, user: &AuthenticatedUser) -> ServiceResult<Vec<Benchmark>>
where
    R: BenchmarkReader,
{
    if ensure_role(user, "parser", None).is_err() {
        return Err(ServiceError::Unauthorized);
    }

    match repo.list_benchmarks(BenchmarkListQuery::new(user.hub_id)) {
        Ok((_total, benchmarks)) => Ok(benchmarks),
        Err(e) => {
            error!("Failed to list benchmarks: {e}");
            Err(ServiceError::Internal)
        }
    }
}

/// Core business logic for rendering a single benchmark page.
///
/// Ensures the user has the `parser` role, verifies that the benchmark belongs
/// to the user's hub and gathers crawlers with their products and similarity
/// distances. Repository errors are mapped to [`ServiceError`] variants so the
/// HTTP route remains a thin wrapper.
pub fn show_benchmark<R>(
    repo: &R,
    user: &AuthenticatedUser,
    benchmark_id: i32,
) -> ServiceResult<(Benchmark, Vec<(Crawler, Paginated<Product>)>, HashMap<i32, f32>)>
where
    R: BenchmarkReader + CrawlerReader + ProductReader,
{
    if ensure_role(user, "parser", None).is_err() {
        return Err(ServiceError::Unauthorized);
    }

    let benchmark = match repo.get_benchmark_by_id(benchmark_id) {
        Ok(Some(benchmark)) if benchmark.hub_id == user.hub_id => benchmark,
        Ok(_) => return Err(ServiceError::NotFound),
        Err(e) => {
            error!("Failed to get benchmark: {e}");
            return Err(ServiceError::Internal);
        }
    };

    let crawlers = match repo.list_crawlers(user.hub_id) {
        Ok(crawlers) => crawlers,
        Err(e) => {
            error!("Failed to list crawlers: {e}");
            return Err(ServiceError::Internal);
        }
    };

    let mut products: Vec<(Crawler, Paginated<Product>)> = vec![];
    for crawler in crawlers {
        let crawler_products = match repo.list_products(
            ProductListQuery::default()
                .benchmark(benchmark_id)
                .crawler(crawler.id)
                .paginate(1, DEFAULT_ITEMS_PER_PAGE),
        ) {
            Ok((total, items)) => {
                Paginated::new(items, 1, total.div_ceil(DEFAULT_ITEMS_PER_PAGE))
            }
            Err(e) => {
                error!("Failed to list products: {e}");
                return Err(ServiceError::Internal);
            }
        };
        products.push((crawler, crawler_products));
    }

    let distances = match repo.list_distances(benchmark_id) {
        Ok(distances) => distances,
        Err(e) => {
            error!("Failed to list distances: {e}");
            return Err(ServiceError::Internal);
        }
    };

    Ok((benchmark, products, distances))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::repository::test::TestRepository;
    use chrono::NaiveDateTime;
    use serde_json::Value;

    fn sample_user() -> AuthenticatedUser {
        AuthenticatedUser {
            sub: "1".into(),
            email: "test@example.com".into(),
            hub_id: 1,
            name: "Test".into(),
            roles: vec!["parser".into()],
            exp: 0,
        }
    }

    fn sample_crawler() -> Crawler {
        Crawler {
            id: 1,
            hub_id: 1,
            name: "crawler".into(),
            url: "http://example.com".into(),
            selector: "body".into(),
            processing: false,
            updated_at: NaiveDateTime::from_timestamp(0, 0),
            num_products: 0,
        }
    }

    fn sample_product() -> Product {
        Product {
            id: 1,
            crawler_id: 1,
            name: "product".into(),
            sku: "SKU1".into(),
            category: Some("cat".into()),
            units: Some("pcs".into()),
            price: 1.0,
            amount: None,
            description: None,
            url: "http://example.com".into(),
            created_at: NaiveDateTime::from_timestamp(0, 0),
            updated_at: NaiveDateTime::from_timestamp(0, 0),
            embedding: None,
        }
    }

    fn sample_benchmark() -> Benchmark {
        Benchmark {
            id: 1,
            hub_id: 1,
            name: "benchmark".into(),
            sku: "SKU1".into(),
            category: "cat".into(),
            units: "pcs".into(),
            price: 1.0,
            amount: 1.0,
            description: "desc".into(),
            created_at: NaiveDateTime::from_timestamp(0, 0),
            updated_at: NaiveDateTime::from_timestamp(0, 0),
            embedding: None,
            processing: false,
            num_products: 0,
        }
    }

    #[test]
    fn returns_benchmarks_for_authorized_user() {
        let repo = TestRepository::new(vec![], vec![], vec![sample_benchmark()]);
        let user = sample_user();

        let benchmarks = show_benchmarks(&repo, &user).unwrap();
        assert_eq!(benchmarks.len(), 1);
    }

    #[test]
    fn returns_benchmark_details_for_authorized_user() {
        let repo = TestRepository::new(
            vec![sample_crawler()],
            vec![sample_product()],
            vec![sample_benchmark()],
        );
        let user = sample_user();

        let (benchmark, crawler_products, distances) =
            show_benchmark(&repo, &user, 1).unwrap();

        assert_eq!(benchmark.id, 1);
        assert_eq!(crawler_products.len(), 1);
        let (crawler, paginated) = &crawler_products[0];
        assert_eq!(crawler.id, 1);
        let value: Value = serde_json::to_value(paginated).unwrap();
        assert_eq!(value["page"], 1);
        assert_eq!(value["items"].as_array().unwrap().len(), 1);
        assert!(distances.is_empty());
    }
}
