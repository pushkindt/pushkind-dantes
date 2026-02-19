use diesel::prelude::*;
use pushkind_common::repository::errors::RepositoryResult;

use crate::domain::benchmark::{Benchmark, NewBenchmark};
use crate::models::benchmark::{Benchmark as DbBenchmark, NewBenchmark as DbNewBenchmark};
use crate::repository::{BenchmarkListQuery, BenchmarkReader, BenchmarkWriter, DieselRepository};

impl BenchmarkReader for DieselRepository {
    fn get_benchmark_by_id(&self, id: i32, hub_id: i32) -> RepositoryResult<Option<Benchmark>> {
        use crate::schema::benchmarks;

        let mut conn = self.conn()?;

        let benchmark = benchmarks::table
            .filter(benchmarks::id.eq(id))
            .filter(benchmarks::hub_id.eq(hub_id))
            .first::<DbBenchmark>(&mut conn)
            .optional()?;

        let benchmark = benchmark.map(TryInto::try_into).transpose()?;
        Ok(benchmark)
    }

    fn list_benchmarks(
        &self,
        query: BenchmarkListQuery,
    ) -> RepositoryResult<(usize, Vec<Benchmark>)> {
        use crate::schema::benchmarks;

        let mut conn = self.conn()?;

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
            .map(TryInto::try_into)
            .collect::<Result<Vec<Benchmark>, _>>()?;

        Ok((total, items))
    }
}
impl BenchmarkWriter for DieselRepository {
    fn create_benchmark(&self, benchmarks: &[NewBenchmark]) -> RepositoryResult<usize> {
        use crate::schema::benchmarks;

        let mut conn = self.conn()?;

        let db_benchmarks = benchmarks
            .iter()
            .map(|benchmark| benchmark.into())
            .collect::<Vec<DbNewBenchmark>>();

        let affected = diesel::insert_into(benchmarks::table)
            .values(&db_benchmarks)
            .execute(&mut conn)?;

        Ok(affected)
    }

    fn remove_benchmark_association(
        &self,
        benchmark_id: i32,
        product_id: i32,
    ) -> RepositoryResult<usize> {
        use crate::schema::product_benchmark;

        let mut conn = self.conn()?;

        let affected = diesel::delete(
            product_benchmark::table
                .filter(product_benchmark::benchmark_id.eq(benchmark_id))
                .filter(product_benchmark::product_id.eq(product_id)),
        )
        .execute(&mut conn)?;

        Ok(affected)
    }

    fn set_benchmark_association(
        &self,
        benchmark_id: i32,
        product_id: i32,
        distance: f32,
    ) -> RepositoryResult<usize> {
        use crate::schema::product_benchmark;

        let mut conn = self.conn()?;

        // Insert association entry with similarity distance
        let affected = diesel::insert_into(product_benchmark::table)
            .values((
                product_benchmark::benchmark_id.eq(benchmark_id),
                product_benchmark::product_id.eq(product_id),
                product_benchmark::distance.eq(distance),
            ))
            .on_conflict((
                product_benchmark::product_id,
                product_benchmark::benchmark_id,
            ))
            .do_nothing()
            .execute(&mut conn)?;

        Ok(affected)
    }
}
