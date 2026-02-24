-- Revert nullability metadata and drop added lookup indexes.
UPDATE products
SET url = 'https://missing.local/product/' || id
WHERE url IS NULL;

PRAGMA foreign_keys=OFF;
PRAGMA writable_schema=ON;

UPDATE sqlite_master
SET sql = replace(
    replace(sql, 'url TEXT,', 'url TEXT NOT NULL,'),
    'url TEXT)',
    'url TEXT NOT NULL)'
)
WHERE type = 'table'
  AND name = 'products'
  AND (sql LIKE '%url TEXT,%' OR sql LIKE '%url TEXT)%');

PRAGMA schema_version = 20260225;
PRAGMA writable_schema=OFF;
PRAGMA foreign_keys=ON;

DROP INDEX IF EXISTS idx_products_crawler_sku;
DROP INDEX IF EXISTS idx_benchmarks_hub_sku;
