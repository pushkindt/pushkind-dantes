use std::collections::HashMap;

use diesel::prelude::*;
use diesel::sql_types::{BigInt, Integer, Text};
use pushkind_common::repository::errors::RepositoryResult;

use crate::domain::product::Product;
use crate::domain::types::{BenchmarkId, ImageUrl, ProductId, SimilarityDistance};
use crate::models::product::Product as DbProduct;
use crate::repository::{DieselRepository, ProductListQuery, ProductReader, ProductWriter};

/// Helper struct used to capture the result of a `COUNT(*)` query.
#[derive(QueryableByName)]
struct ProductCount {
    #[diesel(sql_type = diesel::sql_types::BigInt)]
    count: i64,
}

impl ProductReader for DieselRepository {
    fn get_product_by_id(&self, id: ProductId) -> RepositoryResult<Option<Product>> {
        use crate::schema::{product_images, products};

        let mut conn = self.conn()?;

        let db_product = products::table
            .filter(products::id.eq(id.get()))
            .first::<DbProduct>(&mut conn)
            .optional()?;

        // Short-circuit early if no product exists
        let mut product: Product = match db_product {
            Some(p) => p.try_into()?,
            None => return Ok(None),
        };

        let images = product_images::table
            .filter(product_images::product_id.eq(id.get()))
            .select(product_images::url)
            .load::<String>(&mut conn)?;

        product.images = images
            .into_iter()
            .map(ImageUrl::new)
            .collect::<Result<Vec<ImageUrl>, _>>()?;

        Ok(Some(product))
    }

    fn list_distances(
        &self,
        benchmark_id: BenchmarkId,
    ) -> RepositoryResult<HashMap<ProductId, SimilarityDistance>> {
        use crate::schema::product_benchmark;

        let mut conn = self.conn()?;

        let items: Vec<(i32, f32)> = product_benchmark::table
            .filter(product_benchmark::benchmark_id.eq(benchmark_id.get()))
            .select((product_benchmark::product_id, product_benchmark::distance))
            .order(product_benchmark::distance.asc())
            .load(&mut conn)?;

        let mut distances = HashMap::with_capacity(items.len());
        for (product_id, distance) in items {
            distances.insert(
                ProductId::new(product_id)?,
                SimilarityDistance::new(distance)?,
            );
        }

        Ok(distances)
    }

    fn list_products(&self, query: ProductListQuery) -> RepositoryResult<(usize, Vec<Product>)> {
        use crate::schema::{crawlers, product_benchmark, product_images, products};

        let mut conn = self.conn()?;

        let query_builder = || {
            let mut items = products::table.into_boxed::<diesel::sqlite::Sqlite>();

            if let Some(crawler_id) = query.crawler_id {
                items = items.filter(products::crawler_id.eq(crawler_id.get()));
            }

            if let Some(benchmark_id) = query.benchmark_id {
                items = items.filter(
                    products::id.eq_any(
                        product_benchmark::table
                            .filter(product_benchmark::benchmark_id.eq(benchmark_id.get()))
                            .select(product_benchmark::product_id),
                    ),
                );
            }

            if let Some(hub_id) = query.hub_id {
                items = items.filter(
                    products::crawler_id.eq_any(
                        crawlers::table
                            .filter(crawlers::hub_id.eq(hub_id.get()))
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
        let mut items = items
            .order(products::name.asc())
            .load::<DbProduct>(&mut conn)?
            .into_iter()
            .map(TryInto::try_into)
            .collect::<Result<Vec<Product>, _>>()?;

        if !items.is_empty() {
            let product_ids: Vec<i32> = items.iter().map(|product| product.id.get()).collect();
            let image_rows = product_images::table
                .filter(product_images::product_id.eq_any(&product_ids))
                .select((product_images::product_id, product_images::url))
                .load::<(i32, String)>(&mut conn)?;

            let mut image_map: HashMap<i32, Vec<String>> = HashMap::new();
            for (product_id, url) in image_rows {
                image_map.entry(product_id).or_default().push(url);
            }

            for product in &mut items {
                if let Some(images) = image_map.remove(&product.id.get()) {
                    product.images = images.into_iter().map(ImageUrl::new).collect::<Result<
                        Vec<ImageUrl>,
                        _,
                    >>(
                    )?;
                }
            }
        }

        Ok((total, items))
    }

    fn search_products(&self, query: ProductListQuery) -> RepositoryResult<(usize, Vec<Product>)> {
        let mut conn = self.conn()?;

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
            data_query = data_query.bind::<Integer, _>(crawler_id.get());
            total_query = total_query.bind::<Integer, _>(crawler_id.get());
        }

        if let Some(benchmark_id) = &query.benchmark_id {
            data_query = data_query.bind::<Integer, _>(benchmark_id.get());
            total_query = total_query.bind::<Integer, _>(benchmark_id.get());
        }

        if let Some(hub_id) = &query.hub_id {
            data_query = data_query.bind::<Integer, _>(hub_id.get());
            total_query = total_query.bind::<Integer, _>(hub_id.get());
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
            .map(TryInto::try_into)
            .collect::<Result<Vec<Product>, _>>()?;

        let total = total_query.get_result::<ProductCount>(&mut conn)?.count as usize;
        Ok((total, items))
    }
}
impl ProductWriter for DieselRepository {}
