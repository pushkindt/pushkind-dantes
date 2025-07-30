-- This file should undo anything in `up.sql`
DROP TABLE product_benchmark;
ALTER TABLE crawlers DROP COLUMN num_products;
