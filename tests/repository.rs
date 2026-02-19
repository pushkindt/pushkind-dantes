use chrono::Utc;
use diesel::prelude::*;
use pushkind_dantes::domain::category::NewCategory;
use pushkind_dantes::domain::types::{
    CategoryAssignmentSource, CategoryName, HubId, ProductId, ProductUrl,
};
use pushkind_dantes::repository::{
    CategoryListQuery, CategoryReader, CategoryWriter, DieselRepository, ProductWriter,
};
use pushkind_dantes::schema::products;

mod common;

#[test]
fn test_user_repository_crud() {
    let test_db = common::TestDb::new();
    let _repo = DieselRepository::new(test_db.pool());
}

#[test]
fn delete_category_resets_linked_products_to_automatic() {
    let test_db = common::TestDb::new();
    let repo = DieselRepository::new(test_db.pool());

    let hub_id = HubId::new(1).expect("valid hub id");
    let now = Utc::now().naive_utc();
    let new_category = NewCategory {
        hub_id,
        name: CategoryName::new("Tea/Green/Sencha".to_string()).expect("valid category name"),
        embedding: None,
        created_at: now,
        updated_at: now,
    };
    repo.create_category(&new_category)
        .expect("should create category");

    let (_, categories) = repo
        .list_categories(CategoryListQuery::new(hub_id))
        .expect("should list categories");
    let category = categories
        .into_iter()
        .find(|c| c.name.as_str() == "Tea/Green/Sencha")
        .expect("inserted category should exist");

    let product_url =
        ProductUrl::new("https://example.com/product-1".to_string()).expect("valid product url");

    let mut conn = test_db
        .pool()
        .get()
        .expect("should acquire DB connection for setup");
    diesel::insert_into(products::table)
        .values((
            products::crawler_id.eq(1),
            products::name.eq("Test Product"),
            products::sku.eq("SKU-1"),
            products::price.eq(123.45_f64),
            products::url.eq(product_url.as_str()),
        ))
        .execute(&mut conn)
        .expect("should create product");

    let product_id: i32 = products::table
        .filter(products::url.eq(product_url.as_str()))
        .select(products::id)
        .first(&mut conn)
        .expect("inserted product id should be readable");
    let product_id = ProductId::new(product_id).expect("valid product id");

    repo.set_product_category_manual(product_id, category.id)
        .expect("should set manual assignment");
    repo.delete_category(category.id, hub_id)
        .expect("should delete category");

    let row: (Option<i32>, String) = products::table
        .filter(products::id.eq(product_id.get()))
        .select((products::category_id, products::category_assignment_source))
        .first(&mut conn)
        .expect("product should remain after category deletion");

    assert_eq!(row.0, None);
    assert_eq!(row.1, CategoryAssignmentSource::Automatic.as_str());
}
