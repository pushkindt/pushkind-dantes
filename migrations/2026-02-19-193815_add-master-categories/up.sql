CREATE TABLE categories (
    id INTEGER NOT NULL PRIMARY KEY,
    hub_id INTEGER NOT NULL,
    name TEXT NOT NULL,
    embedding BLOB,
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE UNIQUE INDEX idx_categories_hub_id_name_ci ON categories(hub_id, LOWER(name));

ALTER TABLE products
    ADD COLUMN category_id INTEGER REFERENCES categories(id) ON DELETE SET NULL;

ALTER TABLE products
    ADD COLUMN category_assignment_source TEXT NOT NULL DEFAULT 'automatic'
    CHECK (category_assignment_source IN ('automatic', 'manual'));

CREATE INDEX idx_products_category_id ON products(category_id);
