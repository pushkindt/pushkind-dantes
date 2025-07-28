use diesel::prelude::*;

use crate::domain::crawler::Crawler as DomainCrawler;

#[derive(Debug, Clone, Identifiable, Queryable)]
#[diesel(table_name = crate::schema::crawlers)]
pub struct Crawler {
    pub id: i32,
    pub name: String,
    pub url: String,
    pub selector: String,
    pub processing: bool,
    pub updated_at: chrono::NaiveDateTime,
}

impl From<Crawler> for DomainCrawler {
    fn from(crawler: Crawler) -> Self {
        DomainCrawler {
            id: crawler.id,
            name: crawler.name,
            url: crawler.url,
            selector: crawler.selector,
            processing: crawler.processing,
            updated_at: crawler.updated_at,
        }
    }
}
