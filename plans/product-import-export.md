# Plan: Product Import/Export (Crawler + Benchmark)

Status: stable  
Feature spec: `specs/features/product-import-export.md`

## 1. Scope

Implement import/export workflows defined in the feature spec for:

- crawler products (per crawler),
- benchmark catalog rows (hub-wide),
- explicit `format=csv|xlsx` and `mode=full|partial`,
- nullable `products.url`,
- product embedding invalidation on update.

Out of scope:

- delete-by-absence semantics,
- import/export for embeddings, processing flags, timestamps, images, and counters.

## 2. Technical Decisions

- Keep transport as HTML routes and multipart uploads.
- Keep per-row partial-apply behavior with row-level error reporting.
- Use explicit enums for mode/format parsing and validation.
- Use `csv` crate for CSV parsing/serialization.
- Add `calamine` for XLSX read.
- Add `rust_xlsxwriter` for XLSX write.
- Keep FTS objects untouched in migration SQL (no DDL against `products_fts` or its triggers).

## 3. Work Breakdown

### Phase 1: Data Layer and Migration

- Add migration to make `products.url` nullable.
- Add indexes:
  - `idx_products_crawler_sku` on `(crawler_id, sku)`,
  - `idx_benchmarks_hub_sku` on `(hub_id, sku)`.
- Ensure migration does not create/drop/alter/rebuild:
  - `products_fts`,
  - `products_ai`, `products_au`, `products_ad`.
- Update `src/schema.rs` to `url -> Nullable<Text>` for `products`.

### Phase 2: Domain/Model/Repository Contracts

- Change domain:
  - `Product.url: Option<ProductUrl>`,
  - `NewProduct.url: Option<ProductUrl>`.
- Change Diesel models:
  - `models::product::Product.url: Option<String>`,
  - `models::product::NewProduct.url: Option<String>`.
- Extend repository traits:
  - product create/update methods for import upsert,
  - benchmark update method for import upsert.
- Implement repository writes so product updates always set `embedding = NULL`.
- Add repository read helpers for SKU lookups in scope:
  - product by `(crawler_id, sku)`,
  - benchmark by `(hub_id, sku)`,
  - with duplicate detection.

### Phase 3: Form and File Parsing Layer

- Add new forms module for import/export payloads (products and benchmarks):
  - multipart payload with `file`, `format`, `mode`,
  - typed enums `UploadFormat` and `UploadMode`.
- Add parser layer that normalizes CSV/XLSX rows into a shared row representation.
- Implement strict header validation:
  - full mode requires full header set,
  - partial mode requires `sku` + allowed mutable business columns.
- Implement cell semantics:
  - omitted field => unchanged,
  - empty cell => `NULL` for nullable fields,
  - empty cell on non-nullable field => row error.

### Phase 4: Service Logic

- Add upload report types:
  - totals: `total`, `created`, `updated`, `skipped`,
  - row errors with row number, optional SKU, and message.
- Implement crawler product upload service:
  - scoped by crawler id and hub ownership,
  - full/partial SKU upsert,
  - file-level duplicate SKU detection,
  - DB duplicate SKU conflict detection,
  - partial apply behavior.
- Implement benchmark upload service:
  - extend existing `/benchmarks/upload`,
  - same mode/format/conflict/report semantics in hub scope.
- Implement download services:
  - crawler product export columns: `sku,name,category,units,price,amount,description,url`,
  - benchmark export columns: `sku,name,category,units,price,amount,description`.
- Ensure product update ZMQ workflows filter out `None` URLs; if empty list remains, do not dispatch.

### Phase 5: Routes and Templates

- Add routes:
  - `GET /crawler/{crawler_id}/products/download`,
  - `POST /crawler/{crawler_id}/products/upload`,
  - `GET /benchmarks/download`,
  - extend `POST /benchmarks/upload`.
- Keep role/hub checks in service layer.
- Update templates:
  - crawler page actions menu: upload/download controls,
  - benchmark page/modal: upload/download controls with format/mode selectors,
  - render upload summary + row error table on relevant pages.

### Phase 6: Tests

- Unit tests:
  - mode/format parsing,
  - CSV/XLSX parsing,
  - header validation full vs partial,
  - empty-cell semantics,
  - file duplicate SKU detection.
- Service tests (with `TestRepository`):
  - crawler full/partial upsert paths,
  - benchmark full/partial upsert paths,
  - unknown SKU create rules in partial mode,
  - DB duplicate SKU conflict behavior,
  - product update clears embedding.
- Integration tests:
  - migration allows `products.url = NULL`,
  - unique non-null URL behavior preserved,
  - download content headers and payload columns,
  - ZMQ update path skips null URLs and avoids empty dispatch.

### Phase 7: Documentation and Verification

- Update `SPEC.md` to reflect new routes, upload contract, nullable product URL, and embedding invalidation semantics.
- Keep feature spec and plan aligned after implementation.
- Run quality gates:
  - `cargo build --all-features --verbose`
  - `cargo test --all-features --verbose`
  - `cargo clippy --all-features --tests -- -Dwarnings`
  - `cargo fmt --all -- --check`

## 4. Acceptance Mapping

- All feature acceptance criteria in `specs/features/product-import-export.md` are implemented and validated by tests.
- No migration DDL touches `products_fts` or `products_ai/products_au/products_ad`.
- Any update of an existing crawler product row sets `products.embedding = NULL`.
