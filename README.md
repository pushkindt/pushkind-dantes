# pushkind-dantes

`pushkind-dantes` is the operations console for Pushkind's product crawling pipeline.
Parsers can inspect crawlers, review scraped products, and coordinate benchmark data
used to compare prices across providers. The service is built with Actix Web,
Diesel, and Tera and relies on the shared `pushkind-common` crate for
authentication, configuration, and reusable UI helpers.

## Features

- **Parser dashboard** – Role-gated index of crawlers for the current hub, exposing status, metadata, and quick actions to trigger crawls or price refreshes over ZeroMQ.
- **Product catalog browsing** – Paginated, searchable listings of products per crawler, available in the UI and via the `/api/v1/products` JSON endpoint.
- **Benchmark library** – Interfaces to review benchmarks, add individual entries, and ingest CSV uploads scoped to the current hub.
- **Benchmark associations** – Workflows to link crawler products to benchmarks, inspect similarity distances, and clean up incorrect matches.
- **Crawler orchestrations** – One-click actions that queue crawler runs or benchmark price updates through the background worker bus.

## Architecture at a Glance

The codebase follows a clean, layered structure so that business logic can be
exercised and tested without going through the web framework:

- **Domain (shared via `pushkind-common`)** – Strongly typed models for crawlers,
  products, and benchmarks live in `pushkind_common::domain::dantes`; this crate
  orchestrates them through repositories and services.
- **Repository (`src/repository`)** – Traits that describe the persistence
  contract and a Diesel-backed implementation (`DieselRepository`) that speaks to
  a SQLite database. Each module translates between Diesel models and domain
  types and exposes strongly typed query builders.
- **Services (`src/services`)** – Application use-cases that orchestrate domain
  logic, repository traits, and Pushkind authentication helpers. Services return
  `ServiceResult<T>` and map infrastructure errors into well-defined service
  errors.
- **Forms (`src/forms`)** – `serde`/`validator` powered structs that handle
  request payload validation, CSV parsing, and transformation into domain types.
- **Routes (`src/routes`)** – Actix Web handlers that wire HTTP requests into the
  service layer and render Tera templates or redirect with flash messages.
- **Templates (`templates/`)** – Server-rendered UI built with Tera and
  Bootstrap 5, backed by sanitized HTML rendered via `ammonia` when necessary.

Because the repository traits live in `src/repository/mod.rs`, service functions
accept generic parameters that implement those traits. This makes unit tests easy
by swapping in the `mockall`-based fakes from `src/repository/mock.rs`.

## Technology Stack

- Rust 2024 edition
- [Actix Web](https://actix.rs/) with identity, session, and flash message
  middleware
- [Diesel](https://diesel.rs/) ORM with SQLite and connection pooling via r2d2
- [Tera](https://tera.netlify.app/) templates styled with Bootstrap 5.3
- [`pushkind-common`](https://github.com/pushkindt/pushkind-common) shared crate
  for authentication guards, configuration, database helpers, and reusable
  patterns
- Supporting crates: `chrono`, `validator`, `serde`, `ammonia`, `csv`, and
  `thiserror`

## Getting Started

### Prerequisites

- Rust toolchain (install via [rustup](https://www.rust-lang.org/tools/install))
- `diesel-cli` with SQLite support (`cargo install diesel_cli --no-default-features --features sqlite`)
- SQLite 3 installed on your system

### Environment

The service reads configuration from environment variables. The most important
ones are:

| Variable | Description | Default |
| --- | --- | --- |
| `DATABASE_URL` | Path to the SQLite database file | `app.db` |
| `SECRET_KEY` | 32-byte secret for signing cookies | generated when unset |
| `AUTH_SERVICE_URL` | Base URL of the Pushkind authentication service | _required_ |
| `ZMQ_CRAWLER` | ZeroMQ endpoint used to communicate with the crawler worker | `tcp://127.0.0.1:5555` |
| `PORT` | HTTP port | `8080` |
| `ADDRESS` | Interface to bind | `127.0.0.1` |
| `DOMAIN` | Cookie domain (without protocol) | `localhost` |

Create a `.env` file if you want these values loaded automatically via
[`dotenvy`](https://crates.io/crates/dotenvy).

### Database

Run the Diesel migrations before starting the server:

```bash
diesel setup
cargo install diesel_cli --no-default-features --features sqlite # only once
diesel migration run
```

A SQLite file will be created at the location given by `DATABASE_URL`.

## Running the Application

Start the HTTP server with:

```bash
cargo run
```

The server listens on `http://127.0.0.1:8080` by default and serves static
assets from `./assets` in addition to the Tera-powered HTML pages. Authentication
and authorization are enforced via the Pushkind auth service and the
`SERVICE_ACCESS_ROLE` constant.

## Quality Gates

The project treats formatting, linting, and tests as required gates before
opening a pull request. Use the following commands locally:

```bash
cargo fmt --all -- --check
cargo clippy --all-features --tests -- -Dwarnings
cargo test --all-features --verbose
cargo build --all-features --verbose
```

Alternatively, the `make check` target will format the codebase, run clippy, and
execute the test suite in one step.

## Testing

Unit tests exercise the service and form layers directly, while integration
tests live under `tests/`. Repository tests rely on Diesel’s query builders and
should avoid raw SQL strings whenever possible. Use the mock repository module to
isolate services from the database when writing new tests.

## Project Principles

- **Domain-driven**: keep business rules in the domain and service layers and
  translate to/from external representations at the boundaries.
- **Explicit errors**: use `thiserror` to define granular error types and convert
  them into `ServiceError`/`RepositoryError` variants instead of relying on
  `anyhow`.
- **No panics in production paths**: avoid `unwrap`/`expect` in request handlers,
  services, and repositories—propagate errors instead.
- **Security aware**: sanitize any user-supplied HTML using `ammonia`, validate
  inputs with `validator`, and always enforce role checks with
  `pushkind_common::routes::check_role`.
- **Testable**: accept traits rather than concrete types in services and prefer
  dependency injection so the mock repositories can be used in tests.

Following these guidelines will help new functionality slot seamlessly into the
existing architecture and keep the service reliable in production.
