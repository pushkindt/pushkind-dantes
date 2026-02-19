use crate::domain::category::Category;
use serde::Serialize;

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct CategoryDto {
    pub id: i32,
    pub name: String,
}

impl From<Category> for CategoryDto {
    fn from(value: Category) -> Self {
        Self {
            id: value.id.get(),
            name: value.name.as_str().to_string(),
        }
    }
}
