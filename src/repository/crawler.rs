use diesel::prelude::*;
use pushkind_common::repository::errors::RepositoryResult;

use crate::domain::crawler::Crawler;
use crate::models::crawler::Crawler as DbCrawler;
use crate::repository::{CrawlerReader, DieselRepository};

impl CrawlerReader for DieselRepository {
    fn list_crawlers(&self, hub_id: i32) -> RepositoryResult<Vec<Crawler>> {
        use crate::schema::crawlers;

        let mut conn = self.conn()?;

        let results = crawlers::table
            .filter(crawlers::hub_id.eq(hub_id))
            .order(crawlers::id.asc())
            .get_results::<DbCrawler>(&mut conn)?;

        let results = results
            .into_iter()
            .map(TryInto::try_into)
            .collect::<Result<Vec<Crawler>, _>>()?;
        Ok(results)
    }

    fn get_crawler_by_id(&self, id: i32, hub_id: i32) -> RepositoryResult<Option<Crawler>> {
        use crate::schema::crawlers;

        let mut conn = self.conn()?;

        let result = crawlers::table
            .filter(crawlers::id.eq(id))
            .filter(crawlers::hub_id.eq(hub_id))
            .first::<DbCrawler>(&mut conn)
            .optional()?;

        let result = result.map(TryInto::try_into).transpose()?;
        Ok(result)
    }
}
