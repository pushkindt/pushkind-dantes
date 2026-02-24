use diesel::prelude::*;
use pushkind_common::repository::errors::RepositoryResult;

use crate::domain::benchmark::{Benchmark, NewBenchmark};
use crate::domain::types::{BenchmarkId, BenchmarkSku, HubId, ProductId, SimilarityDistance};
use crate::models::benchmark::{Benchmark as DbBenchmark, NewBenchmark as DbNewBenchmark};
use crate::repository::{BenchmarkListQuery, BenchmarkReader, BenchmarkWriter, DieselRepository};

impl BenchmarkReader for DieselRepository {
    fn get_benchmark_by_id(
        &self,
        id: BenchmarkId,
        hub_id: HubId,
    ) -> RepositoryResult<Option<Benchmark>> {
        use crate::schema::benchmarks;

        let mut conn = self.conn()?;

        let benchmark = benchmarks::table
            .filter(benchmarks::id.eq(id.get()))
            .filter(benchmarks::hub_id.eq(hub_id.get()))
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
                .filter(benchmarks::hub_id.eq(query.hub_id.get()))
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

    fn list_benchmarks_by_hub_and_sku(
        &self,
        hub_id: HubId,
        sku: &BenchmarkSku,
    ) -> RepositoryResult<Vec<Benchmark>> {
        use crate::schema::benchmarks;

        let mut conn = self.conn()?;

        let items = benchmarks::table
            .filter(benchmarks::hub_id.eq(hub_id.get()))
            .filter(benchmarks::sku.eq(sku.as_str()))
            .load::<DbBenchmark>(&mut conn)?
            .into_iter()
            .map(TryInto::try_into)
            .collect::<Result<Vec<Benchmark>, _>>()?;

        Ok(items)
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

    fn update_benchmark(
        &self,
        benchmark_id: BenchmarkId,
        benchmark: &NewBenchmark,
    ) -> RepositoryResult<usize> {
        use crate::schema::benchmarks;

        let mut conn = self.conn()?;
        let db_benchmark: DbNewBenchmark = benchmark.into();

        let affected =
            diesel::update(benchmarks::table.filter(benchmarks::id.eq(benchmark_id.get())))
                .set((
                    benchmarks::name.eq(db_benchmark.name),
                    benchmarks::sku.eq(db_benchmark.sku),
                    benchmarks::category.eq(db_benchmark.category),
                    benchmarks::units.eq(db_benchmark.units),
                    benchmarks::price.eq(db_benchmark.price),
                    benchmarks::amount.eq(db_benchmark.amount),
                    benchmarks::description.eq(db_benchmark.description),
                    benchmarks::updated_at.eq(db_benchmark.updated_at),
                ))
                .execute(&mut conn)?;

        Ok(affected)
    }

    fn remove_benchmark_association(
        &self,
        benchmark_id: BenchmarkId,
        product_id: ProductId,
    ) -> RepositoryResult<usize> {
        use crate::schema::product_benchmark;

        let mut conn = self.conn()?;

        let affected = diesel::delete(
            product_benchmark::table
                .filter(product_benchmark::benchmark_id.eq(benchmark_id.get()))
                .filter(product_benchmark::product_id.eq(product_id.get())),
        )
        .execute(&mut conn)?;

        Ok(affected)
    }

    fn set_benchmark_association(
        &self,
        benchmark_id: BenchmarkId,
        product_id: ProductId,
        distance: SimilarityDistance,
    ) -> RepositoryResult<usize> {
        use crate::schema::product_benchmark;

        let mut conn = self.conn()?;

        // Insert association entry with similarity distance
        let affected = diesel::insert_into(product_benchmark::table)
            .values((
                product_benchmark::benchmark_id.eq(benchmark_id.get()),
                product_benchmark::product_id.eq(product_id.get()),
                product_benchmark::distance.eq(distance.get()),
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
