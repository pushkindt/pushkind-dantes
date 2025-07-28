use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Crawler {
    pub id: i32,
    pub name: String,
    pub url: String,
    pub selector: String,
    pub processing: bool,
    pub updated_at: NaiveDateTime,
}
