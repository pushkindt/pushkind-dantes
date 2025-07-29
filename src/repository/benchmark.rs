use diesel::prelude::*;
use pushkind_common::db::DbPool;
use pushkind_common::repository::errors::RepositoryResult;

use crate::domain::benchmark::{Benchmark, NewBenchmark};
use crate::models::benchmark::{Benchmark as DbBenchmark, NewBenchmark as DbNewBenchmark};
use crate::repository::{BenchmarkReader, BenchmarkWriter};

pub struct DieselBenchmarkRepository<'a> {
    pub pool: &'a DbPool,
}

impl<'a> DieselBenchmarkRepository<'a> {
    pub fn new(pool: &'a DbPool) -> Self {
        Self { pool }
    }
}

impl BenchmarkReader for DieselBenchmarkRepository<'_> {
    fn list(&self, hub_id: i32) -> RepositoryResult<Vec<Benchmark>> {
        use crate::schema::benchmarks;

        let mut conn = self.pool.get()?;

        let results = benchmarks::table
            .filter(benchmarks::hub_id.eq(hub_id))
            .order(benchmarks::name.asc())
            .get_results::<DbBenchmark>(&mut conn)?;

        Ok(results
            .into_iter()
            .map(|db_benchmark| db_benchmark.into())
            .collect()) // Convert DbBenchmark to DomainBenchmark
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
