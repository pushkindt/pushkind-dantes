use diesel::prelude::*;
use pushkind_common::db::DbPool;
use pushkind_common::repository::errors::RepositoryResult;

use crate::repository::CrawlerReader;
use pushkind_common::domain::crawler::Crawler;
use pushkind_common::models::crawler::Crawler as DbCrawler;

pub struct DieselCrawlerRepository<'a> {
    pub pool: &'a DbPool,
}

impl<'a> DieselCrawlerRepository<'a> {
    pub fn new(pool: &'a DbPool) -> Self {
        Self { pool }
    }
}

impl CrawlerReader for DieselCrawlerRepository<'_> {
    fn list_crawlers(&self, hub_id: i32) -> RepositoryResult<Vec<Crawler>> {
        use pushkind_common::schema::dantes::crawlers;

        let mut conn = self.pool.get()?;

        let results = crawlers::table
            .filter(crawlers::hub_id.eq(hub_id))
            .order(crawlers::id.asc())
            .get_results::<DbCrawler>(&mut conn)?;

        Ok(results
            .into_iter()
            .map(|db_crawler| db_crawler.into())
            .collect()) // Convert DbCrawler to DomainCrawler
    }

    fn get_crawler_by_id(&self, id: i32) -> RepositoryResult<Option<Crawler>> {
        use pushkind_common::schema::dantes::crawlers;

        let mut conn = self.pool.get()?;

        let result = crawlers::table
            .filter(crawlers::id.eq(id))
            .first::<DbCrawler>(&mut conn)
            .optional()?;

        Ok(result.map(|db_crawler| db_crawler.into())) // Convert DbCrawler to DomainCrawler
    }
}
