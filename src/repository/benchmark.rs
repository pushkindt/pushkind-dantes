use diesel::prelude::*;
use pushkind_common::db::DbPool;
use pushkind_common::repository::errors::RepositoryResult;

use crate::repository::{BenchmarkListQuery, BenchmarkReader, BenchmarkWriter};
use pushkind_common::domain::benchmark::{Benchmark, NewBenchmark};
use pushkind_common::models::benchmark::{
    Benchmark as DbBenchmark, NewBenchmark as DbNewBenchmark,
};

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
        use pushkind_common::schema::dantes::benchmarks;

        let mut conn = self.pool.get()?;

        let benchmark = benchmarks::table
            .filter(benchmarks::id.eq(id))
            .first::<DbBenchmark>(&mut conn)
            .optional()?;

        Ok(benchmark.map(Into::into))
    }

    fn list(&self, query: BenchmarkListQuery) -> RepositoryResult<(usize, Vec<Benchmark>)> {
        use pushkind_common::schema::dantes::benchmarks;

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
        use pushkind_common::schema::dantes::benchmarks;

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

    fn delete_association(&self, benchmark_id: i32, product_id: i32) -> RepositoryResult<usize> {
        use pushkind_common::schema::dantes::product_benchmark;

        let mut conn = self.pool.get()?;

        let affected = diesel::delete(
            product_benchmark::table
                .filter(product_benchmark::benchmark_id.eq(benchmark_id))
                .filter(product_benchmark::product_id.eq(product_id)),
        )
        .execute(&mut conn)?;

        Ok(affected)
    }
}
