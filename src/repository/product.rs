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
    fn list(&self, query: ProductListQuery) -> RepositoryResult<(usize, Vec<Product>)> {
        use crate::schema::products;

        let mut conn = self.pool.get()?;

        let query_builder = || {
            products::table
                .filter(products::crawler_id.eq(query.crawler_id))
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
            .order(products::name.asc())
            .load::<DbProduct>(&mut conn)?
            .into_iter()
            .map(Into::into)
            .collect::<Vec<Product>>();

        Ok((total, items))
    }
}
impl ProductWriter for DieselProductRepository<'_> {}
