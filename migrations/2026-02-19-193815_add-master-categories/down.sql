DROP INDEX idx_products_category_id;
ALTER TABLE products DROP COLUMN category_id;
ALTER TABLE products DROP COLUMN category_assignment_source;

DROP INDEX idx_categories_hub_id_name_ci;
DROP TABLE categories;
