# pushkind-dantes Specification

Status: implementation-aligned specification
Last updated: 2026-02-19
Scope: reflects the code currently present in this repository (`src/`, `templates/`, `migrations/`).

## 1. Purpose

`pushkind-dantes` is an operations console for parser users that manage:
- crawler runs,
- crawled product catalogs,
- benchmark products,
- benchmark to product matching,
- price update jobs.

The app is server-rendered (Actix + Tera) with one JSON search endpoint used by the benchmark UI.

## 2. Roles and Access

- Required role: `parser` (`SERVICE_ACCESS_ROLE`).
- User context comes from `pushkind-common` authentication (`AuthenticatedUser`).
- Authorization model:
  - UI service functions enforce `parser` role and hub scoping.
  - Most UI unauthorized paths redirect to `/na`.
  - API unauthorized responses return `401 Unauthorized`.

Hub scoping rules:
- Crawler and benchmark reads are filtered by `user.hub_id`.
- Category directory reads/writes are filtered by `user.hub_id`.
- Product records are validated through their owning crawler when needed.

## 3. Architecture

Layering in code:
- `src/domain`: strongly typed domain entities and value objects.
- `src/models`: Diesel database models.
- `src/repository`: traits + Diesel implementation (`DieselRepository`).
- `src/forms`: request payload parsing/validation (especially benchmark workflows).
- `src/services`: business logic and orchestration.
- `src/routes`: Actix handlers (thin HTTP layer).
- `templates/`: Tera server-side UI.

Infra integrations:
- SQLite via Diesel + r2d2 pool.
- ZeroMQ publisher via `pushkind_common::zmq::ZmqSender`.
- Cookie session + identity + flash messages.

## 4. Functional Requirements

### FR-01 Dashboard: Crawlers
- Show all crawlers for current user hub at `GET /`.
- For each crawler show name, URL, last update time, product count, processing state.
- Row click navigates to crawler detail (`/crawler/{id}`).

### FR-02 Crawler Product Listing
- `GET /crawler/{crawler_id}?page={n}`:
  - verify crawler exists in user hub,
  - load paginated products for that crawler,
  - render products table and pagination.
- UI allows client-side table sorting by name/category/price.

### FR-03 Trigger Crawler Run
- `POST /crawler/{crawler_id}/crawl`:
  - verify role and crawler ownership,
  - enqueue ZeroMQ message `Crawler(Selector(crawler.selector))`.
- Return behavior:
  - success send: flash success,
  - send failure: flash error,
  - crawler not found: flash error.

### FR-04 Trigger Crawler Price Update
- `POST /crawler/{crawler_id}/update`:
  - verify role and crawler ownership,
  - load all crawler products,
  - enqueue ZeroMQ message `Crawler(SelectorProducts((selector, urls)))`.

### FR-05 Benchmark List
- `GET /benchmarks`:
  - list benchmarks for current hub,
  - show name, last update, associated product count, processing state.

### FR-06 Benchmark Detail
- `GET /benchmark/{benchmark_id}`:
  - load benchmark by id and hub,
  - list crawlers for hub,
  - for each crawler, show first page of benchmark-associated products,
  - load similarity distances (`product_id -> distance`) for display.

### FR-07 Add Single Benchmark
- `POST /benchmark/add` using form fields:
  - `name`, `sku`, `category`, `units`, `price`, `amount`, `description`.
- Validation:
  - string fields must be non-empty,
  - price and amount must be positive finite values.

### FR-08 Upload Benchmarks from CSV
- `POST /benchmarks/upload` multipart form with `csv` file (max 10MB).
- CSV row schema:
  - `name,sku,category,units,price,amount,description`.
- Each row is validated with same constraints as FR-07.

### FR-09 Match Benchmark (Background Job)
- `POST /benchmark/{benchmark_id}/match`:
  - verify benchmark exists in user hub,
  - enqueue ZeroMQ message `Benchmark(benchmark_id)`.

### FR-10 Update Prices for Matched Benchmark Products
- `POST /benchmark/{benchmark_id}/update`:
  - for each crawler in hub, collect products linked to benchmark,
  - skip crawlers with zero linked products,
  - enqueue one `SelectorProducts` message per crawler.
- UI gets per-crawler flash message (success/failure).

### FR-11 Manual Match Association Management
- Create association: `POST /benchmark/associate` (`benchmark_id`, `product_id`).
- Remove association: `POST /benchmark/unassociate` (`benchmark_id`, `product_id`).
- Validation:
  - both IDs must be >= 1,
  - benchmark must belong to current hub,
  - product must exist,
  - product's crawler must belong to current hub.
- New manual association uses default `distance = 1.0`.

### FR-12 Product Search API for Benchmark UI
- `GET /api/v1/products?crawler_id={id}&query={q?}&page={n?}`.
- Behavior:
  - role and hub checks,
  - paginated list with optional full-text search,
  - strips `embedding` before JSON response.
- Used by benchmark page selectize search dropdown (front-end limits shown results to first 20).

### FR-13 Category Directory CRUD
- `GET /categories` lists categories for the current hub.
- `POST /categories` creates a category in the current hub.
- `POST /categories/{category_id}/update` updates category name in the current hub.
- `POST /categories/{category_id}/delete` deletes category in the current hub.
- Validation:
  - category name is required and non-empty,
  - category path parts are split by `/`,
  - each part is trimmed, must stay non-empty, and is re-joined with `/`.
- Embedding behavior:
  - category embedding is optional in storage,
  - create/update flows in this service set embedding to `None`,
  - embedding regeneration is handled asynchronously by `pushkind-crawlers`.

### FR-14 Manual Product Category Override
- Set manual assignment: `POST /products/{product_id}/category` (`product_id`, `category_id`).
- Clear manual assignment: `POST /products/{product_id}/category/clear` (`product_id`).
- Validation and ownership:
  - product must exist and belong to a crawler in current hub,
  - target category must exist in current hub for set operation.
- Behavior:
  - set writes `products.category_id` and sets `products.category_assignment_source = manual`,
  - clear removes `products.category_id` and sets `products.category_assignment_source = automatic`.

### FR-15 Trigger Product-to-Category Matching Job
- `POST /categories/match-products`:
  - verifies role,
  - enqueues ZeroMQ message `ProductCategoryMatch(hub_id)` for `pushkind-crawlers`.
- Worker-side contract:
  - automatic matching must not overwrite products with `category_assignment_source = manual`.

### FR-16 Canonical Category Display Precedence
- Product listing views use canonical category name from `categories.name` when `products.category_id` is set.
- Existing raw `products.category` text remains stored and available for compatibility and downstream embedding inputs.

## 5. HTTP Surface

### HTML Routes
- `GET /` -> crawler dashboard.
- `GET /crawler/{crawler_id}` -> crawler product list.
- `POST /crawler/{crawler_id}/crawl` -> start crawler job.
- `POST /crawler/{crawler_id}/update` -> update crawler product prices.
- `GET /benchmarks` -> benchmark list.
- `GET /benchmark/{benchmark_id}` -> benchmark detail.
- `POST /benchmark/add` -> add benchmark.
- `POST /benchmarks/upload` -> CSV upload.
- `POST /benchmark/{benchmark_id}/match` -> queue matching.
- `POST /benchmark/{benchmark_id}/update` -> queue price updates.
- `POST /benchmark/associate` -> manual match.
- `POST /benchmark/unassociate` -> remove match.
- `GET /categories` -> category directory page.
- `POST /categories` -> add category.
- `POST /categories/{category_id}/update` -> update category.
- `POST /categories/{category_id}/delete` -> delete category.
- `POST /products/{product_id}/category` -> set manual product category.
- `POST /products/{product_id}/category/clear` -> clear manual product category.
- `POST /categories/match-products` -> queue product-to-category matching for hub.

### JSON API
- `GET /api/v1/products` -> product list/search JSON.

### Other Mounted Endpoints
- `GET /na` (not assigned page, from shared crate).
- `POST /logout` (from shared crate).
- `GET /assets/*` static files.

## 6. Data Model

Core tables:
- `crawlers`:
  - `id`, `hub_id`, `name`, `url`, `selector`, `processing`, `updated_at`, `num_products`.
- `products`:
  - `id`, `crawler_id`, `name`, `sku`, optional `category` (raw crawler text kept for compatibility and embedding input), optional `units`, `price`, optional `amount`, optional `description`, `url`, timestamps, optional `embedding` blob, optional `category_id`, `category_assignment_source`.
- `benchmarks`:
  - `id`, `hub_id`, `name`, `sku`, `category`, `units`, `price`, `amount`, `description`, timestamps, optional `embedding`, `processing`, `num_products`.
- `categories`:
  - `id`, `hub_id`, `name`, optional `embedding`, timestamps.
- `product_benchmark` (many-to-many join):
  - composite PK (`product_id`, `benchmark_id`), `distance` float.
- `product_images`:
  - `id`, `product_id`, `url`.

Search/indexing:
- SQLite FTS5 virtual table `products_fts` over product text columns.
- Triggers keep FTS table synced on insert/update/delete.
- Unique index on `(products.crawler_id, products.url)`.
- Case-insensitive unique index on `(categories.hub_id, lower(categories.name))`.
- `products.category_id` has FK relation to `categories.id`.

Seed data in migrations:
- Initial crawler records are inserted for hub `1` (selectors: `101tea`, `rusteaco`, `gutenberg`).

## 7. Domain Type Constraints

Strongly typed wrappers enforce invariants:
- IDs (`HubId`, `CrawlerId`, `ProductId`, `BenchmarkId`) > 0.
- Text wrappers (`ProductName`, `BenchmarkSku`, etc.) are trimmed and non-empty.
- Category path normalization trims each slash-separated path segment and rejects empty segments.
- URL wrappers (`CrawlerUrl`, `ProductUrl`, `ImageUrl`) must pass URL validation.
- `ProductPrice`, `ProductAmount` must be positive finite numbers.
- `ProductCount` must be >= 0.
- `SimilarityDistance` must be within `[0.0, 1.0]`.

## 8. ZeroMQ Contract

Message enum (`ZMQCrawlerMessage`):
- `Crawler(CrawlerSelector)` where selector is:
  - `Selector(crawler_selector)`, or
  - `SelectorProducts((crawler_selector, Vec<ProductUrl>))`.
- `Benchmark(benchmark_id)`.
- `ProductCategoryMatch(hub_id)`.

Emission points:
- Crawl crawler: `Selector`.
- Update crawler prices: `SelectorProducts` with crawler product URLs.
- Match benchmark: `Benchmark`.
- Update benchmark prices: one `SelectorProducts` per crawler that has matched products.
- Match products to categories for hub: `ProductCategoryMatch`.
- Worker rule for category matching: do not overwrite records with manual assignment source.

## 9. Configuration and Runtime

Startup behavior:
- Loads `.env` when present.
- Loads config from:
  - `config/default.yaml`,
  - `config/{APP_ENV}.yaml` (default `APP_ENV=local`),
  - environment variables with `APP_` prefix.

Required runtime settings (effective `ServerConfig`):
- `domain`
- `address`
- `port`
- `database_url`
- `zmq_crawlers_pub`
- `templates_dir`
- `secret`
- `auth_service_url`

Server middleware/features:
- compression, logging,
- cookie session + identity,
- flash message framework,
- redirect middleware for unauthorized UI traffic.

## 10. UI Behavior Notes

- UI language is Russian.
- Flash messages are used for most success/failure outcomes.
- Tables support client-side sorting.
- Product description text can toggle truncation on click.
- Benchmark detail page uses selectize + `/api/v1/products` to add manual associations.
- Categories page provides category CRUD and a trigger button for bulk product-category matching.
- Product rows can be manually assigned/cleared against category directory entries and display assignment source badge.
- This section is descriptive, not contractual: route semantics are stable, while UI markup/text/layout may change without API contract changes.

## 11. Error Handling Contract

Service-level mapping:
- `Unauthorized` -> redirect `/na` (UI) or `401` (API).
- `NotFound` -> redirect + flash for most UI detail actions; `404` for API.
- Validation failures in forms -> `ServiceError::Form(message)` and flash errors.
- Infra/repository failures are logged and usually returned as `Internal`.

Notable implementation detail:
- Some write operations (`add_benchmark`, `upload_benchmarks`, association writes) convert repository failures to `Ok(false)` and rely on route-level flash messaging instead of hard failing.

## 12. Quality Gates and Testing

Repository guidance in this repo expects running:
- `cargo build --all-features --verbose`
- `cargo test --all-features --verbose`
- `cargo clippy --all-features --tests -- -Dwarnings`
- `cargo fmt --all -- --check`

Current automated coverage shape:
- service unit tests inline in service modules,
- form conversion/validation tests in `src/forms/benchmarks.rs` and `src/forms/categories.rs`,
- in-memory mock repository for service tests (`src/repository/test.rs`),
- basic integration tests for DB bootstrap and repository wiring (`tests/`).

## 13. Current Limitations / Gaps (As Implemented)

- DTO usage is minimal and currently limited to category page transport mapping (`CategoryDto`).
- Product search in UI is only exposed via benchmark association workflow (`/api/v1/products`), not crawler page filtering.
- Benchmark detail always loads page 1 of associated products per crawler in service logic.
- Some migration `down.sql` statements use SQLite-incompatible `DROP COLUMN` syntax; rollback paths may require manual adjustment.

## 14. Non-Functional Baseline

Current implementation exposes the following implicit NFR profile:

- Performance:
  - list/search endpoints are paginated via `DEFAULT_ITEMS_PER_PAGE`,
  - product text search uses SQLite FTS5 with triggers (`products_fts`),
  - no explicit latency SLO/SLA is defined in code.
- Concurrency:
  - app uses Actix worker model + Diesel r2d2 pool,
  - no explicit application-level locking/state machine coordination for crawler or benchmark processing flags.
- Delivery semantics (job enqueue):
  - HTTP layer sends ZeroMQ messages and reports immediate send success/failure only,
  - service layer does not implement retries, deduplication, or end-to-end acknowledgements.
- Restart/failure behavior:
  - after process restart, in-flight HTTP requests are lost,
  - enqueued job completion guarantees are delegated to downstream worker/transport behavior outside this codebase.

## 15. Processing State Model

Processing flags are currently boolean fields:
- `crawlers.processing`
- `benchmarks.processing`

Observed state model from this service:

| Entity | State | Storage Value | Set/Clear Owner in This Repo |
|---|---|---|---|
| Crawler | Idle | `false` | Not set here (read-only in app layer) |
| Crawler | Processing | `true` | Not set here (read-only in app layer) |
| Benchmark | Idle | `false` | Not set here (read-only in app layer) |
| Benchmark | Processing | `true` | Not set here (read-only in app layer) |

Notes:
- This service triggers work by publishing ZMQ messages; it does not mutate processing flags directly.
- Transition policy (`false -> true -> false`), race handling, and stuck-state remediation are external concerns (crawler/matching worker side).

## 16. Security and Trust Boundaries

Boundary overview:
- Browser/client -> Actix server (cookie session + identity via shared auth stack).
- Actix server -> SQLite (Diesel repository layer).
- Actix server -> ZeroMQ publisher (job dispatch only).

Current safeguards:
- Role checks (`parser`) and hub-scoped data reads.
- Typed form validation for benchmark workflows.
- Multipart CSV upload capped at 10MB in form definition.

Current explicit non-guarantees in this repository:
- No explicit CSRF token validation middleware is declared in this crate.
- No application-layer malware/content scanning for uploaded CSV files.
- No message signing/auth layer on ZMQ payloads at this boundary (transport/auth assumptions belong to deployment topology and worker environment).

## 17. Glossary

- Crawler:
  - a source configuration (`name`, `url`, `selector`) that owns products and can be queued for crawl/price updates.
- Benchmark:
  - canonical reference product for comparison/matching within a hub.
- Match (Association):
  - link between a benchmark and product stored in `product_benchmark`.
- Distance:
  - numeric value in `product_benchmark.distance` (float, domain-validated range `[0.0, 1.0]` for typed usage),
  - service displays stored values and sorts via repository query order; exact scoring algorithm is external to this service.

## 18. Non-Goals (Current Scope)

- No real-time push updates to UI (no websocket/SSE in this crate).
- No cross-hub data sharing workflows in UI/service behavior.
- No multi-currency support in UI rendering (templates currently use `₽` constant).
- No public, versioned API surface beyond `GET /api/v1/products` used by the internal benchmark UI flow.
