-- Your SQL goes here
ALTER TABLE crawlers ADD COLUMN num_products INTEGER NOT NULL DEFAULT 0;

CREATE TABLE product_benchmark (
    product_id INTEGER NOT NULL REFERENCES products(id),
    benchmark_id INTEGER NOT NULL REFERENCES benchmarks(id),
    PRIMARY KEY (product_id, benchmark_id)
);
