-- This file should undo anything in `up.sql`
ALTER TABLE benchmarks DROP COLUMN embedding;
ALTER TABLE products DROP COLUMN embedding;
