use pushkind_common::domain::auth::AuthenticatedUser;
use pushkind_common::routes::check_role;

use crate::SERVICE_ACCESS_ROLE;
use crate::domain::crawler::Crawler;
use crate::domain::types::HubId;
use crate::repository::CrawlerReader;

use super::{ServiceError, ServiceResult};

/// Core business logic for rendering the index page.
///
/// The function validates that the user has the `parser` role and fetches
/// all crawlers associated with the user's hub. Any repository errors are
/// translated into `ServiceError` so that the HTTP route can remain a thin
/// wrapper.
pub fn show_index<R>(user: &AuthenticatedUser, repo: &R) -> ServiceResult<Vec<Crawler>>
where
    R: CrawlerReader,
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

    match repo.list_crawlers(hub_id) {
        Ok(crawlers) => Ok(crawlers),
        Err(e) => {
            log::error!("Failed to list crawlers: {e}");
            Err(ServiceError::Internal)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::crawler::Crawler;
    use crate::domain::types::{
        CrawlerId, CrawlerName, CrawlerSelectorValue, CrawlerUrl, HubId, ProductCount,
    };
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

    #[test]
    fn returns_crawlers_for_authorized_user() {
        let repo = TestRepository::new(vec![sample_crawler()], vec![], vec![]);
        let user = sample_user();

        let result = show_index(&user, &repo).unwrap();

        assert_eq!(result.len(), 1);
        assert_eq!(result[0].id, 1);
    }
}
