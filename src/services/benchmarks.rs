use std::collections::HashMap;

use pushkind_common::domain::auth::AuthenticatedUser;
use pushkind_common::pagination::{DEFAULT_ITEMS_PER_PAGE, Paginated};
use pushkind_common::routes::check_role;
use validator::Validate;

use crate::domain::zmq::{CrawlerSelector, ZMQCrawlerMessage};
use crate::domain::{benchmark::Benchmark, crawler::Crawler, product::Product};
use crate::forms::benchmarks::{
    AddBenchmarkForm, AssociateForm, UnassociateForm, UploadBenchmarksForm,
};
use crate::repository::{
    BenchmarkListQuery, BenchmarkReader, BenchmarkWriter, CrawlerReader, ProductListQuery,
    ProductReader,
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
    if !check_role("parser", &user.roles) {
        return Err(ServiceError::Unauthorized);
    }

    match repo.list_benchmarks(BenchmarkListQuery::new(user.hub_id)) {
        Ok((_total, benchmarks)) => Ok(benchmarks),
        Err(e) => {
            log::error!("Failed to list benchmarks: {e}");
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
#[allow(clippy::type_complexity)]
pub fn show_benchmark<R>(
    repo: &R,
    user: &AuthenticatedUser,
    benchmark_id: i32,
) -> ServiceResult<(
    Benchmark,
    Vec<(Crawler, Paginated<Product>)>,
    HashMap<i32, f32>,
)>
where
    R: BenchmarkReader + CrawlerReader + ProductReader,
{
    if !check_role("parser", &user.roles) {
        return Err(ServiceError::Unauthorized);
    }

    let benchmark = match repo.get_benchmark_by_id(benchmark_id, user.hub_id) {
        Ok(Some(benchmark)) => benchmark,
        Ok(None) => return Err(ServiceError::NotFound),
        Err(e) => {
            log::error!("Failed to get benchmark: {e}");
            return Err(ServiceError::Internal);
        }
    };

    let crawlers = match repo.list_crawlers(user.hub_id) {
        Ok(crawlers) => crawlers,
        Err(e) => {
            log::error!("Failed to list crawlers: {e}");
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
            Ok((total, items)) => Paginated::new(items, 1, total.div_ceil(DEFAULT_ITEMS_PER_PAGE)),
            Err(e) => {
                log::error!("Failed to list products: {e}");
                return Err(ServiceError::Internal);
            }
        };
        products.push((crawler, crawler_products));
    }

    let distances = match repo.list_distances(benchmark_id) {
        Ok(distances) => distances,
        Err(e) => {
            log::error!("Failed to list distances: {e}");
            return Err(ServiceError::Internal);
        }
    };

    Ok((benchmark, products, distances))
}

/// Adds a new benchmark from the supplied form.
///
/// Validates the `parser` role and the form itself before persisting the
/// benchmark. Returns `Ok(true)` if the benchmark was created, `Ok(false)` if
/// validation failed or the repository returned an error.
pub fn add_benchmark<R>(
    repo: &R,
    user: &AuthenticatedUser,
    form: AddBenchmarkForm,
) -> ServiceResult<bool>
where
    R: BenchmarkWriter,
{
    if !check_role("parser", &user.roles) {
        return Err(ServiceError::Unauthorized);
    }

    if let Err(e) = form.validate() {
        log::error!("Failed to validate form: {e}");
        return Ok(false);
    }

    let new_benchmark = form.into_new_benchmark(user.hub_id);

    match repo.create_benchmark(&[new_benchmark]) {
        Ok(_) => Ok(true),
        Err(e) => {
            log::error!("Failed to add a benchmark: {e}");
            Ok(false)
        }
    }
}

/// Parses and uploads multiple benchmarks.
///
/// Returns `Ok(true)` if benchmarks were created successfully, `Ok(false)` if
/// parsing failed or the repository returned an error.
pub fn upload_benchmarks<R>(
    repo: &R,
    user: &AuthenticatedUser,
    form: &mut UploadBenchmarksForm,
) -> ServiceResult<bool>
where
    R: BenchmarkWriter,
{
    if !check_role("parser", &user.roles) {
        return Err(ServiceError::Unauthorized);
    }

    let benchmarks = match form.parse(user.hub_id) {
        Ok(benchmarks) => benchmarks,
        Err(e) => {
            log::error!("Failed to parse benchmarks: {e}");
            return Ok(false);
        }
    };

    match repo.create_benchmark(&benchmarks) {
        Ok(_) => Ok(true),
        Err(e) => {
            log::error!("Failed to add benchmarks: {e}");
            Ok(false)
        }
    }
}

/// Sends a ZMQ message to match the specified benchmark.
///
/// Returns `Ok(true)` if the message was sent successfully, `Ok(false)` if
/// sending failed.
pub async fn match_benchmark<R, F>(
    repo: &R,
    user: &AuthenticatedUser,
    benchmark_id: i32,
    send: F,
) -> ServiceResult<bool>
where
    R: BenchmarkReader,
    F: AsyncFn(&ZMQCrawlerMessage) -> Result<(), ()>,
{
    if !check_role("parser", &user.roles) {
        return Err(ServiceError::Unauthorized);
    }

    let benchmark = match repo.get_benchmark_by_id(benchmark_id, user.hub_id) {
        Ok(Some(benchmark)) => benchmark,
        Ok(None) => return Err(ServiceError::NotFound),
        Err(e) => {
            log::error!("Failed to get benchmark: {e}");
            return Err(ServiceError::Internal);
        }
    };

    let message = ZMQCrawlerMessage::Benchmark(benchmark.id);
    match send(&message).await {
        Ok(_) => Ok(true),
        Err(_) => {
            log::error!("Failed to send ZMQ message");
            Ok(false)
        }
    }
}

/// Sends ZMQ messages to update prices for all products associated with a benchmark.
///
/// Returns a list of crawler selectors and whether sending the message for that
/// crawler succeeded.
pub async fn update_benchmark_prices<R, F>(
    repo: &R,
    user: &AuthenticatedUser,
    benchmark_id: i32,
    send: F,
) -> ServiceResult<Vec<(String, bool)>>
where
    R: BenchmarkReader + CrawlerReader + ProductReader,
    F: AsyncFn(&ZMQCrawlerMessage) -> Result<(), ()>,
{
    if !check_role("parser", &user.roles) {
        return Err(ServiceError::Unauthorized);
    }

    let benchmark = match repo.get_benchmark_by_id(benchmark_id, user.hub_id) {
        Ok(Some(benchmark)) => benchmark,
        Ok(None) => return Err(ServiceError::NotFound),
        Err(e) => {
            log::error!("Failed to get benchmark: {e}");
            return Err(ServiceError::Internal);
        }
    };

    let crawlers = match repo.list_crawlers(user.hub_id) {
        Ok(crawlers) => crawlers,
        Err(e) => {
            log::error!("Failed to list crawlers: {e}");
            return Err(ServiceError::Internal);
        }
    };

    let mut results = Vec::new();
    for crawler in crawlers {
        let products = match repo.list_products(
            ProductListQuery::default()
                .benchmark(benchmark.id)
                .crawler(crawler.id),
        ) {
            Ok((_total, products)) => products,
            Err(e) => {
                log::error!("Failed to list products: {e}");
                return Err(ServiceError::Internal);
            }
        };

        if products.is_empty() {
            continue;
        }

        let urls = products.into_iter().map(|p| p.url).collect();
        let message = ZMQCrawlerMessage::Crawler(CrawlerSelector::SelectorProducts((
            crawler.selector.clone(),
            urls,
        )));
        let sent = send(&message).await.is_ok();
        if !sent {
            log::error!("Failed to send ZMQ message");
        }
        results.push((crawler.selector, sent));
    }

    Ok(results)
}

/// Removes an association between a benchmark and a product.
///
/// Returns `Ok(true)` if the association was removed, `Ok(false)` if the
/// repository returned an error or entities were not found.
pub fn delete_benchmark_product<R>(
    repo: &R,
    user: &AuthenticatedUser,
    form: UnassociateForm,
) -> ServiceResult<bool>
where
    R: BenchmarkReader + ProductReader + CrawlerReader + BenchmarkWriter,
{
    if !check_role("parser", &user.roles) {
        return Err(ServiceError::Unauthorized);
    }

    let benchmark = match repo.get_benchmark_by_id(form.benchmark_id, user.hub_id) {
        Ok(Some(b)) => b,
        Ok(None) => return Err(ServiceError::NotFound),
        Err(e) => {
            log::error!("Failed to get benchmark: {e}");
            return Err(ServiceError::Internal);
        }
    };

    let product = match repo.get_product_by_id(form.product_id) {
        Ok(Some(p)) => p,
        Ok(None) => return Err(ServiceError::NotFound),
        Err(e) => {
            log::error!("Failed to get product: {e}");
            return Err(ServiceError::Internal);
        }
    };

    match repo.get_crawler_by_id(product.crawler_id, user.hub_id) {
        Ok(Some(crawler)) => crawler,
        Ok(None) => return Err(ServiceError::NotFound),
        Err(e) => {
            log::error!("Failed to get crawler: {e}");
            return Err(ServiceError::Internal);
        }
    };

    match repo.remove_benchmark_association(benchmark.id, product.id) {
        Ok(_) => Ok(true),
        Err(e) => {
            log::error!("Failed to delete association: {e}");
            Ok(false)
        }
    }
}

/// Creates an association between a benchmark and a product.
///
/// Returns `Ok(true)` if the association was created, `Ok(false)` if the
/// repository returned an error or entities were not found.
pub fn create_benchmark_product<R>(
    repo: &R,
    user: &AuthenticatedUser,
    form: AssociateForm,
) -> ServiceResult<bool>
where
    R: BenchmarkReader + ProductReader + CrawlerReader + BenchmarkWriter,
{
    if !check_role("parser", &user.roles) {
        return Err(ServiceError::Unauthorized);
    }

    let benchmark = match repo.get_benchmark_by_id(form.benchmark_id, user.hub_id) {
        Ok(Some(b)) => b,
        Ok(None) => return Err(ServiceError::NotFound),
        Err(e) => {
            log::error!("Failed to get benchmark: {e}");
            return Err(ServiceError::Internal);
        }
    };

    let product = match repo.get_product_by_id(form.product_id) {
        Ok(Some(p)) => p,
        Ok(None) => return Err(ServiceError::NotFound),
        Err(e) => {
            log::error!("Failed to get product: {e}");
            return Err(ServiceError::Internal);
        }
    };

    match repo.get_crawler_by_id(product.crawler_id, user.hub_id) {
        Ok(Some(crawler)) => crawler,
        Ok(None) => return Err(ServiceError::NotFound),
        Err(e) => {
            log::error!("Failed to get crawler: {e}");
            return Err(ServiceError::Internal);
        }
    };

    match repo.set_benchmark_association(benchmark.id, product.id, 1.0) {
        Ok(_) => Ok(true),
        Err(e) => {
            log::error!("Failed to create benchmark association: {e}");
            Ok(false)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::repository::test::TestRepository;
    use chrono::DateTime;
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
            updated_at: DateTime::from_timestamp(0, 0).unwrap().naive_utc(),
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
            created_at: DateTime::from_timestamp(0, 0).unwrap().naive_utc(),
            updated_at: DateTime::from_timestamp(0, 0).unwrap().naive_utc(),
            embedding: None,
            images: vec![],
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
            created_at: DateTime::from_timestamp(0, 0).unwrap().naive_utc(),
            updated_at: DateTime::from_timestamp(0, 0).unwrap().naive_utc(),
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

        let (benchmark, crawler_products, distances) = show_benchmark(&repo, &user, 1).unwrap();

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
