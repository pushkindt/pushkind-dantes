-- Keep FTS objects untouched; only adjust table SQL metadata and add indexes.
PRAGMA foreign_keys=OFF;
PRAGMA writable_schema=ON;

UPDATE sqlite_master
SET sql = replace(
    replace(sql, 'url TEXT NOT NULL,', 'url TEXT,'),
    'url TEXT NOT NULL',
    'url TEXT'
)
WHERE type = 'table'
  AND name = 'products'
  AND sql LIKE '%url TEXT NOT NULL%';

PRAGMA schema_version = 20260224;
PRAGMA writable_schema=OFF;
PRAGMA foreign_keys=ON;

CREATE INDEX IF NOT EXISTS idx_products_crawler_sku ON products(crawler_id, sku);
CREATE INDEX IF NOT EXISTS idx_benchmarks_hub_sku ON benchmarks(hub_id, sku);
