use pushkind_common::domain::auth::AuthenticatedUser;
use pushkind_common::pagination::DEFAULT_ITEMS_PER_PAGE;
use pushkind_common::routes::check_role;
use serde::Deserialize;

use crate::SERVICE_ACCESS_ROLE;
use crate::domain::product::Product;
use crate::domain::types::{CrawlerId, HubId};
use crate::repository::{CrawlerReader, ProductListQuery, ProductReader};

use super::{ServiceError, ServiceResult};

/// Query parameters accepted by the `api_v1_products` endpoint.
#[derive(Deserialize, Debug)]
pub struct ApiV1ProductsQueryParams {
    pub crawler_id: i32,
    pub query: Option<String>,
    pub page: Option<usize>,
}

/// Core business logic for the `/v1/products` API endpoint.
///
/// The function returns a list of products for the requested crawler,
/// performing optional search and pagination. All repository interactions and
/// role checks are handled here so that the HTTP route can remain a thin
/// wrapper.
pub fn api_v1_products<R>(
    params: ApiV1ProductsQueryParams,
    user: &AuthenticatedUser,
    repo: &R,
) -> ServiceResult<Vec<Product>>
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

    let crawler_id = match CrawlerId::new(params.crawler_id) {
        Ok(crawler_id) => crawler_id,
        Err(_) => return Err(ServiceError::NotFound),
    };

    let crawler = match repo.get_crawler_by_id(crawler_id, hub_id) {
        Ok(Some(crawler)) => crawler,
        Err(e) => {
            log::error!("Failed to get crawler: {e}");
            return Err(ServiceError::Internal);
        }
        Ok(None) => return Err(ServiceError::NotFound),
    };

    let mut list_query = ProductListQuery::default().crawler(crawler.id);

    let page = params.page.unwrap_or(1);
    list_query = list_query.paginate(page, DEFAULT_ITEMS_PER_PAGE);

    let result = match &params.query {
        Some(query) if !query.is_empty() => {
            list_query = list_query.search(query);
            repo.search_products(list_query)
        }
        _ => repo.list_products(list_query),
    };

    match result {
        Ok((_total, products)) => Ok(products
            .into_iter()
            .map(|mut p| {
                p.embedding = None;
                p
            })
            .collect::<Vec<Product>>()),
        Err(e) => {
            log::error!("Failed to list products: {e}");
            Err(ServiceError::Internal)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::types::{
        CategoryAssignmentSource, CrawlerId, CrawlerName, CrawlerSelectorValue, CrawlerUrl, HubId,
        ProductCount, ProductId, ProductName, ProductPrice, ProductSku, ProductUrl,
    };
    use crate::domain::{crawler::Crawler, product::Product};
    use crate::repository::test::TestRepository;
    use chrono::DateTime;

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
            name: ProductName::new("Apple").unwrap(),
            sku: ProductSku::new("SKU1").unwrap(),
            category: None,
            units: None,
            price: ProductPrice::new(1.0).unwrap(),
            amount: None,
            description: None,
            url: ProductUrl::new("http://example.com/apple").unwrap(),
            created_at: DateTime::from_timestamp(0, 0).unwrap().naive_utc(),
            updated_at: DateTime::from_timestamp(0, 0).unwrap().naive_utc(),
            embedding: Some(vec![1, 2, 3]),
            category_id: None,
            category_assignment_source: CategoryAssignmentSource::Automatic,
            images: vec![],
        }
    }

    #[test]
    fn returns_products_without_embeddings() {
        let repo = TestRepository::new(vec![sample_crawler()], vec![sample_product()], vec![]);
        let user = sample_user();
        let params = ApiV1ProductsQueryParams {
            crawler_id: 1,
            query: None,
            page: None,
        };

        let result = api_v1_products(params, &user, &repo).unwrap();

        assert_eq!(result.len(), 1);
        assert!(result[0].embedding.is_none());
    }
}
