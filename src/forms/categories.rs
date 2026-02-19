use chrono::Utc;
use serde::Deserialize;
use thiserror::Error;
use validator::{Validate, ValidationErrors};

use crate::domain::category::NewCategory;
use crate::domain::types::{CategoryId, CategoryName, HubId, ProductId, TypeConstraintError};

fn normalize_category_path(value: String) -> Result<String, TypeConstraintError> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Err(TypeConstraintError::EmptyString("category"));
    }

    let mut normalized_parts = Vec::new();
    for part in trimmed.split('/') {
        let part = part.trim();
        if part.is_empty() {
            return Err(TypeConstraintError::InvalidValue(
                "category path contains empty segments".to_string(),
            ));
        }
        normalized_parts.push(part);
    }

    Ok(normalized_parts.join("/"))
}

#[derive(Deserialize, Validate)]
pub struct AddCategoryForm {
    #[validate(length(min = 1))]
    pub name: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct AddCategoryFormPayload {
    pub name: CategoryName,
}

impl AddCategoryFormPayload {
    pub fn into_new_category(self, hub_id: HubId) -> NewCategory {
        let now = Utc::now().naive_utc();
        NewCategory {
            hub_id,
            name: self.name,
            // Embedding is generated asynchronously by pushkind-crawlers.
            embedding: None,
            created_at: now,
            updated_at: now,
        }
    }
}

#[derive(Debug, Error)]
pub enum AddCategoryFormError {
    #[error("Add category form validation failed: {0}")]
    Validation(String),
    #[error("Add category form contains invalid data: {0}")]
    TypeConstraint(String),
}

impl From<ValidationErrors> for AddCategoryFormError {
    fn from(value: ValidationErrors) -> Self {
        Self::Validation(value.to_string())
    }
}

impl From<TypeConstraintError> for AddCategoryFormError {
    fn from(value: TypeConstraintError) -> Self {
        Self::TypeConstraint(value.to_string())
    }
}

impl TryFrom<AddCategoryForm> for AddCategoryFormPayload {
    type Error = AddCategoryFormError;

    fn try_from(value: AddCategoryForm) -> Result<Self, Self::Error> {
        value.validate()?;
        let normalized_name = normalize_category_path(value.name)?;

        Ok(Self {
            name: CategoryName::new(normalized_name)?,
        })
    }
}

#[derive(Deserialize, Validate)]
pub struct UpdateCategoryForm {
    #[validate(range(min = 1))]
    pub category_id: i32,
    #[validate(length(min = 1))]
    pub name: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct UpdateCategoryFormPayload {
    pub category_id: CategoryId,
    pub name: CategoryName,
    pub embedding: Option<Vec<u8>>,
}

#[derive(Debug, Error)]
pub enum UpdateCategoryFormError {
    #[error("Update category form validation failed: {0}")]
    Validation(String),
    #[error("Update category form contains invalid data: {0}")]
    TypeConstraint(String),
}

impl From<ValidationErrors> for UpdateCategoryFormError {
    fn from(value: ValidationErrors) -> Self {
        Self::Validation(value.to_string())
    }
}

impl From<TypeConstraintError> for UpdateCategoryFormError {
    fn from(value: TypeConstraintError) -> Self {
        Self::TypeConstraint(value.to_string())
    }
}

impl TryFrom<UpdateCategoryForm> for UpdateCategoryFormPayload {
    type Error = UpdateCategoryFormError;

    fn try_from(value: UpdateCategoryForm) -> Result<Self, Self::Error> {
        value.validate()?;
        let normalized_name = normalize_category_path(value.name)?;

        Ok(Self {
            category_id: CategoryId::new(value.category_id)?,
            name: CategoryName::new(normalized_name)?,
            // Embedding is regenerated asynchronously by pushkind-crawlers.
            embedding: None,
        })
    }
}

#[derive(Deserialize, Validate)]
pub struct DeleteCategoryForm {
    #[validate(range(min = 1))]
    pub category_id: i32,
}

#[derive(Debug, Clone, PartialEq)]
pub struct DeleteCategoryFormPayload {
    pub category_id: CategoryId,
}

#[derive(Debug, Error)]
pub enum DeleteCategoryFormError {
    #[error("Delete category form validation failed: {0}")]
    Validation(String),
    #[error("Delete category form contains invalid data: {0}")]
    TypeConstraint(String),
}

impl From<ValidationErrors> for DeleteCategoryFormError {
    fn from(value: ValidationErrors) -> Self {
        Self::Validation(value.to_string())
    }
}

impl From<TypeConstraintError> for DeleteCategoryFormError {
    fn from(value: TypeConstraintError) -> Self {
        Self::TypeConstraint(value.to_string())
    }
}

impl TryFrom<DeleteCategoryForm> for DeleteCategoryFormPayload {
    type Error = DeleteCategoryFormError;

    fn try_from(value: DeleteCategoryForm) -> Result<Self, Self::Error> {
        value.validate()?;
        Ok(Self {
            category_id: CategoryId::new(value.category_id)?,
        })
    }
}

#[derive(Deserialize, Validate)]
pub struct SetProductCategoryForm {
    #[validate(range(min = 1))]
    pub product_id: i32,
    #[validate(range(min = 1))]
    pub category_id: i32,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SetProductCategoryFormPayload {
    pub product_id: ProductId,
    pub category_id: CategoryId,
}

#[derive(Debug, Error)]
pub enum SetProductCategoryFormError {
    #[error("Set product category form validation failed: {0}")]
    Validation(String),
    #[error("Set product category form contains invalid data: {0}")]
    TypeConstraint(String),
}

impl From<ValidationErrors> for SetProductCategoryFormError {
    fn from(value: ValidationErrors) -> Self {
        Self::Validation(value.to_string())
    }
}

impl From<TypeConstraintError> for SetProductCategoryFormError {
    fn from(value: TypeConstraintError) -> Self {
        Self::TypeConstraint(value.to_string())
    }
}

impl TryFrom<SetProductCategoryForm> for SetProductCategoryFormPayload {
    type Error = SetProductCategoryFormError;

    fn try_from(value: SetProductCategoryForm) -> Result<Self, Self::Error> {
        value.validate()?;
        Ok(Self {
            product_id: ProductId::new(value.product_id)?,
            category_id: CategoryId::new(value.category_id)?,
        })
    }
}

#[derive(Deserialize, Validate)]
pub struct ClearProductCategoryForm {
    #[validate(range(min = 1))]
    pub product_id: i32,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ClearProductCategoryFormPayload {
    pub product_id: ProductId,
}

#[derive(Debug, Error)]
pub enum ClearProductCategoryFormError {
    #[error("Clear product category form validation failed: {0}")]
    Validation(String),
    #[error("Clear product category form contains invalid data: {0}")]
    TypeConstraint(String),
}

impl From<ValidationErrors> for ClearProductCategoryFormError {
    fn from(value: ValidationErrors) -> Self {
        Self::Validation(value.to_string())
    }
}

impl From<TypeConstraintError> for ClearProductCategoryFormError {
    fn from(value: TypeConstraintError) -> Self {
        Self::TypeConstraint(value.to_string())
    }
}

impl TryFrom<ClearProductCategoryForm> for ClearProductCategoryFormPayload {
    type Error = ClearProductCategoryFormError;

    fn try_from(value: ClearProductCategoryForm) -> Result<Self, Self::Error> {
        value.validate()?;
        Ok(Self {
            product_id: ProductId::new(value.product_id)?,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn add_category_normalizes_path_segments() {
        let form = AddCategoryForm {
            name: " Tea / Green / Sencha ".to_string(),
        };

        let payload: AddCategoryFormPayload = form.try_into().unwrap();
        assert_eq!(payload.name.as_str(), "Tea/Green/Sencha");
    }

    #[test]
    fn add_category_rejects_empty_segments() {
        let form = AddCategoryForm {
            name: "Tea//Sencha".to_string(),
        };

        let payload: Result<AddCategoryFormPayload, _> = form.try_into();
        assert!(payload.is_err());
    }

    #[test]
    fn set_product_category_form_validates_ids() {
        let form = SetProductCategoryForm {
            product_id: 1,
            category_id: 2,
        };
        let payload: SetProductCategoryFormPayload = form.try_into().unwrap();
        assert_eq!(payload.product_id.get(), 1);
        assert_eq!(payload.category_id.get(), 2);
    }
}
