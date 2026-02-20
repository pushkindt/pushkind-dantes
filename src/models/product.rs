use chrono::NaiveDateTime;
use diesel::prelude::*;

use crate::domain::product::{NewProduct as DomainNewProduct, Product as DomainProduct};
use crate::domain::types::{
    CategoryAssignmentSource, CategoryId, CategoryName, ProductAmount, ProductDescription,
    ProductName, ProductPrice, ProductSku, ProductUnits, ProductUrl, TypeConstraintError,
};

/// Diesel model representing the `products` table.
#[derive(Debug, Clone, Identifiable, Queryable, QueryableByName)]
#[diesel(table_name = crate::schema::products)]
#[diesel(foreign_derive)]
pub struct Product {
    pub id: i32,
    pub crawler_id: i32,
    pub name: String,
    pub sku: String,
    pub category: Option<String>,
    pub units: Option<String>,
    pub price: f64,
    pub amount: Option<f64>,
    pub description: Option<String>,
    pub url: String,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
    pub embedding: Option<Vec<u8>>,
    pub category_id: Option<i32>,
    pub category_assignment_source: String,
}

/// Insertable/patchable form of [`Product`].
#[derive(Debug, Insertable, AsChangeset)]
#[diesel(table_name = crate::schema::products)]
pub struct NewProduct {
    pub crawler_id: i32,
    pub name: String,
    pub sku: String,
    pub category: Option<String>,
    pub units: Option<String>,
    pub price: f64,
    pub amount: Option<f64>,
    pub description: Option<String>,
    pub url: String,
}

impl TryFrom<Product> for DomainProduct {
    type Error = TypeConstraintError;

    fn try_from(product: Product) -> Result<Self, Self::Error> {
        Ok(Self {
            id: product.id.try_into()?,
            crawler_id: product.crawler_id.try_into()?,
            name: ProductName::new(product.name)?,
            sku: ProductSku::new(product.sku)?,
            category: product.category.map(CategoryName::new).transpose()?,
            associated_category: None,
            units: product.units.map(ProductUnits::new).transpose()?,
            price: ProductPrice::new(product.price)?,
            amount: product.amount.map(ProductAmount::new).transpose()?,
            description: product
                .description
                .map(ProductDescription::new)
                .transpose()?,
            url: ProductUrl::new(product.url)?,
            created_at: product.created_at,
            updated_at: product.updated_at,
            embedding: product.embedding,
            category_id: product.category_id.map(CategoryId::new).transpose()?,
            category_assignment_source: CategoryAssignmentSource::try_from(
                product.category_assignment_source,
            )?,
            images: vec![],
        })
    }
}

impl From<DomainNewProduct> for NewProduct {
    fn from(product: DomainNewProduct) -> Self {
        Self {
            crawler_id: product.crawler_id.get(),
            name: product.name.into_inner(),
            sku: product.sku.into_inner(),
            category: product.category.map(Into::into),
            units: product.units.map(Into::into),
            price: product.price.get(),
            amount: product.amount.map(ProductAmount::get),
            description: product.description.map(Into::into),
            url: product.url.into_inner(),
        }
    }
}
