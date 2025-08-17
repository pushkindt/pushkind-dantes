use log::error;
use pushkind_common::domain::benchmark::Benchmark;
use pushkind_common::models::auth::AuthenticatedUser;
use pushkind_common::routes::ensure_role;

use crate::repository::{BenchmarkListQuery, BenchmarkReader};

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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::repository::test::TestRepository;
    use chrono::NaiveDateTime;
    use pushkind_common::domain::benchmark::Benchmark;

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
}
