use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize};

use crate::domain::types::{
    CategoryAssignmentSource, CategoryId, CategoryName, CrawlerId, ImageUrl, ProductAmount,
    ProductDescription, ProductId, ProductName, ProductPrice, ProductSku, ProductUnits, ProductUrl,
};

/// A product extracted from a crawler run.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Product {
    pub id: ProductId,
    pub crawler_id: CrawlerId,
    pub name: ProductName,
    pub sku: ProductSku,
    /// Original category extracted from source data.
    pub category: Option<CategoryName>,
    /// Canonical category associated via `category_id`.
    pub associated_category: Option<CategoryName>,
    pub units: Option<ProductUnits>,
    pub price: ProductPrice,
    pub amount: Option<ProductAmount>,
    pub description: Option<ProductDescription>,
    pub url: ProductUrl,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
    pub embedding: Option<Vec<u8>>,
    pub category_id: Option<CategoryId>,
    pub category_assignment_source: CategoryAssignmentSource,
    pub images: Vec<ImageUrl>,
}

/// Information required to create a new [`Product`].
#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
pub struct NewProduct {
    pub crawler_id: CrawlerId,
    pub name: ProductName,
    pub sku: ProductSku,
    pub category: Option<CategoryName>,
    pub units: Option<ProductUnits>,
    pub price: ProductPrice,
    pub amount: Option<ProductAmount>,
    pub description: Option<ProductDescription>,
    pub url: ProductUrl,
    pub images: Vec<ImageUrl>,
}
