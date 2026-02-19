use chrono::NaiveDateTime;
use diesel::prelude::*;

use crate::domain::crawler::Crawler as DomainCrawler;
use crate::domain::types::{
    CrawlerName, CrawlerSelectorValue, CrawlerUrl, ProductCount, TypeConstraintError,
};

/// Diesel representation of a crawler row.
#[derive(Debug, Clone, Identifiable, Queryable)]
#[diesel(table_name = crate::schema::crawlers)]
pub struct Crawler {
    pub id: i32,
    pub hub_id: i32,
    pub name: String,
    pub url: String,
    pub selector: String,
    pub processing: bool,
    pub updated_at: NaiveDateTime,
    pub num_products: i32,
}

impl TryFrom<Crawler> for DomainCrawler {
    type Error = TypeConstraintError;

    fn try_from(crawler: Crawler) -> Result<Self, Self::Error> {
        Ok(DomainCrawler {
            id: crawler.id.try_into()?,
            hub_id: crawler.hub_id.try_into()?,
            name: CrawlerName::new(crawler.name)?,
            url: CrawlerUrl::new(crawler.url)?,
            selector: CrawlerSelectorValue::new(crawler.selector)?,
            processing: crawler.processing,
            updated_at: crawler.updated_at,
            num_products: ProductCount::new(crawler.num_products)?,
        })
    }
}
