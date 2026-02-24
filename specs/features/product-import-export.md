# Feature Spec: Product Import/Export (Crawler + Benchmark)

Status: stable  
Created: 2026-02-24  
Related: `SPEC.md`

## 1. Summary

Add bidirectional import/export for product-like data in two scopes:

- crawler products (per crawler),
- benchmark catalog rows (hub-wide).

Uploads must support `full` and `partial` modes with explicit `mode` selection.
Uploads/downloads must support both CSV and XLSX with explicit `format` selection.

`products.url` becomes nullable. URL-dependent update jobs must skip products without URL.

## 2. Goals

- Enable parser users to download and upload crawler products per crawler.
- Enable parser users to download and upload benchmark rows per hub.
- Support partial SKU-based upsert workflows.
- Keep hub and crawler scoping guarantees intact.
- Preserve existing asynchronous ZMQ update workflows while tolerating missing product URLs.

## 3. Non-Goals

- No deletion/synchronization by omission in uploaded files.
- No embedding, processing-flag, image, timestamp, or count import/export.
- No public JSON API for import/export in V1 (HTML workflows only).

## 4. Scope and Routes

### 4.1 Crawler products (per crawler)

- `GET /crawler/{crawler_id}/products/download?format=csv|xlsx`
- `POST /crawler/{crawler_id}/products/upload`

Scope: only products of the route crawler.

### 4.2 Benchmark catalog (hub-wide)

- `GET /benchmarks/download?format=csv|xlsx`
- `POST /benchmarks/upload`

Scope: all benchmark rows in current user hub.

### 4.3 Authorization

- Required role: `parser`.
- All reads/writes are hub-scoped via authenticated user context.

## 5. Upload Contract

Multipart fields:

- `file` (required, max 10MB)
- `format` (required): `csv|xlsx`
- `mode` (required): `full|partial`

Validation rules:

- `format` must match filename extension and provided content type (when present).
- CSV is UTF-8, comma-delimited, header-based.
- XLSX reads first worksheet with first row as headers.

## 6. Export Schema

Internal IDs are not exported.

Crawler product export columns:

- `sku,name,category,units,price,amount,description,url`

Benchmark export columns:

- `sku,name,category,units,price,amount,description`

## 7. Upload Semantics

## 7.1 Common behavior

- Matching key is SKU scoped by entity context:
  - crawler products: `(crawler_id, sku)`
  - benchmarks: `(hub_id, sku)`
- Upload strategy is partial apply:
  - valid rows are persisted,
  - invalid/conflicting rows are skipped.
- UI must show flash summary + row-level error table.

## 7.2 Full mode

- Requires full header set for target entity.
- Each row performs SKU upsert (create or update).
- For crawler products, when an existing row is updated, `products.embedding` must be set to `NULL` in the same write.

## 7.3 Partial mode

- `sku` is required.
- Any mutable business columns are allowed.
- Omitted columns keep existing values.
- Present empty cell semantics:
  - nullable fields: clear to `NULL`,
  - non-nullable fields: row validation error.
- Unknown SKU:
  - create only if row includes all required non-nullable fields,
  - otherwise row validation error.
- For crawler products, when an existing row is updated, `products.embedding` must be set to `NULL` in the same write.

## 7.4 Conflict handling

- File conflict: duplicate SKU rows within one upload => row conflict.
- DB conflict: if multiple existing rows already match same scoped SKU => row conflict.
- Conflicted rows are skipped and reported.

## 8. Data Model Changes

## 8.1 `products.url` optional

- Change `products.url` from `TEXT NOT NULL` to nullable `TEXT`.
- Keep unique index on `(products.crawler_id, products.url)`.
  - Non-null URL uniqueness remains enforced.
  - Multiple `NULL` URLs are allowed by SQLite unique-index behavior.

## 8.2 Supporting indexes

Add non-unique indexes for upload lookup performance:

- `products(crawler_id, sku)`
- `benchmarks(hub_id, sku)`

## 8.3 SQLite migration constraints

Migration must not touch FTS artifacts tied to `products` in any way:

- do not create/drop/alter/rebuild `products_fts`,
- do not create/drop/alter/rebuild `products_ai/products_au/products_ad` triggers,
- the `products.url` nullability migration must be implemented without any DDL changes to those FTS objects.

## 9. Domain and Service Behavior Changes

- Product URL domain field becomes optional.
- Product URL in create/update flows becomes optional.
- Any crawler product update must invalidate stale embeddings by writing `products.embedding = NULL`.
- URL-dependent ZMQ update emissions (`SelectorProducts`) must filter out products with `NULL` URL.
- If no URLs remain after filtering, no update message is emitted and user receives informative result.

## 10. Error Reporting and UX

Upload response data should include:

- total rows,
- created count,
- updated count,
- skipped count,
- row-level errors with row number, optional SKU, and message.

Route/template behavior:

- success-only uploads may redirect with summary flash,
- uploads with row errors must render summary and detailed error table on the relevant page.

## 11. Acceptance Criteria

- Crawler products can be downloaded/uploaded in CSV and XLSX per crawler.
- Benchmarks can be downloaded/uploaded in CSV and XLSX per hub.
- Full and partial mode behavior matches sections 7.2 and 7.3.
- SKU conflicts (file-level and DB-level) are detected and reported.
- `products.url` is nullable without breaking list/search/render flows.
- Migration leaves `products_fts` and its triggers unchanged.
- Updating any existing crawler product clears `products.embedding` to `NULL`.
- URL-based update jobs skip null URLs and avoid empty dispatch payloads.
- Export files contain only business columns and no internal IDs.
