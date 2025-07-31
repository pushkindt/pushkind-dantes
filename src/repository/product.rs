use std::collections::HashMap;

use diesel::prelude::*;
use pushkind_common::db::DbPool;
use pushkind_common::repository::errors::RepositoryResult;

use crate::domain::product::Product;
use crate::models::product::Product as DbProduct;
use crate::repository::{ProductListQuery, ProductReader, ProductWriter};

pub struct DieselProductRepository<'a> {
    pub pool: &'a DbPool,
}

impl<'a> DieselProductRepository<'a> {
    pub fn new(pool: &'a DbPool) -> Self {
        Self { pool }
    }
}

impl ProductReader for DieselProductRepository<'_> {
    fn list_distances(&self, benchmark_id: i32) -> RepositoryResult<HashMap<i32, f32>> {
        use crate::schema::product_benchmark;

        let mut conn = self.pool.get()?;

        let items: Vec<(i32, f32)> = product_benchmark::table
            .filter(product_benchmark::benchmark_id.eq(benchmark_id))
            .select((product_benchmark::product_id, product_benchmark::distance))
            .order(product_benchmark::distance.asc())
            .load(&mut conn)?;

        Ok(items.into_iter().collect())
    }

    fn list(&self, query: ProductListQuery) -> RepositoryResult<(usize, Vec<Product>)> {
        use crate::schema::{product_benchmark, products};

        let mut conn = self.pool.get()?;

        let query_builder = || {
            let mut items = products::table.into_boxed::<diesel::sqlite::Sqlite>();

            if let Some(crawler_id) = query.crawler_id {
                items = items.filter(products::crawler_id.eq(crawler_id));
            }

            if let Some(benchmark_id) = query.benchmark_id {
                items = items.filter(
                    products::id.eq_any(
                        product_benchmark::table
                            .filter(product_benchmark::benchmark_id.eq(benchmark_id))
                            .select(product_benchmark::product_id),
                    ),
                );
            }

            items
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
            .order(products::name.asc())
            .load::<DbProduct>(&mut conn)?
            .into_iter()
            .map(Into::into)
            .collect::<Vec<Product>>();

        Ok((total, items))
    }
}
impl ProductWriter for DieselProductRepository<'_> {}
