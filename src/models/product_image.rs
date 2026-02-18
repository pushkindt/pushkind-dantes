use diesel::prelude::*;

/// Diesel model representing the `product_images` table.
#[derive(Debug, Clone, Identifiable, Queryable, QueryableByName)]
#[diesel(table_name = crate::schema::product_images)]
#[diesel(foreign_derive)]
pub struct ProductImage {
    pub id: i32,
    pub product_id: i32,
    pub url: String,
}

/// Insertable/patchable form of [`ProductImage`].
#[derive(Debug, Insertable, AsChangeset)]
#[diesel(table_name = crate::schema::product_images)]
pub struct NewProductImage {
    pub product_id: i32,
    pub url: String,
}
