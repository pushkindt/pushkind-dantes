use log::error;
use pushkind_common::domain::{crawler::Crawler, product::Product};
use pushkind_common::models::auth::AuthenticatedUser;
use pushkind_common::models::zmq::dantes::{CrawlerSelector, ZMQCrawlerMessage};
use pushkind_common::pagination::{DEFAULT_ITEMS_PER_PAGE, Paginated};
use pushkind_common::routes::ensure_role;

use crate::repository::{CrawlerReader, ProductListQuery, ProductReader};

use super::errors::{ServiceError, ServiceResult};

/// Core business logic for rendering the products page.
///
/// Validates that the user has the `parser` role, ensures the crawler belongs
/// to the user's hub, and fetches paginated products for the crawler.
/// Repository errors are converted into `ServiceError` variants so that the
/// HTTP route can remain a thin wrapper.
pub fn show_products<R>(
    repo: &R,
    user: &AuthenticatedUser,
    crawler_id: i32,
    page: usize,
) -> ServiceResult<(Crawler, Paginated<Product>)>
where
    R: CrawlerReader + ProductReader,
{
    if ensure_role(user, "parser", None).is_err() {
        return Err(ServiceError::Unauthorized);
    }

    let crawler = match repo.get_crawler_by_id(crawler_id, user.hub_id) {
        Ok(Some(crawler)) => crawler,
        Ok(None) => return Err(ServiceError::NotFound),
        Err(e) => {
            error!("Failed to get crawler: {e}");
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
            error!("Failed to list products: {e}");
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
pub async fn crawl_crawler<R, F>(
    repo: &R,
    user: &AuthenticatedUser,
    crawler_id: i32,
    send: F,
) -> ServiceResult<bool>
where
    R: CrawlerReader,
    F: AsyncFn(&ZMQCrawlerMessage) -> Result<(), ()>,
{
    if ensure_role(user, "parser", None).is_err() {
        return Err(ServiceError::Unauthorized);
    }

    let crawler = match repo.get_crawler_by_id(crawler_id, user.hub_id) {
        Ok(Some(crawler)) => crawler,
        Ok(None) => return Err(ServiceError::NotFound),
        Err(e) => {
            error!("Failed to get crawler by id: {e}");
            return Err(ServiceError::Internal);
        }
    };

    let message = ZMQCrawlerMessage::Crawler(CrawlerSelector::Selector(crawler.selector));
    match send(&message).await {
        Ok(_) => Ok(true),
        Err(_) => {
            error!("Failed to send ZMQ message");
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
pub async fn update_crawler_prices<R, F>(
    repo: &R,
    user: &AuthenticatedUser,
    crawler_id: i32,
    send: F,
) -> ServiceResult<bool>
where
    R: CrawlerReader + ProductReader,
    F: AsyncFn(&ZMQCrawlerMessage) -> Result<(), ()>,
{
    if ensure_role(user, "parser", None).is_err() {
        return Err(ServiceError::Unauthorized);
    }

    let crawler = match repo.get_crawler_by_id(crawler_id, user.hub_id) {
        Ok(Some(crawler)) => crawler,
        Ok(None) => return Err(ServiceError::NotFound),
        Err(e) => {
            error!("Failed to get crawler by id: {e}");
            return Err(ServiceError::Internal);
        }
    };

    let products = match repo.list_products(ProductListQuery::default().crawler(crawler_id)) {
        Ok((_total, products)) => products,
        Err(e) => {
            error!("Failed to get products: {e}");
            return Err(ServiceError::Internal);
        }
    };

    let message = ZMQCrawlerMessage::Crawler(CrawlerSelector::SelectorProducts((
        crawler.selector,
        products.into_iter().map(|p| p.url).collect(),
    )));

    match send(&message).await {
        Ok(_) => Ok(true),
        Err(_) => {
            error!("Failed to send ZMQ message");
            Ok(false)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::repository::test::TestRepository;
    use chrono::NaiveDateTime;
    use pushkind_common::models::auth::AuthenticatedUser;
    use pushkind_common::models::zmq::dantes::{CrawlerSelector, ZMQCrawlerMessage};
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
            category: Some("category".into()),
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

    #[test]
    fn returns_products_for_authorized_user() {
        let repo = TestRepository::new(vec![sample_crawler()], vec![sample_product()], vec![]);
        let user = sample_user();

        let (crawler, paginated) = show_products(&repo, &user, 1, 1).unwrap();

        assert_eq!(crawler.id, 1);
        let value: Value = serde_json::to_value(&paginated).unwrap();
        assert_eq!(value["page"], 1);
        assert_eq!(value["items"].as_array().unwrap().len(), 1);
    }
}
