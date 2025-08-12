use std::collections::HashMap;

use diesel::prelude::*;
use diesel::sql_types::{BigInt, Integer, Text};
use pushkind_common::db::DbPool;
use pushkind_common::domain::product::Product;
use pushkind_common::models::product::Product as DbProduct;
use pushkind_common::repository::errors::RepositoryResult;

use crate::repository::{ProductListQuery, ProductReader, ProductWriter};

pub struct DieselProductRepository<'a> {
    pub pool: &'a DbPool,
}

impl<'a> DieselProductRepository<'a> {
    pub fn new(pool: &'a DbPool) -> Self {
        Self { pool }
    }
}

#[derive(QueryableByName)]
struct ProductCount {
    #[diesel(sql_type = diesel::sql_types::BigInt)]
    count: i64,
}

impl ProductReader for DieselProductRepository<'_> {
    fn get_product_by_id(&self, id: i32) -> RepositoryResult<Option<Product>> {
        use pushkind_common::schema::dantes::products;

        let mut conn = self.pool.get()?;

        let item = products::table
            .filter(products::id.eq(id))
            .first::<DbProduct>(&mut conn)
            .optional()?;

        Ok(item.map(Into::into))
    }

    fn list_distances(&self, benchmark_id: i32) -> RepositoryResult<HashMap<i32, f32>> {
        use pushkind_common::schema::dantes::product_benchmark;

        let mut conn = self.pool.get()?;

        let items: Vec<(i32, f32)> = product_benchmark::table
            .filter(product_benchmark::benchmark_id.eq(benchmark_id))
            .select((product_benchmark::product_id, product_benchmark::distance))
            .order(product_benchmark::distance.asc())
            .load(&mut conn)?;

        Ok(items.into_iter().collect())
    }

    fn list_products(&self, query: ProductListQuery) -> RepositoryResult<(usize, Vec<Product>)> {
        use pushkind_common::schema::dantes::{crawlers, product_benchmark, products};

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

            if let Some(hub_id) = query.hub_id {
                items = items.filter(
                    products::crawler_id.eq_any(
                        crawlers::table
                            .filter(crawlers::hub_id.eq(hub_id))
                            .select(crawlers::id),
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

    fn search_products(&self, query: ProductListQuery) -> RepositoryResult<(usize, Vec<Product>)> {
        let mut conn = self.pool.get()?;

        let match_query = match &query.search {
            None => return Ok((0, vec![])),
            Some(query) if query.trim().is_empty() => {
                return Ok((0, vec![]));
            }
            Some(query) => {
                format!("{query}*")
            }
        };

        // Build base SQL
        let mut sql = String::from(
            r#"
            SELECT products.*
            FROM products
            JOIN products_fts ON products.id = products_fts.rowid
            WHERE products_fts MATCH ?
            "#,
        );

        if query.crawler_id.is_some() {
            let crawler_filter = r#"
                AND products.crawler_id = ?
            "#;
            sql.push_str(crawler_filter);
        }

        if query.benchmark_id.is_some() {
            let benchmark_filter = r#"
                AND products.id IN (
                    SELECT product_benchmark.product_id
                    FROM product_benchmark
                    WHERE product_benchmark.benchmark_id = ?
                )
            "#;
            sql.push_str(benchmark_filter);
        }

        if query.hub_id.is_some() {
            let benchmark_filter = r#"
                AND products.crawler_id IN (
                    SELECT crawlers.id
                    FROM crawlers
                    WHERE crawlers.hub_id = ?
                )
            "#;
            sql.push_str(benchmark_filter);
        }

        let total_sql = format!("SELECT COUNT(*) as count FROM ({sql})");

        // Now add pagination to SQL (but not count)
        if query.pagination.is_some() {
            sql.push_str(" LIMIT ? OFFSET ? ");
        }

        // Build final data query
        let mut data_query = diesel::sql_query(&sql)
            .into_boxed()
            .bind::<Text, _>(&match_query);

        let mut total_query = diesel::sql_query(&total_sql)
            .into_boxed()
            .bind::<Text, _>(&match_query);

        if let Some(crawler_id) = &query.crawler_id {
            data_query = data_query.bind::<Integer, _>(crawler_id);
            total_query = total_query.bind::<Integer, _>(crawler_id);
        }

        if let Some(benchmark_id) = &query.benchmark_id {
            data_query = data_query.bind::<Integer, _>(benchmark_id);
            total_query = total_query.bind::<Integer, _>(benchmark_id);
        }

        if let Some(hub_id) = &query.hub_id {
            data_query = data_query.bind::<Integer, _>(hub_id);
            total_query = total_query.bind::<Integer, _>(hub_id);
        }

        if let Some(pagination) = &query.pagination {
            let limit = pagination.per_page as i64;
            let offset = ((pagination.page.max(1) - 1) * pagination.per_page) as i64;
            data_query = data_query
                .bind::<BigInt, _>(limit)
                .bind::<BigInt, _>(offset);
        }

        let items = data_query
            .load::<DbProduct>(&mut conn)?
            .into_iter()
            .map(Into::into)
            .collect();

        let total = total_query.get_result::<ProductCount>(&mut conn)?.count as usize;
        Ok((total, items))
    }
}
impl ProductWriter for DieselProductRepository<'_> {}
