use log::error;
use pushkind_common::domain::auth::AuthenticatedUser;
use pushkind_common::domain::dantes::crawler::Crawler;
use pushkind_common::routes::ensure_role;

use crate::repository::CrawlerReader;

use super::errors::{ServiceError, ServiceResult};

/// Core business logic for rendering the index page.
///
/// The function validates that the user has the `parser` role and fetches
/// all crawlers associated with the user's hub. Any repository errors are
/// translated into `ServiceError` so that the HTTP route can remain a thin
/// wrapper.
pub fn show_index<R>(repo: &R, user: &AuthenticatedUser) -> ServiceResult<Vec<Crawler>>
where
    R: CrawlerReader,
{
    if ensure_role(user, "parser", None).is_err() {
        return Err(ServiceError::Unauthorized);
    }

    match repo.list_crawlers(user.hub_id) {
        Ok(crawlers) => Ok(crawlers),
        Err(e) => {
            error!("Failed to list crawlers: {e}");
            Err(ServiceError::Internal)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::repository::test::TestRepository;
    use chrono::DateTime;
    use pushkind_common::domain::dantes::crawler::Crawler;

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

    #[test]
    fn returns_crawlers_for_authorized_user() {
        let repo = TestRepository::new(vec![sample_crawler()], vec![], vec![]);
        let user = sample_user();

        let result = show_index(&repo, &user).unwrap();

        assert_eq!(result.len(), 1);
        assert_eq!(result[0].id, 1);
    }
}
