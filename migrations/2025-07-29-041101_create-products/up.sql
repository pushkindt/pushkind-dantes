-- Your SQL goes here
CREATE TABLE benchmarks (
    id INTEGER NOT NULL PRIMARY KEY,
    hub_id INTEGER NOT NULL,
    name TEXT NOT NULL,
    sku TEXT NOT NULL,
    category TEXT NOT NULL,
    units TEXT NOT NULL,
    price DECIMAL(10, 2) NOT NULL,
    amount DECIMAL(10, 2) NOT NULL,
    description TEXT NOT NULL,
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE products (
    id INTEGER NOT NULL PRIMARY KEY,
    crawler_id INTEGER NOT NULL REFERENCES crawlers(id),
    name TEXT NOT NULL,
    sku TEXT NOT NULL,
    category TEXT,
    units TEXT,
    price DECIMAL(10, 2) NOT NULL,
    amount DECIMAL(10, 2),
    description TEXT,
    url TEXT NOT NULL,
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);
