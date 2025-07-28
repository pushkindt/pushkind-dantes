use diesel::prelude::*;
use pushkind_common::db::DbPool;
use pushkind_common::repository::errors::RepositoryResult;

use crate::domain::crawler::Crawler;
use crate::models::crawler::Crawler as DbCrawler;
use crate::repository::{CrawlerReader, CrawlerWriter};

pub struct DieselCrawlerRepository<'a> {
    pub pool: &'a DbPool,
}

impl<'a> DieselCrawlerRepository<'a> {
    pub fn new(pool: &'a DbPool) -> Self {
        Self { pool }
    }
}

impl CrawlerReader for DieselCrawlerRepository<'_> {
    fn list(&mut self) -> RepositoryResult<Vec<Crawler>> {
        use crate::schema::crawlers;

        let mut conn = self.pool.get()?;

        let results = crawlers::table.load::<DbCrawler>(&mut conn)?;

        Ok(results
            .into_iter()
            .map(|db_crawler| db_crawler.into())
            .collect()) // Convert DbCrawler to DomainCrawler
    }

    fn get_by_id(&mut self, id: i32) -> RepositoryResult<Option<Crawler>> {
        use crate::schema::crawlers;

        let mut conn = self.pool.get()?;

        let result = crawlers::table
            .filter(crawlers::id.eq(id))
            .first::<DbCrawler>(&mut conn)
            .optional()?;

        Ok(result.map(|db_crawler| db_crawler.into())) // Convert DbCrawler to DomainCrawler
    }
}
impl CrawlerWriter for DieselCrawlerRepository<'_> {
    fn set_processing(&mut self, id: i32, status: bool) -> RepositoryResult<usize> {
        use crate::schema::crawlers;

        let mut conn = self.pool.get()?;

        diesel::update(crawlers::table.filter(crawlers::id.eq(id)))
            .set(crawlers::processing.eq(status))
            .execute(&mut conn)
            .map_err(|e| e.into())
    }
}
