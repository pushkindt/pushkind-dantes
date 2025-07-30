use chrono::NaiveDateTime;
use serde::Serialize;

#[derive(Serialize)]
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
