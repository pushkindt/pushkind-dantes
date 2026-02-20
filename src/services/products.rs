use pushkind_common::domain::auth::AuthenticatedUser;
use pushkind_common::pagination::{DEFAULT_ITEMS_PER_PAGE, Paginated};
use pushkind_common::routes::check_role;
use pushkind_common::zmq::ZmqSenderExt;

use crate::SERVICE_ACCESS_ROLE;
use crate::domain::types::{CrawlerId, HubId};
use crate::domain::zmq::{CrawlerSelector, ZMQCrawlerMessage};
use crate::domain::{crawler::Crawler, product::Product};
use crate::repository::{CrawlerReader, ProductListQuery, ProductReader};

use super::{ServiceError, ServiceResult};

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

    let message = ZMQCrawlerMessage::Crawler(CrawlerSelector::SelectorProducts((
        crawler.selector,
        products.into_iter().map(|p| p.url).collect(),
    )));

    match sender.send_json(&message).await {
        Ok(_) => Ok(true),
        Err(_) => {
            log::error!("Failed to send ZMQ message");
            Ok(false)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::types::{
        CategoryAssignmentSource, CategoryName, CrawlerId, CrawlerName, CrawlerSelectorValue,
        CrawlerUrl, HubId, ProductCount, ProductId, ProductName, ProductPrice, ProductSku,
        ProductUnits, ProductUrl,
    };
    use crate::repository::test::TestRepository;
    use chrono::DateTime;
    use pushkind_common::domain::auth::AuthenticatedUser;
    use serde_json::Value;

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
            url: ProductUrl::new("http://example.com").unwrap(),
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
}
