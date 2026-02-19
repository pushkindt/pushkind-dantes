use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize};

use crate::domain::types::{
    CrawlerId, CrawlerName, CrawlerSelectorValue, CrawlerUrl, HubId, ProductCount,
};

/// Metadata about a crawler job and its progress.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Crawler {
    pub id: CrawlerId,
    pub hub_id: HubId,
    pub name: CrawlerName,
    pub url: CrawlerUrl,
    pub selector: CrawlerSelectorValue,
    pub processing: bool,
    pub updated_at: NaiveDateTime,
    pub num_products: ProductCount,
}
