use diesel::prelude::*;
use pushkind_common::domain::crawler::Crawler;
use pushkind_common::models::crawler::Crawler as DbCrawler;
use pushkind_common::repository::errors::RepositoryResult;

use crate::repository::{CrawlerReader, DieselRepository};

impl CrawlerReader for DieselRepository {
    fn list_crawlers(&self, hub_id: i32) -> RepositoryResult<Vec<Crawler>> {
        use pushkind_common::schema::dantes::crawlers;

        let mut conn = self.conn()?;

        let results = crawlers::table
            .filter(crawlers::hub_id.eq(hub_id))
            .order(crawlers::id.asc())
            .get_results::<DbCrawler>(&mut conn)?;

        Ok(results
            .into_iter()
            .map(|db_crawler| db_crawler.into())
            .collect()) // Convert DbCrawler to DomainCrawler
    }

    fn get_crawler_by_id(&self, id: i32, hub_id: i32) -> RepositoryResult<Option<Crawler>> {
        use pushkind_common::schema::dantes::crawlers;

        let mut conn = self.conn()?;

        let result = crawlers::table
            .filter(crawlers::id.eq(id))
            .filter(crawlers::hub_id.eq(hub_id))
            .first::<DbCrawler>(&mut conn)
            .optional()?;

        Ok(result.map(|db_crawler| db_crawler.into())) // Convert DbCrawler to DomainCrawler
    }
}
