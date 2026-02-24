use std::collections::HashMap;

use pushkind_common::domain::auth::AuthenticatedUser;
use pushkind_common::pagination::{DEFAULT_ITEMS_PER_PAGE, Paginated};
use pushkind_common::routes::check_role;
use pushkind_common::zmq::ZmqSenderExt;

use crate::SERVICE_ACCESS_ROLE;
use crate::domain::types::{BenchmarkId, HubId, SimilarityDistance};
use crate::domain::zmq::{CrawlerSelector, ZMQCrawlerMessage};
use crate::domain::{
    benchmark::Benchmark, benchmark::NewBenchmark, crawler::Crawler, product::Product,
};
use crate::forms::benchmarks::{
    AddBenchmarkForm, AddBenchmarkFormPayload, AssociateForm, AssociateFormPayload,
    UnassociateForm, UnassociateFormPayload, UploadBenchmarksForm, UploadBenchmarksFormPayload,
};
use crate::forms::import_export::{UploadImportForm, UploadMode, UploadTarget, parse_upload};
use crate::repository::{
    BenchmarkListQuery, BenchmarkReader, BenchmarkWriter, CrawlerReader, ProductListQuery,
    ProductReader,
};
use crate::services::import_export::{
    DownloadFile, DownloadFormat, UploadReport, render_download_file,
};

use super::{ServiceError, ServiceResult};

fn parse_f64(value: &str, field: &str) -> Result<f64, String> {
    value
        .parse::<f64>()
        .map_err(|_| format!("Invalid numeric value for {field}"))
}

fn build_benchmark_from_row(
    row: &std::collections::HashMap<String, String>,
    hub_id: HubId,
) -> Result<NewBenchmark, String> {
    let name = row.get("name").cloned().unwrap_or_default();
    let sku = row.get("sku").cloned().unwrap_or_default();
    let category = row.get("category").cloned().unwrap_or_default();
    let units = row.get("units").cloned().unwrap_or_default();
    let price = parse_f64(
        row.get("price").map(String::as_str).unwrap_or_default(),
        "price",
    )?;
    let amount = parse_f64(
        row.get("amount").map(String::as_str).unwrap_or_default(),
        "amount",
    )?;
    let description = row.get("description").cloned().unwrap_or_default();

    let payload = AddBenchmarkFormPayload::try_from(AddBenchmarkForm {
        name,
        sku,
        category,
        units,
        price,
        amount,
        description,
    })
    .map_err(|err| err.to_string())?;

    Ok(payload.into_new_benchmark(hub_id))
}

/// Core business logic for rendering the benchmarks page.
///
/// Validates the `parser` role and fetches paginated benchmarks for the
/// user's hub. Repository errors are translated into [`ServiceError`] so the
/// HTTP route can remain a thin wrapper.
pub fn show_benchmarks<R>(user: &AuthenticatedUser, repo: &R) -> ServiceResult<Vec<Benchmark>>
where
    R: BenchmarkReader,
{
    if !check_role(SERVICE_ACCESS_ROLE, &user.roles) {
        return Err(ServiceError::Unauthorized);
    }

    let hub_id = match HubId::new(user.hub_id) {
        Ok(hub_id) => hub_id,
        Err(e) => {
            log::error!("Invalid hub id in user context: {e}");
            return Err(ServiceError::Internal);
        }
    };

    match repo.list_benchmarks(BenchmarkListQuery::new(hub_id)) {
        Ok((_total, benchmarks)) => Ok(benchmarks),
        Err(e) => {
            log::error!("Failed to list benchmarks: {e}");
            Err(ServiceError::Internal)
        }
    }
}

pub fn download_benchmarks<R>(
    format: &str,
    user: &AuthenticatedUser,
    repo: &R,
) -> ServiceResult<DownloadFile>
where
    R: BenchmarkReader,
{
    if !check_role(SERVICE_ACCESS_ROLE, &user.roles) {
        return Err(ServiceError::Unauthorized);
    }

    let hub_id = HubId::new(user.hub_id).map_err(|_| ServiceError::Internal)?;
    let format =
        DownloadFormat::try_from(format).map_err(|err| ServiceError::Form(err.to_string()))?;
    let benchmarks = repo
        .list_benchmarks(BenchmarkListQuery::new(hub_id))
        .map_err(|_| ServiceError::Internal)?
        .1;

    let rows = benchmarks
        .into_iter()
        .map(|b| {
            vec![
                b.sku.as_str().to_string(),
                b.name.as_str().to_string(),
                b.category.as_str().to_string(),
                b.units.as_str().to_string(),
                b.price.get().to_string(),
                b.amount.get().to_string(),
                b.description.as_str().to_string(),
            ]
        })
        .collect::<Vec<_>>();

    render_download_file(
        "benchmarks",
        format,
        &[
            "sku",
            "name",
            "category",
            "units",
            "price",
            "amount",
            "description",
        ],
        &rows,
    )
    .map_err(|err| ServiceError::Form(err.to_string()))
}

/// Core business logic for rendering a single benchmark page.
///
/// Ensures the user has the `parser` role, verifies that the benchmark belongs
/// to the user's hub and gathers crawlers with their products and similarity
/// distances. Repository errors are mapped to [`ServiceError`] variants so the
/// HTTP route remains a thin wrapper.
#[allow(clippy::type_complexity)]
pub fn show_benchmark<R>(
    benchmark_id: i32,
    user: &AuthenticatedUser,
    repo: &R,
) -> ServiceResult<(
    Benchmark,
    Vec<(Crawler, Paginated<Product>)>,
    HashMap<i32, f32>,
)>
where
    R: BenchmarkReader + CrawlerReader + ProductReader,
{
    if !check_role(SERVICE_ACCESS_ROLE, &user.roles) {
        return Err(ServiceError::Unauthorized);
    }

    let hub_id = match HubId::new(user.hub_id) {
        Ok(hub_id) => hub_id,
        Err(e) => {
            log::error!("Invalid hub id in user context: {e}");
            return Err(ServiceError::Internal);
        }
    };

    let benchmark_id = match BenchmarkId::new(benchmark_id) {
        Ok(benchmark_id) => benchmark_id,
        Err(_) => return Err(ServiceError::NotFound),
    };

    let benchmark = match repo.get_benchmark_by_id(benchmark_id, hub_id) {
        Ok(Some(benchmark)) => benchmark,
        Ok(None) => return Err(ServiceError::NotFound),
        Err(e) => {
            log::error!("Failed to get benchmark: {e}");
            return Err(ServiceError::Internal);
        }
    };

    let crawlers = match repo.list_crawlers(hub_id) {
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
        Ok(distances) => distances
            .into_iter()
            .map(|(product_id, distance)| (product_id.get(), distance.get()))
            .collect(),
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
/// benchmark. Returns `Ok(true)` if the benchmark was created,
/// `Err(ServiceError::Form(_))` if form validation failed, and `Ok(false)` if
/// the repository returned an error.
pub fn add_benchmark<R>(
    form: AddBenchmarkForm,
    user: &AuthenticatedUser,
    repo: &R,
) -> ServiceResult<bool>
where
    R: BenchmarkWriter,
{
    if !check_role(SERVICE_ACCESS_ROLE, &user.roles) {
        return Err(ServiceError::Unauthorized);
    }

    let payload: AddBenchmarkFormPayload = match form.try_into() {
        Ok(payload) => payload,
        Err(e) => {
            log::error!("Failed to parse add benchmark form: {e}");
            return Err(ServiceError::Form(e.to_string()));
        }
    };

    let hub_id = match HubId::new(user.hub_id) {
        Ok(hub_id) => hub_id,
        Err(e) => {
            log::error!("Invalid hub id in user context: {e}");
            return Ok(false);
        }
    };

    let new_benchmark = payload.into_new_benchmark(hub_id);

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
/// Returns `Ok(true)` if benchmarks were created successfully,
/// `Err(ServiceError::Form(_))` if parsing failed, and `Ok(false)` if the
/// repository returned an error.
pub fn upload_benchmarks<R>(
    form: &mut UploadBenchmarksForm,
    user: &AuthenticatedUser,
    repo: &R,
) -> ServiceResult<bool>
where
    R: BenchmarkWriter,
{
    if !check_role(SERVICE_ACCESS_ROLE, &user.roles) {
        return Err(ServiceError::Unauthorized);
    }

    let payload: UploadBenchmarksFormPayload = match form.try_into() {
        Ok(payload) => payload,
        Err(e) => {
            log::error!("Failed to parse upload benchmarks form: {e}");
            return Err(ServiceError::Form(e.to_string()));
        }
    };

    let hub_id = match HubId::new(user.hub_id) {
        Ok(hub_id) => hub_id,
        Err(e) => {
            log::error!("Invalid hub id in user context: {e}");
            return Ok(false);
        }
    };

    let benchmarks = payload.into_new_benchmarks(hub_id);

    match repo.create_benchmark(&benchmarks) {
        Ok(_) => Ok(true),
        Err(e) => {
            log::error!("Failed to add benchmarks: {e}");
            Ok(false)
        }
    }
}

/// Upload benchmarks using format/mode-aware import parser and SKU upsert semantics.
pub fn upload_benchmarks_import<R>(
    form: &mut UploadImportForm,
    user: &AuthenticatedUser,
    repo: &R,
) -> ServiceResult<UploadReport>
where
    R: BenchmarkReader + BenchmarkWriter,
{
    if !check_role(SERVICE_ACCESS_ROLE, &user.roles) {
        return Err(ServiceError::Unauthorized);
    }

    let hub_id = HubId::new(user.hub_id).map_err(|_| ServiceError::Internal)?;
    let parsed = parse_upload(form, UploadTarget::Benchmarks)
        .map_err(|err| ServiceError::Form(err.to_string()))?;
    apply_benchmark_upload(parsed, hub_id, repo)
}

fn apply_benchmark_upload<R>(
    parsed: crate::forms::import_export::ParsedUpload,
    hub_id: HubId,
    repo: &R,
) -> ServiceResult<UploadReport>
where
    R: BenchmarkReader + BenchmarkWriter,
{
    let mut report = UploadReport::with_total(parsed.rows.len());
    let mut seen_skus = std::collections::HashSet::new();

    for row in parsed.rows {
        let raw_sku = row.values.get("sku").cloned().unwrap_or_default();
        let sku_value = raw_sku.trim().to_string();
        if sku_value.is_empty() {
            report.push_error(row.row_number, None, "Missing sku");
            continue;
        }

        if !seen_skus.insert(sku_value.clone()) {
            report.push_error(
                row.row_number,
                Some(sku_value),
                "Duplicate sku in uploaded file",
            );
            continue;
        }

        let sku = match crate::domain::types::BenchmarkSku::new(sku_value.clone()) {
            Ok(sku) => sku,
            Err(err) => {
                report.push_error(row.row_number, Some(sku_value), err.to_string());
                continue;
            }
        };

        let existing = match repo.list_benchmarks_by_hub_and_sku(hub_id, &sku) {
            Ok(items) => items,
            Err(err) => {
                log::error!("Failed to lookup benchmark by sku: {err}");
                return Err(ServiceError::Internal);
            }
        };

        if existing.len() > 1 {
            report.push_error(
                row.row_number,
                Some(sku_value),
                "Multiple existing benchmarks found for sku",
            );
            continue;
        }

        let mut merged = row.values.clone();
        if parsed.mode == UploadMode::Partial
            && let Some(current) = existing.first()
        {
            merged
                .entry("name".to_string())
                .or_insert_with(|| current.name.as_str().to_string());
            merged
                .entry("category".to_string())
                .or_insert_with(|| current.category.as_str().to_string());
            merged
                .entry("units".to_string())
                .or_insert_with(|| current.units.as_str().to_string());
            merged
                .entry("price".to_string())
                .or_insert_with(|| current.price.get().to_string());
            merged
                .entry("amount".to_string())
                .or_insert_with(|| current.amount.get().to_string());
            merged
                .entry("description".to_string())
                .or_insert_with(|| current.description.as_str().to_string());
        }

        let new_benchmark = match build_benchmark_from_row(&merged, hub_id) {
            Ok(item) => item,
            Err(err) => {
                report.push_error(row.row_number, Some(sku_value), err);
                continue;
            }
        };

        if let Some(current) = existing.first() {
            match repo.update_benchmark(current.id, &new_benchmark) {
                Ok(_) => report.updated += 1,
                Err(err) => {
                    log::error!("Failed to update benchmark: {err}");
                    report.push_error(
                        row.row_number,
                        Some(sku_value),
                        "Failed to update benchmark",
                    );
                }
            }
            continue;
        }

        if parsed.mode == UploadMode::Partial {
            let required = [
                "name",
                "category",
                "units",
                "price",
                "amount",
                "description",
            ];
            let has_all_required = required.iter().all(|field| {
                merged
                    .get(*field)
                    .map(|value| !value.trim().is_empty())
                    .unwrap_or(false)
            });
            if !has_all_required {
                report.push_error(
                    row.row_number,
                    Some(sku_value),
                    "Partial mode create requires all required fields",
                );
                continue;
            }
        }

        match repo.create_benchmark(&[new_benchmark]) {
            Ok(_) => report.created += 1,
            Err(err) => {
                log::error!("Failed to create benchmark: {err}");
                report.push_error(
                    row.row_number,
                    Some(sku_value),
                    "Failed to create benchmark",
                );
            }
        }
    }

    Ok(report)
}

/// Sends a ZMQ message to match the specified benchmark.
///
/// Returns `Ok(true)` if the message was sent successfully, `Ok(false)` if
/// sending failed.
pub async fn match_benchmark<R, S>(
    benchmark_id: i32,
    user: &AuthenticatedUser,
    repo: &R,
    sender: &S,
) -> ServiceResult<bool>
where
    R: BenchmarkReader,
    S: ZmqSenderExt + ?Sized,
{
    if !check_role(SERVICE_ACCESS_ROLE, &user.roles) {
        return Err(ServiceError::Unauthorized);
    }

    let hub_id = match HubId::new(user.hub_id) {
        Ok(hub_id) => hub_id,
        Err(e) => {
            log::error!("Invalid hub id in user context: {e}");
            return Err(ServiceError::Internal);
        }
    };

    let benchmark_id = match BenchmarkId::new(benchmark_id) {
        Ok(benchmark_id) => benchmark_id,
        Err(_) => return Err(ServiceError::NotFound),
    };

    let benchmark = match repo.get_benchmark_by_id(benchmark_id, hub_id) {
        Ok(Some(benchmark)) => benchmark,
        Ok(None) => return Err(ServiceError::NotFound),
        Err(e) => {
            log::error!("Failed to get benchmark: {e}");
            return Err(ServiceError::Internal);
        }
    };

    let message = ZMQCrawlerMessage::Benchmark(benchmark.id);
    match sender.send_json(&message).await {
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
pub async fn update_benchmark_prices<R, S>(
    benchmark_id: i32,
    user: &AuthenticatedUser,
    repo: &R,
    sender: &S,
) -> ServiceResult<Vec<(String, bool)>>
where
    R: BenchmarkReader + CrawlerReader + ProductReader,
    S: ZmqSenderExt + ?Sized,
{
    if !check_role(SERVICE_ACCESS_ROLE, &user.roles) {
        return Err(ServiceError::Unauthorized);
    }

    let hub_id = match HubId::new(user.hub_id) {
        Ok(hub_id) => hub_id,
        Err(e) => {
            log::error!("Invalid hub id in user context: {e}");
            return Err(ServiceError::Internal);
        }
    };

    let benchmark_id = match BenchmarkId::new(benchmark_id) {
        Ok(benchmark_id) => benchmark_id,
        Err(_) => return Err(ServiceError::NotFound),
    };

    let benchmark = match repo.get_benchmark_by_id(benchmark_id, hub_id) {
        Ok(Some(benchmark)) => benchmark,
        Ok(None) => return Err(ServiceError::NotFound),
        Err(e) => {
            log::error!("Failed to get benchmark: {e}");
            return Err(ServiceError::Internal);
        }
    };

    let crawlers = match repo.list_crawlers(hub_id) {
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

        let urls = products
            .into_iter()
            .filter_map(|p| p.url)
            .collect::<Vec<_>>();
        if urls.is_empty() {
            continue;
        }
        let message = ZMQCrawlerMessage::Crawler(CrawlerSelector::SelectorProducts((
            crawler.selector.clone(),
            urls,
        )));
        let sent = sender.send_json(&message).await.is_ok();
        if !sent {
            log::error!("Failed to send ZMQ message");
        }
        results.push((crawler.selector.into_inner(), sent));
    }

    Ok(results)
}

/// Removes an association between a benchmark and a product.
///
/// Returns `Ok(true)` if the association was removed,
/// `Err(ServiceError::Form(_))` if form validation failed, and `Ok(false)` if
/// the repository returned an error or entities were not found.
pub fn delete_benchmark_product<R>(
    form: UnassociateForm,
    user: &AuthenticatedUser,
    repo: &R,
) -> ServiceResult<bool>
where
    R: BenchmarkReader + ProductReader + CrawlerReader + BenchmarkWriter,
{
    if !check_role(SERVICE_ACCESS_ROLE, &user.roles) {
        return Err(ServiceError::Unauthorized);
    }

    let payload: UnassociateFormPayload = match form.try_into() {
        Ok(payload) => payload,
        Err(e) => {
            log::error!("Failed to parse unassociate form: {e}");
            return Err(ServiceError::Form(e.to_string()));
        }
    };

    let hub_id = match HubId::new(user.hub_id) {
        Ok(hub_id) => hub_id,
        Err(e) => {
            log::error!("Invalid hub id in user context: {e}");
            return Err(ServiceError::Internal);
        }
    };

    let benchmark = match repo.get_benchmark_by_id(payload.benchmark_id, hub_id) {
        Ok(Some(b)) => b,
        Ok(None) => return Err(ServiceError::NotFound),
        Err(e) => {
            log::error!("Failed to get benchmark: {e}");
            return Err(ServiceError::Internal);
        }
    };

    let product = match repo.get_product_by_id(payload.product_id) {
        Ok(Some(p)) => p,
        Ok(None) => return Err(ServiceError::NotFound),
        Err(e) => {
            log::error!("Failed to get product: {e}");
            return Err(ServiceError::Internal);
        }
    };

    match repo.get_crawler_by_id(product.crawler_id, hub_id) {
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
/// Returns `Ok(true)` if the association was created,
/// `Err(ServiceError::Form(_))` if form validation failed, and `Ok(false)` if
/// the repository returned an error or entities were not found.
pub fn create_benchmark_product<R>(
    form: AssociateForm,
    user: &AuthenticatedUser,
    repo: &R,
) -> ServiceResult<bool>
where
    R: BenchmarkReader + ProductReader + CrawlerReader + BenchmarkWriter,
{
    if !check_role(SERVICE_ACCESS_ROLE, &user.roles) {
        return Err(ServiceError::Unauthorized);
    }

    let payload: AssociateFormPayload = match form.try_into() {
        Ok(payload) => payload,
        Err(e) => {
            log::error!("Failed to parse associate form: {e}");
            return Err(ServiceError::Form(e.to_string()));
        }
    };

    let hub_id = match HubId::new(user.hub_id) {
        Ok(hub_id) => hub_id,
        Err(e) => {
            log::error!("Invalid hub id in user context: {e}");
            return Err(ServiceError::Internal);
        }
    };

    let benchmark = match repo.get_benchmark_by_id(payload.benchmark_id, hub_id) {
        Ok(Some(b)) => b,
        Ok(None) => return Err(ServiceError::NotFound),
        Err(e) => {
            log::error!("Failed to get benchmark: {e}");
            return Err(ServiceError::Internal);
        }
    };

    let product = match repo.get_product_by_id(payload.product_id) {
        Ok(Some(p)) => p,
        Ok(None) => return Err(ServiceError::NotFound),
        Err(e) => {
            log::error!("Failed to get product: {e}");
            return Err(ServiceError::Internal);
        }
    };

    match repo.get_crawler_by_id(product.crawler_id, hub_id) {
        Ok(Some(crawler)) => crawler,
        Ok(None) => return Err(ServiceError::NotFound),
        Err(e) => {
            log::error!("Failed to get crawler: {e}");
            return Err(ServiceError::Internal);
        }
    };

    let distance = match SimilarityDistance::new(1.0) {
        Ok(distance) => distance,
        Err(e) => {
            log::error!("Invalid default similarity distance: {e}");
            return Ok(false);
        }
    };

    match repo.set_benchmark_association(benchmark.id, product.id, distance) {
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
    use crate::domain::types::{
        BenchmarkId, BenchmarkName, BenchmarkSku, CategoryAssignmentSource, CategoryName,
        CrawlerId, CrawlerName, CrawlerSelectorValue, CrawlerUrl, HubId, ProductAmount,
        ProductCount, ProductDescription, ProductId, ProductName, ProductPrice, ProductSku,
        ProductUnits, ProductUrl,
    };
    use crate::forms::import_export::{ParsedUpload, ParsedUploadRow, UploadFormat, UploadMode};
    use crate::repository::test::TestRepository;
    use chrono::DateTime;
    use pushkind_common::zmq::{SendFuture, ZmqSenderError, ZmqSenderTrait};
    use serde_json::Value;
    use std::collections::HashMap;

    fn sample_user() -> AuthenticatedUser {
        AuthenticatedUser {
            sub: "1".into(),
            email: "test@example.com".into(),
            hub_id: 1,
            name: "Test".into(),
            roles: vec![SERVICE_ACCESS_ROLE.into()],
            exp: 0,
        }
    }

    fn sample_crawler() -> Crawler {
        Crawler {
            id: CrawlerId::new(1).unwrap(),
            hub_id: HubId::new(1).unwrap(),
            name: CrawlerName::new("crawler").unwrap(),
            url: CrawlerUrl::new("http://example.com").unwrap(),
            selector: CrawlerSelectorValue::new("body").unwrap(),
            processing: false,
            updated_at: DateTime::from_timestamp(0, 0).unwrap().naive_utc(),
            num_products: ProductCount::new(0).unwrap(),
        }
    }

    fn sample_product() -> Product {
        Product {
            id: ProductId::new(1).unwrap(),
            crawler_id: CrawlerId::new(1).unwrap(),
            name: ProductName::new("product").unwrap(),
            sku: ProductSku::new("SKU1").unwrap(),
            category: Some(CategoryName::new("cat").unwrap()),
            associated_category: None,
            units: Some(ProductUnits::new("pcs").unwrap()),
            price: ProductPrice::new(1.0).unwrap(),
            amount: None,
            description: None,
            url: Some(ProductUrl::new("http://example.com").unwrap()),
            created_at: DateTime::from_timestamp(0, 0).unwrap().naive_utc(),
            updated_at: DateTime::from_timestamp(0, 0).unwrap().naive_utc(),
            embedding: None,
            category_id: None,
            category_assignment_source: CategoryAssignmentSource::Automatic,
            images: vec![],
        }
    }

    fn sample_benchmark() -> Benchmark {
        Benchmark {
            id: BenchmarkId::new(1).unwrap(),
            hub_id: HubId::new(1).unwrap(),
            name: BenchmarkName::new("benchmark").unwrap(),
            sku: BenchmarkSku::new("SKU1").unwrap(),
            category: CategoryName::new("cat").unwrap(),
            units: ProductUnits::new("pcs").unwrap(),
            price: ProductPrice::new(1.0).unwrap(),
            amount: ProductAmount::new(1.0).unwrap(),
            description: ProductDescription::new("desc").unwrap(),
            created_at: DateTime::from_timestamp(0, 0).unwrap().naive_utc(),
            updated_at: DateTime::from_timestamp(0, 0).unwrap().naive_utc(),
            embedding: None,
            processing: false,
            num_products: ProductCount::new(0).unwrap(),
        }
    }

    #[test]
    fn returns_benchmarks_for_authorized_user() {
        let repo = TestRepository::new(vec![], vec![], vec![sample_benchmark()]);
        let user = sample_user();

        let benchmarks = show_benchmarks(&user, &repo).unwrap();
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

        let (benchmark, crawler_products, distances) = show_benchmark(1, &user, &repo).unwrap();

        assert_eq!(benchmark.id, 1);
        assert_eq!(crawler_products.len(), 1);
        let (crawler, paginated) = &crawler_products[0];
        assert_eq!(crawler.id, 1);
        let value: Value = serde_json::to_value(paginated).unwrap();
        assert_eq!(value["page"], 1);
        assert_eq!(value["items"].as_array().unwrap().len(), 1);
        assert!(distances.is_empty());
    }

    #[test]
    fn add_benchmark_returns_form_error_for_invalid_form() {
        let repo = TestRepository::default();
        let user = sample_user();
        let form = AddBenchmarkForm {
            name: String::new(),
            sku: "SKU1".into(),
            category: "cat".into(),
            units: "pcs".into(),
            price: 1.0,
            amount: 1.0,
            description: "desc".into(),
        };

        let result = add_benchmark(form, &user, &repo);

        assert!(matches!(result, Err(ServiceError::Form(_))));
    }

    #[test]
    fn delete_benchmark_product_returns_form_error_for_invalid_form() {
        let repo = TestRepository::default();
        let user = sample_user();
        let form = UnassociateForm {
            benchmark_id: 0,
            product_id: 1,
        };

        let result = delete_benchmark_product(form, &user, &repo);

        assert!(matches!(result, Err(ServiceError::Form(_))));
    }

    #[test]
    fn create_benchmark_product_returns_form_error_for_invalid_form() {
        let repo = TestRepository::default();
        let user = sample_user();
        let form = AssociateForm {
            benchmark_id: 1,
            product_id: 0,
        };

        let result = create_benchmark_product(form, &user, &repo);

        assert!(matches!(result, Err(ServiceError::Form(_))));
    }

    #[test]
    fn benchmark_download_csv_contains_expected_headers() {
        let repo = TestRepository::new(vec![], vec![], vec![sample_benchmark()]);
        let user = sample_user();

        let file = download_benchmarks("csv", &user, &repo).unwrap();
        let body = String::from_utf8(file.bytes).unwrap();
        assert!(body.starts_with("sku,name,category,units,price,amount,description"));
    }

    #[test]
    fn benchmark_upload_reports_db_duplicate_sku_conflict() {
        let mut b1 = sample_benchmark();
        b1.id = BenchmarkId::new(1).unwrap();
        let mut b2 = sample_benchmark();
        b2.id = BenchmarkId::new(2).unwrap();

        let repo = TestRepository::new(vec![], vec![], vec![b1, b2]);
        let parsed = ParsedUpload {
            format: UploadFormat::Csv,
            mode: UploadMode::Partial,
            headers: vec!["sku".into(), "price".into()],
            rows: vec![ParsedUploadRow {
                row_number: 2,
                values: HashMap::from([
                    ("sku".into(), "SKU1".into()),
                    ("price".into(), "10.0".into()),
                ]),
            }],
        };

        let report = apply_benchmark_upload(parsed, HubId::new(1).unwrap(), &repo).unwrap();
        assert_eq!(report.skipped, 1);
        assert_eq!(report.errors.len(), 1);
    }

    struct NoopSender;

    impl ZmqSenderTrait for NoopSender {
        fn send_bytes<'a>(&'a self, _bytes: Vec<u8>) -> SendFuture<'a> {
            Box::pin(async { Ok(()) })
        }

        fn try_send_bytes(&self, _bytes: Vec<u8>) -> Result<(), ZmqSenderError> {
            Ok(())
        }

        fn send_multipart<'a>(&'a self, _frames: Vec<Vec<u8>>) -> SendFuture<'a> {
            Box::pin(async { Ok(()) })
        }
    }

    #[actix_web::test]
    async fn update_benchmark_prices_skips_crawlers_without_urls() {
        let mut p = sample_product();
        p.url = None;
        let repo = TestRepository::new(vec![sample_crawler()], vec![p], vec![sample_benchmark()]);
        let user = sample_user();
        let sender = NoopSender;

        let results = update_benchmark_prices(1, &user, &repo, &sender)
            .await
            .unwrap();
        assert!(results.is_empty());
    }
}
