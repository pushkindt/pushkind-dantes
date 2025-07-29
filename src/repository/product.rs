use diesel::prelude::*;
use pushkind_common::db::DbPool;
use pushkind_common::repository::errors::RepositoryResult;

use crate::domain::product::Product;
use crate::models::product::Product as DbProduct;
use crate::repository::{ProductReader, ProductWriter};

pub struct DieselProductRepository<'a> {
    pub pool: &'a DbPool,
}

impl<'a> DieselProductRepository<'a> {
    pub fn new(pool: &'a DbPool) -> Self {
        Self { pool }
    }
}

impl ProductReader for DieselProductRepository<'_> {
    fn list(&self, crawler_id: i32) -> RepositoryResult<Vec<Product>> {
        use crate::schema::products;

        let mut conn = self.pool.get()?;

        let results = products::table
            .filter(products::crawler_id.eq(crawler_id))
            .order(products::name.asc())
            .get_results::<DbProduct>(&mut conn)?;

        Ok(results
            .into_iter()
            .map(|db_product| db_product.into())
            .collect()) // Convert DbProduct to DomainProduct
    }
}
impl ProductWriter for DieselProductRepository<'_> {}
