use log::error;
use pushkind_common::domain::auth::AuthenticatedUser;
use pushkind_common::domain::dantes::product::Product;
use pushkind_common::pagination::DEFAULT_ITEMS_PER_PAGE;
use pushkind_common::routes::ensure_role;
use serde::Deserialize;

use crate::repository::{CrawlerReader, ProductListQuery, ProductReader};

use super::errors::{ServiceError, ServiceResult};

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
    repo: &R,
    params: ApiV1ProductsQueryParams,
    user: &AuthenticatedUser,
) -> ServiceResult<Vec<Product>>
where
    R: CrawlerReader + ProductReader,
{
    if ensure_role(user, "parser", None).is_err() {
        return Err(ServiceError::Unauthorized);
    }

    let crawler = match repo.get_crawler_by_id(params.crawler_id, user.hub_id) {
        Ok(Some(crawler)) => crawler,
        Err(e) => {
            error!("Failed to get crawler: {e}");
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
            error!("Failed to list products: {e}");
            Err(ServiceError::Internal)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::repository::test::TestRepository;
    use chrono::NaiveDateTime;
    use pushkind_common::domain::dantes::{crawler::Crawler, product::Product};

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
            name: "Apple".into(),
            sku: "SKU1".into(),
            category: None,
            units: None,
            price: 1.0,
            amount: None,
            description: None,
            url: "http://example.com/apple".into(),
            created_at: NaiveDateTime::from_timestamp(0, 0),
            updated_at: NaiveDateTime::from_timestamp(0, 0),
            embedding: Some(vec![1, 2, 3]),
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

        let result = api_v1_products(&repo, params, &user).unwrap();

        assert_eq!(result.len(), 1);
        assert!(result[0].embedding.is_none());
    }
}
