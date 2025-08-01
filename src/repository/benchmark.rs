use diesel::prelude::*;
use pushkind_common::db::DbPool;
use pushkind_common::repository::errors::RepositoryResult;

use crate::domain::benchmark::{Benchmark, NewBenchmark};
use crate::models::benchmark::{Benchmark as DbBenchmark, NewBenchmark as DbNewBenchmark};
use crate::repository::{BenchmarkListQuery, BenchmarkReader, BenchmarkWriter};

pub struct DieselBenchmarkRepository<'a> {
    pub pool: &'a DbPool,
}

impl<'a> DieselBenchmarkRepository<'a> {
    pub fn new(pool: &'a DbPool) -> Self {
        Self { pool }
    }
}

impl BenchmarkReader for DieselBenchmarkRepository<'_> {
    fn get_by_id(&self, id: i32) -> RepositoryResult<Option<Benchmark>> {
        use crate::schema::benchmarks;

        let mut conn = self.pool.get()?;

        let benchmark = benchmarks::table
            .filter(benchmarks::id.eq(id))
            .first::<DbBenchmark>(&mut conn)
            .optional()?;

        Ok(benchmark.map(Into::into))
    }

    fn list(&self, query: BenchmarkListQuery) -> RepositoryResult<(usize, Vec<Benchmark>)> {
        use crate::schema::benchmarks;

        let mut conn = self.pool.get()?;

        let query_builder = || {
            benchmarks::table
                .filter(benchmarks::hub_id.eq(query.hub_id))
                .into_boxed::<diesel::sqlite::Sqlite>()
        };

        let total = query_builder().count().get_result::<i64>(&mut conn)? as usize;

        let mut items = query_builder();

        // Apply pagination if requested
        if let Some(pagination) = &query.pagination {
            let offset = ((pagination.page.max(1) - 1) * pagination.per_page) as i64;
            let limit = pagination.per_page as i64;
            items = items.offset(offset).limit(limit);
        }

        // Final load
        let items = items
            .order(benchmarks::name.asc())
            .load::<DbBenchmark>(&mut conn)?
            .into_iter()
            .map(Into::into)
            .collect::<Vec<Benchmark>>();

        Ok((total, items))
    }
}
impl BenchmarkWriter for DieselBenchmarkRepository<'_> {
    fn create(&self, benchmarks: &[NewBenchmark]) -> RepositoryResult<usize> {
        use crate::schema::benchmarks;

        let mut conn = self.pool.get()?;

        let db_benchmarks = benchmarks
            .iter()
            .map(|benchmark| benchmark.into())
            .collect::<Vec<DbNewBenchmark>>();

        let affected = diesel::insert_into(benchmarks::table)
            .values(&db_benchmarks)
            .execute(&mut conn)?;

        Ok(affected)
    }
}
