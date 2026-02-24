use chrono::Utc;
use pushkind_common::domain::auth::AuthenticatedUser;
use pushkind_common::pagination::{DEFAULT_ITEMS_PER_PAGE, Paginated};
use pushkind_common::routes::check_role;
use pushkind_common::zmq::ZmqSenderExt;

use crate::SERVICE_ACCESS_ROLE;
use crate::domain::product::NewProduct;
use crate::domain::types::{CrawlerId, HubId};
use crate::domain::zmq::{CrawlerSelector, ZMQCrawlerMessage};
use crate::domain::{crawler::Crawler, product::Product};
use crate::forms::import_export::{UploadImportForm, UploadMode, UploadTarget, parse_upload};
use crate::repository::{CrawlerReader, ProductListQuery, ProductReader, ProductWriter};
use crate::services::import_export::{
    DownloadFile, DownloadFormat, UploadReport, render_download_file,
};

use super::{ServiceError, ServiceResult};

fn parse_required_f64(value: Option<&String>, field: &str) -> Result<f64, String> {
    value
        .map(String::as_str)
        .unwrap_or_default()
        .parse::<f64>()
        .map_err(|_| format!("Invalid numeric value for {field}"))
}

fn parse_optional_f64(value: Option<&String>, field: &str) -> Result<Option<f64>, String> {
    let Some(value) = value else {
        return Ok(None);
    };
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Ok(None);
    }
    trimmed
        .parse::<f64>()
        .map(Some)
        .map_err(|_| format!("Invalid numeric value for {field}"))
}

fn build_product_from_row(
    row: &std::collections::HashMap<String, String>,
    crawler_id: CrawlerId,
) -> Result<NewProduct, String> {
    let name = crate::domain::types::ProductName::new(row.get("name").cloned().unwrap_or_default())
        .map_err(|err| err.to_string())?;
    let sku = crate::domain::types::ProductSku::new(row.get("sku").cloned().unwrap_or_default())
        .map_err(|err| err.to_string())?;

    let price =
        crate::domain::types::ProductPrice::new(parse_required_f64(row.get("price"), "price")?)
            .map_err(|err| err.to_string())?;
    let amount = parse_optional_f64(row.get("amount"), "amount")?
        .map(crate::domain::types::ProductAmount::new)
        .transpose()
        .map_err(|err| err.to_string())?;

    let category = row
        .get("category")
        .filter(|value| !value.trim().is_empty())
        .cloned()
        .map(crate::domain::types::CategoryName::new)
        .transpose()
        .map_err(|err| err.to_string())?;
    let units = row
        .get("units")
        .filter(|value| !value.trim().is_empty())
        .cloned()
        .map(crate::domain::types::ProductUnits::new)
        .transpose()
        .map_err(|err| err.to_string())?;
    let description = row
        .get("description")
        .filter(|value| !value.trim().is_empty())
        .cloned()
        .map(crate::domain::types::ProductDescription::new)
        .transpose()
        .map_err(|err| err.to_string())?;
    let url = row
        .get("url")
        .filter(|value| !value.trim().is_empty())
        .cloned()
        .map(crate::domain::types::ProductUrl::new)
        .transpose()
        .map_err(|err| err.to_string())?;

    let _now = Utc::now().naive_utc();
    Ok(NewProduct {
        crawler_id,
        name,
        sku,
        category,
        units,
        price,
        amount,
        description,
        url,
        images: vec![],
    })
}

/// Core business logic for rendering the products page.
///
/// Validates that the user has the `parser` role, ensures the crawler belongs
/// to the user's hub, and fetches paginated products for the crawler.
/// Repository errors are converted into `ServiceError` variants so that the
/// HTTP route can remain a thin wrapper.
pub fn show_products<R>(
    crawler_id: i32,
    page: usize,
    user: &AuthenticatedUser,
    repo: &R,
) -> ServiceResult<(Crawler, Paginated<Product>)>
where
    R: CrawlerReader + ProductReader,
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

    let crawler_id = match CrawlerId::new(crawler_id) {
        Ok(crawler_id) => crawler_id,
        Err(_) => return Err(ServiceError::NotFound),
    };

    let crawler = match repo.get_crawler_by_id(crawler_id, hub_id) {
        Ok(Some(crawler)) => crawler,
        Ok(None) => return Err(ServiceError::NotFound),
        Err(e) => {
            log::error!("Failed to get crawler: {e}");
            return Err(ServiceError::Internal);
        }
    };

    let products = match repo.list_products(
        ProductListQuery::default()
            .crawler(crawler_id)
            .paginate(page, DEFAULT_ITEMS_PER_PAGE),
    ) {
        Ok((total, products)) => {
            Paginated::new(products, page, total.div_ceil(DEFAULT_ITEMS_PER_PAGE))
        }
        Err(e) => {
            log::error!("Failed to list products: {e}");
            return Err(ServiceError::Internal);
        }
    };

    Ok((crawler, products))
}

pub fn download_crawler_products<R>(
    crawler_id: i32,
    format: &str,
    user: &AuthenticatedUser,
    repo: &R,
) -> ServiceResult<DownloadFile>
where
    R: CrawlerReader + ProductReader,
{
    if !check_role(SERVICE_ACCESS_ROLE, &user.roles) {
        return Err(ServiceError::Unauthorized);
    }

    let hub_id = HubId::new(user.hub_id).map_err(|_| ServiceError::Internal)?;
    let crawler_id = CrawlerId::new(crawler_id).map_err(|_| ServiceError::NotFound)?;
    let format =
        DownloadFormat::try_from(format).map_err(|err| ServiceError::Form(err.to_string()))?;

    match repo.get_crawler_by_id(crawler_id, hub_id) {
        Ok(Some(_)) => {}
        Ok(None) => return Err(ServiceError::NotFound),
        Err(_) => return Err(ServiceError::Internal),
    }

    let products = repo
        .list_products(ProductListQuery::default().crawler(crawler_id))
        .map_err(|_| ServiceError::Internal)?
        .1;

    let rows = products
        .into_iter()
        .map(|p| {
            vec![
                p.sku.as_str().to_string(),
                p.name.as_str().to_string(),
                p.category
                    .as_ref()
                    .map(|v| v.as_str().to_string())
                    .unwrap_or_default(),
                p.units
                    .as_ref()
                    .map(|v| v.as_str().to_string())
                    .unwrap_or_default(),
                p.price.get().to_string(),
                p.amount.map(|v| v.get().to_string()).unwrap_or_default(),
                p.description
                    .as_ref()
                    .map(|v| v.as_str().to_string())
                    .unwrap_or_default(),
                p.url
                    .as_ref()
                    .map(|v| v.as_str().to_string())
                    .unwrap_or_default(),
            ]
        })
        .collect::<Vec<_>>();

    render_download_file(
        &format!("crawler-{}-products", crawler_id.get()),
        format,
        &[
            "sku",
            "name",
            "category",
            "units",
            "price",
            "amount",
            "description",
            "url",
        ],
        &rows,
    )
    .map_err(|err| ServiceError::Form(err.to_string()))
}

/// Starts crawling for the specified crawler.
///
/// Validates the `parser` role, ensures the crawler belongs to the user's hub
/// and sends a ZMQ message to trigger crawling. Returns `Ok(true)` if the
/// message was sent successfully, `Ok(false)` if sending failed, or an error if
/// the crawler was not found or a repository error occurred.
pub async fn crawl_crawler<R, S>(
    crawler_id: i32,
    user: &AuthenticatedUser,
    repo: &R,
    sender: &S,
) -> ServiceResult<bool>
where
    R: CrawlerReader,
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

    let crawler_id = match CrawlerId::new(crawler_id) {
        Ok(crawler_id) => crawler_id,
        Err(_) => return Err(ServiceError::NotFound),
    };

    let crawler = match repo.get_crawler_by_id(crawler_id, hub_id) {
        Ok(Some(crawler)) => crawler,
        Ok(None) => return Err(ServiceError::NotFound),
        Err(e) => {
            log::error!("Failed to get crawler by id: {e}");
            return Err(ServiceError::Internal);
        }
    };

    let message = ZMQCrawlerMessage::Crawler(CrawlerSelector::Selector(crawler.selector));
    match sender.send_json(&message).await {
        Ok(_) => Ok(true),
        Err(_) => {
            log::error!("Failed to send ZMQ message");
            Ok(false)
        }
    }
}

/// Updates prices for all products of the specified crawler.
///
/// Performs the same validations as [`crawl_crawler`] but also fetches all
/// product URLs for the crawler before sending a ZMQ message. Returns
/// `Ok(true)` if the message was sent successfully, `Ok(false)` if sending
/// failed, or an error if the crawler was not found or a repository error
/// occurred.
pub async fn update_crawler_prices<R, S>(
    crawler_id: i32,
    user: &AuthenticatedUser,
    repo: &R,
    sender: &S,
) -> ServiceResult<bool>
where
    R: CrawlerReader + ProductReader,
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

    let crawler_id = match CrawlerId::new(crawler_id) {
        Ok(crawler_id) => crawler_id,
        Err(_) => return Err(ServiceError::NotFound),
    };

    let crawler = match repo.get_crawler_by_id(crawler_id, hub_id) {
        Ok(Some(crawler)) => crawler,
        Ok(None) => return Err(ServiceError::NotFound),
        Err(e) => {
            log::error!("Failed to get crawler by id: {e}");
            return Err(ServiceError::Internal);
        }
    };

    let products = match repo.list_products(ProductListQuery::default().crawler(crawler_id)) {
        Ok((_total, products)) => products,
        Err(e) => {
            log::error!("Failed to get products: {e}");
            return Err(ServiceError::Internal);
        }
    };

    let urls = products
        .into_iter()
        .filter_map(|p| p.url)
        .collect::<Vec<_>>();
    if urls.is_empty() {
        return Ok(false);
    }

    let message =
        ZMQCrawlerMessage::Crawler(CrawlerSelector::SelectorProducts((crawler.selector, urls)));

    match sender.send_json(&message).await {
        Ok(_) => Ok(true),
        Err(_) => {
            log::error!("Failed to send ZMQ message");
            Ok(false)
        }
    }
}

/// Upload crawler products using format/mode-aware import parser and SKU upsert semantics.
pub fn upload_crawler_products<R>(
    crawler_id: i32,
    form: &mut UploadImportForm,
    user: &AuthenticatedUser,
    repo: &R,
) -> ServiceResult<UploadReport>
where
    R: CrawlerReader + ProductReader + ProductWriter,
{
    if !check_role(SERVICE_ACCESS_ROLE, &user.roles) {
        return Err(ServiceError::Unauthorized);
    }

    let hub_id = HubId::new(user.hub_id).map_err(|_| ServiceError::Internal)?;
    let crawler_id = CrawlerId::new(crawler_id).map_err(|_| ServiceError::NotFound)?;
    match repo.get_crawler_by_id(crawler_id, hub_id) {
        Ok(Some(_)) => {}
        Ok(None) => return Err(ServiceError::NotFound),
        Err(err) => {
            log::error!("Failed to load crawler for upload: {err}");
            return Err(ServiceError::Internal);
        }
    }

    let parsed = parse_upload(form, UploadTarget::CrawlerProducts)
        .map_err(|err| ServiceError::Form(err.to_string()))?;
    apply_crawler_upload(parsed, crawler_id, repo)
}

fn apply_crawler_upload<R>(
    parsed: crate::forms::import_export::ParsedUpload,
    crawler_id: CrawlerId,
    repo: &R,
) -> ServiceResult<UploadReport>
where
    R: ProductReader + ProductWriter,
{
    let mut report = UploadReport::with_total(parsed.rows.len());
    let mut seen_skus = std::collections::HashSet::new();

    for row in parsed.rows {
        let sku_value = row
            .values
            .get("sku")
            .cloned()
            .unwrap_or_default()
            .trim()
            .to_string();
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

        let sku = match crate::domain::types::ProductSku::new(sku_value.clone()) {
            Ok(sku) => sku,
            Err(err) => {
                report.push_error(row.row_number, Some(sku_value), err.to_string());
                continue;
            }
        };

        let existing = match repo.list_products_by_crawler_and_sku(crawler_id, &sku) {
            Ok(items) => items,
            Err(err) => {
                log::error!("Failed to lookup products by sku: {err}");
                return Err(ServiceError::Internal);
            }
        };
        if existing.len() > 1 {
            report.push_error(
                row.row_number,
                Some(sku_value),
                "Multiple existing products found for sku",
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
                .entry("price".to_string())
                .or_insert_with(|| current.price.get().to_string());
            if !merged.contains_key("category") {
                merged.insert(
                    "category".to_string(),
                    current
                        .category
                        .as_ref()
                        .map(|v| v.as_str().to_string())
                        .unwrap_or_default(),
                );
            }
            if !merged.contains_key("units") {
                merged.insert(
                    "units".to_string(),
                    current
                        .units
                        .as_ref()
                        .map(|v| v.as_str().to_string())
                        .unwrap_or_default(),
                );
            }
            if !merged.contains_key("amount") {
                merged.insert(
                    "amount".to_string(),
                    current
                        .amount
                        .map(|v| v.get().to_string())
                        .unwrap_or_default(),
                );
            }
            if !merged.contains_key("description") {
                merged.insert(
                    "description".to_string(),
                    current
                        .description
                        .as_ref()
                        .map(|v| v.as_str().to_string())
                        .unwrap_or_default(),
                );
            }
            if !merged.contains_key("url") {
                merged.insert(
                    "url".to_string(),
                    current
                        .url
                        .as_ref()
                        .map(|v| v.as_str().to_string())
                        .unwrap_or_default(),
                );
            }
        }

        let new_product = match build_product_from_row(&merged, crawler_id) {
            Ok(item) => item,
            Err(err) => {
                report.push_error(row.row_number, Some(sku_value), err);
                continue;
            }
        };

        if let Some(current) = existing.first() {
            match repo.update_product(current.id, &new_product) {
                Ok(_) => report.updated += 1,
                Err(err) => {
                    log::error!("Failed to update product: {err}");
                    report.push_error(row.row_number, Some(sku_value), "Failed to update product");
                }
            }
            continue;
        }

        if parsed.mode == UploadMode::Partial {
            let has_required = ["name", "price"].iter().all(|field| {
                merged
                    .get(*field)
                    .map(|value| !value.trim().is_empty())
                    .unwrap_or(false)
            });
            if !has_required {
                report.push_error(
                    row.row_number,
                    Some(sku_value),
                    "Partial mode create requires name and price",
                );
                continue;
            }
        }

        match repo.create_product(&new_product) {
            Ok(_) => report.created += 1,
            Err(err) => {
                log::error!("Failed to create product: {err}");
                report.push_error(row.row_number, Some(sku_value), "Failed to create product");
            }
        }
    }

    Ok(report)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::types::{
        CategoryAssignmentSource, CategoryName, CrawlerId, CrawlerName, CrawlerSelectorValue,
        CrawlerUrl, HubId, ProductCount, ProductId, ProductName, ProductPrice, ProductSku,
        ProductUnits, ProductUrl,
    };
    use crate::forms::import_export::{ParsedUpload, ParsedUploadRow, UploadFormat, UploadMode};
    use crate::repository::test::TestRepository;
    use chrono::DateTime;
    use pushkind_common::domain::auth::AuthenticatedUser;
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
            category: Some(CategoryName::new("category").unwrap()),
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

    #[test]
    fn returns_products_for_authorized_user() {
        let repo = TestRepository::new(vec![sample_crawler()], vec![sample_product()], vec![]);
        let user = sample_user();

        let (crawler, paginated) = show_products(1, 1, &user, &repo).unwrap();

        assert_eq!(crawler.id, 1);
        let value: Value = serde_json::to_value(&paginated).unwrap();
        assert_eq!(value["page"], 1);
        assert_eq!(value["items"].as_array().unwrap().len(), 1);
    }

    #[test]
    fn crawler_download_csv_contains_expected_headers() {
        let repo = TestRepository::new(vec![sample_crawler()], vec![sample_product()], vec![]);
        let user = sample_user();

        let file = download_crawler_products(1, "csv", &user, &repo).unwrap();
        let body = String::from_utf8(file.bytes).unwrap();
        assert!(body.starts_with("sku,name,category,units,price,amount,description,url"));
    }

    #[test]
    fn crawler_upload_reports_db_duplicate_sku_conflict() {
        let mut p1 = sample_product();
        p1.id = ProductId::new(1).unwrap();
        let mut p2 = sample_product();
        p2.id = ProductId::new(2).unwrap();
        let repo = TestRepository::new(vec![sample_crawler()], vec![p1, p2], vec![]);
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

        let report = apply_crawler_upload(parsed, CrawlerId::new(1).unwrap(), &repo).unwrap();
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
    async fn update_crawler_prices_returns_false_when_all_urls_missing() {
        let mut p = sample_product();
        p.url = None;
        let repo = TestRepository::new(vec![sample_crawler()], vec![p], vec![]);
        let user = sample_user();
        let sender = NoopSender;

        let sent = update_crawler_prices(1, &user, &repo, &sender)
            .await
            .unwrap();
        assert!(!sent);
    }
}
