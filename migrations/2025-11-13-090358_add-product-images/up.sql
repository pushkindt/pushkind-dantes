-- Your SQL goes here
CREATE TABLE product_images (
    id INTEGER NOT NULL PRIMARY KEY,
    product_id INTEGER NOT NULL REFERENCES products(id),
    url TEXT NOT NULL
);
