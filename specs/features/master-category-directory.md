# Feature Spec: Master Category Directory

Status: stable  
Created: 2026-02-19  
Related: `SPEC.md`, `plans/master-category-directory.md`, `specs/decisions/0001-master-category-directory.md`

## 1. Summary

Introduce a per-hub master category directory as the canonical source of product categories.
Users with parser access must be able to create, read, update, and delete categories.
Each category has at minimum:

- `id`
- `hub_id`
- `name`

Optional field:

- `embedding`

Category names are flat strings and may represent hierarchy using slash-separated path segments (for example `Tea/Green/Sencha`), but no database tree structure is introduced.

Products gain an optional link to this directory through `products.category_id`.
The app can enqueue a ZeroMQ job instructing crawler workers to assign exactly one most relevant category from the directory to each product.
Users can manually override per-product category assignments, and automatic matching must not overwrite manual overrides.

## 2. Goals

- Add a canonical category directory managed from this application and isolated per hub.
- Support full CRUD operations for category records.
- Store optional category embeddings alongside category names.
- Allow products to reference canonical categories via optional foreign key.
- Support manual per-product category assignment overrides.
- Preserve manual overrides across subsequent automatic matching runs.

## 3. Non-Goals (V1)

- No database-level parent/child tree for categories.
- No automatic category import/sync from external taxonomy providers.
- No replacement of existing benchmark category text behavior in this iteration.
- No in-repo implementation of category matching workers or matching algorithms (owned by `pushkind-crawlers`).

## 4. Domain Rules

### 4.1 Category Name

- Required and non-empty after normalization.
- The full value and each slash-delimited segment are trimmed.
- Uses slash `/` as a path separator convention only.
- No leading or trailing slash in normalized value.
- Empty segments are invalid (`Tea//Green` is invalid).
- Normalized storage joins trimmed segments with `/` (no surrounding spaces).
- Category name must be unique within a hub (case-insensitive uniqueness in DB).

### 4.2 Category Embedding

- Stored as binary payload (`BLOB`) compatible with existing embedding storage patterns.
- Optional field at entity level.
- On category create/update in this app, embedding may be empty and is expected to be populated by `pushkind-crawlers`.
- Interpretation of vector format/dimension is delegated to worker pipeline and shared conventions.

### 4.3 Product Category Link

- A product may reference zero or one category.
- Category assignment is stored in `products.category_id`.
- `products.category_id` must be nullable.

### 4.4 Category Assignment Source and Locking

- Each product has assignment source metadata with allowed values:
  - `automatic`
  - `manual`
- Manual override operations set source to `manual`.
- Automatic matching operations may update only products with source `automatic`.
- Clearing a manual override sets `category_id = NULL` and source back to `automatic`.

## 5. Functional Requirements

### FR-01 Category Directory CRUD

Authorized users can:

- list categories,
- create a category,
- update category name (embedding remains crawler-managed),
- delete a category.

All operations require role `parser`.
All category CRUD is scoped to the current user hub.

### FR-02 Category Directory UI

Provide a server-rendered category management page with:

- category listing,
- add/edit forms,
- delete action,
- flash messages for operation result.

### FR-03 Product Canonical Category

- Add optional `category_id` foreign key on `products`.
- Product views and APIs that surface category data should prefer canonical category name from directory when `category_id` is set.
- Existing raw crawler category text column must remain because it is used as an input signal for product embedding generation in category matching.

### FR-04 Manual Product Category Override

Add authenticated operations allowing parser users to:

- set a category for a specific product manually,
- clear manual assignment for a specific product.

Manual set behavior:

- validates product is in current user's hub,
- validates referenced category exists in current user's hub,
- writes `products.category_id` and marks source as `manual`.

Manual clear behavior:

- validates product is in current user's hub,
- sets `products.category_id = NULL`,
- marks source as `automatic`.

### FR-05 Queue Product Category Matching Job

Add an authenticated endpoint that enqueues a ZeroMQ message to crawler workers:

- action: match categories for products,
- scope: all products in requesting user hub,
- behavior: worker assigns one best category per product from that hub's category directory.

### FR-06 Matching Write Semantics

When worker completes a matching run:

- each processed product has at most one category assignment,
- assignments with source `automatic` may be overwritten by better/latest match,
- assignments with source `manual` must remain unchanged,
- assignment may remain `NULL` if no valid match is available.

## 6. HTTP Surface (Proposed)

HTML routes:

- `GET /categories` -> category directory page.
- `POST /categories` -> create category.
- `POST /categories/{category_id}/update` -> update category.
- `POST /categories/{category_id}/delete` -> delete category.
- `POST /products/{product_id}/category` -> set manual category assignment.
- `POST /products/{product_id}/category/clear` -> clear manual assignment and unlock automatic matching.
- `POST /categories/match-products` -> enqueue bulk product category matching.

JSON routes:

- None required in V1 (server-rendered CRUD).

## 7. Data Model Changes (Proposed)

### 7.1 New `categories` table

Required columns:

- `id INTEGER PRIMARY KEY`
- `hub_id INTEGER NOT NULL`
- `name TEXT NOT NULL`
- `embedding BLOB NULL`

Recommended support columns:

- `created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP`
- `updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP`

Indexes/constraints:

- unique index on `(hub_id, normalized_name)` (case-insensitive on name component).

### 7.2 Update `products` table

- add `category_id INTEGER NULL REFERENCES categories(id) ON DELETE SET NULL`.
- add `category_assignment_source TEXT NOT NULL DEFAULT 'automatic'`.
- add check constraint restricting `category_assignment_source` to `('automatic', 'manual')`.
- add index on `products(category_id)`.

### 7.3 Existing `products.category` text

`products.category` must be retained as an independent raw-text field because it is an input source for product embedding generation used in category matching. Canonical assignment is represented by `category_id`, which points to a category in the same hub scope.

## 8. ZMQ Contract Changes (Proposed)

Extend `ZMQCrawlerMessage` with a dedicated category matching command.

Proposed shape:

- `ProductCategoryMatch(HubId)`  
or equivalent payload carrying `hub_id`.

Contract semantics:

- worker loads category directory + product data for target hub,
- computes relevance and assigns single best category per product,
- skips products where assignment source is `manual`,
- persists `products.category_id` updates.

## 9. Authorization and Scoping

- Role gate remains `parser`.
- Category CRUD is hub-scoped (each hub has an isolated directory).
- Manual product category overrides are hub-scoped (product ownership checked through crawler hub).
- Product mutations from matching are hub-scoped via `hub_id` in job payload.

## 10. Error Handling

- Duplicate category name -> validation/form error.
- Invalid name path format -> validation/form error.
- Cross-hub category access attempts -> not found or unauthorized.
- Invalid manual assignment target product/category -> not found or validation/form error.
- Delete category in use -> allowed, sets `products.category_id = NULL`; affected manual assignments are reset to `automatic`.
- ZMQ send failure -> non-fatal UI error (flash), no DB mutation.

## 11. Acceptance Criteria

- Categories can be created, listed, edited, and deleted from UI by parser users.
- Category CRUD and reads are isolated by hub.
- DB contains new `categories` table with `hub_id` and `products.category_id` FK.
- DB contains product assignment source metadata for manual/automatic ownership.
- Product records support zero/one canonical category reference.
- Parser users can manually set and clear product category assignments for products in their hub.
- Automatic matching does not overwrite manual assignments.
- Triggering category match endpoint publishes the new ZMQ message successfully.
- Unauthorized users cannot access category CRUD, manual assignment overrides, or matching action.
- Existing crawler and benchmark flows continue to work without regression.

## 12. Testing Requirements

- Unit tests for category form validation (`name`).
- Unit tests for manual assignment forms/payloads.
- Service tests for CRUD authorization and repository orchestration.
- Service tests for manual set/clear assignment behavior and hub scoping checks.
- Service test for ZMQ dispatch of category matching command.
- Service/repository tests verifying automatic matching updates skip manual assignments.
- Repository tests for category CRUD and product-category FK behavior.
- Regression tests for existing product/benchmark listing behavior.

## 13. Rollout Notes

- Add migration with reversible SQLite-safe `down.sql` strategy.
- Regenerate Diesel schema after migration.
- Update implementation-aligned `SPEC.md` only after feature implementation is complete.
