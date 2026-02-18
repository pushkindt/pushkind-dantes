use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize};

/// Metadata about a crawler job and its progress.
#[derive(Debug, Clone, Serialize, Deserialize)]
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
