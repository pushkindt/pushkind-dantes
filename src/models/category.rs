use chrono::NaiveDateTime;
use diesel::prelude::*;

use crate::domain::category::{Category as DomainCategory, NewCategory as DomainNewCategory};
use crate::domain::types::{CategoryName, TypeConstraintError};

/// Diesel model representing the `categories` table.
#[derive(Debug, Clone, Identifiable, Queryable)]
#[diesel(table_name = crate::schema::categories)]
pub struct Category {
    pub id: i32,
    pub hub_id: i32,
    pub name: String,
    pub embedding: Option<Vec<u8>>,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
}

/// Insertable/patchable form of [`Category`].
#[derive(Debug, Insertable, AsChangeset)]
#[diesel(table_name = crate::schema::categories)]
pub struct NewCategory {
    pub hub_id: i32,
    pub name: String,
    pub embedding: Option<Vec<u8>>,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
}

impl TryFrom<Category> for DomainCategory {
    type Error = TypeConstraintError;

    fn try_from(category: Category) -> Result<Self, Self::Error> {
        Ok(Self {
            id: category.id.try_into()?,
            hub_id: category.hub_id.try_into()?,
            name: CategoryName::new(category.name)?,
            embedding: category.embedding,
            created_at: category.created_at,
            updated_at: category.updated_at,
        })
    }
}

impl From<DomainNewCategory> for NewCategory {
    fn from(category: DomainNewCategory) -> Self {
        Self {
            hub_id: category.hub_id.get(),
            name: category.name.into_inner(),
            embedding: category.embedding,
            created_at: category.created_at,
            updated_at: category.updated_at,
        }
    }
}
