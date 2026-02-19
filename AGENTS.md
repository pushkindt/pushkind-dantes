# AGENTS.md

This document provides guidance to AI code generators when working in this
repository. Follow these practices so new code stays aligned with the current
implementation.

## Project Context

`pushkind-dantes` is a Rust 2024 application with feature-gated layers:
- `data` feature: domain types, Diesel models, schema, and shared conversions.
- `server` feature (default): Actix Web routes, services, forms, templates,
  and ZeroMQ integration.

Core stack:
- Actix Web + session/identity/flash middleware
- Diesel + SQLite + r2d2 pooling
- Tera templates
- shared `pushkind-common` crate (auth, middleware, db helpers, ZMQ sender)

Current module layout:
- `src/domain`: entities and strongly-typed value objects (`src/domain/types.rs`)
- `src/models`: Diesel structs and domain conversions
- `src/repository`: traits + `DieselRepository` implementation + in-memory
  `TestRepository` for unit tests
- `src/forms`: form parsing/validation (currently benchmark workflows)
- `src/services`: business logic + authorization checks
- `src/routes`: HTTP handlers
- `templates/`: server-rendered UI
- `src/dto/`: currently present but effectively unused in runtime flows

## Development Commands

Use these commands to verify changes:

**Build**
```bash
cargo build --all-features --verbose
```

**Run Tests**
```bash
cargo test --all-features --verbose
```

**Lint (Clippy)**
```bash
cargo clippy --all-features --tests -- -Dwarnings
```

**Format**
```bash
cargo fmt --all -- --check
```

Optional aggregate check:
```bash
make check
```

**Diesel Migrations**
```bash
diesel migration generate <migration-name> # for creating a new migration
diesel migration run # applies migrations and regenerates src/schema.rs
```

## Coding Standards

- Use idiomatic Rust; avoid `unwrap`/`expect` in production paths.
- Keep responsibilities separated:
  - routes: HTTP I/O + flash/redirect/response formatting,
  - services: orchestration + authorization + business rules,
  - repository: persistence concerns,
  - models/domain: representation and type conversions.
- Prefer explicit `From`/`TryFrom` conversions between Diesel and domain structs.
- Construct strong domain types (`HubId`, `ProductUrl`, etc.) at boundaries
  (forms/services). These wrappers enforce trimming/validation constraints.
- Forms should have typed `*Payload` companions and `TryFrom<*Form>` validation.
- Prefer dependency injection through trait bounds in services so
  `DieselRepository` and `TestRepository` remain interchangeable.
- Use `thiserror`-based error types and return `RepositoryResult<T>` /
  `ServiceResult<T>` conventions already used in the codebase.
- Document public APIs and breaking behavior changes.

## Database Guidelines

- Prefer Diesel query builder with `schema.rs` for standard CRUD/filtering.
- Reuse and extend existing query builders (`ProductListQuery`,
  `BenchmarkListQuery`) instead of duplicating filtering logic.
- Keep domain conversion logic in repository/model boundaries.
- For SQLite FTS search paths, raw SQL is an accepted existing pattern
  (`search_products`); keep bindings typed and avoid string interpolation for
  untrusted values.
- Preserve hub scoping guarantees when adding or changing queries.
- Ensure new migrations include realistic `down.sql`; SQLite does not support all
  `ALTER TABLE ... DROP COLUMN` operations directly.

## HTTP and Template Guidelines

- Keep Actix handlers in `src/routes` thin and transport-focused.
- Keep services transport-agnostic; do not embed HTTP response logic there.
- Continue using `pushkind_common::routes` helpers (`base_context`,
  `render_template`, `redirect`) in route handlers.
- Authorization is enforced in services via role checks using
  `pushkind_common::routes::check_role` and `SERVICE_ACCESS_ROLE`.
- Current API responses return domain structs directly (not DTO-based mapping).
- Reuse shared templates/components under `templates/components`.

## ZeroMQ Integration

- Job dispatch uses `Arc<ZmqSender>` injected via Actix app data.
- Message contracts live in `src/domain/zmq.rs`.
- The web app publishes crawl/match/update commands; worker execution,
  retries, and completion semantics are external to this repository.

## Testing Expectations

- Add unit tests for new service and form logic.
- Service unit tests should use `src/repository/test.rs` (`TestRepository`) to
  isolate business logic from Diesel.
- Integration tests under `tests/` should use Diesel migrations/helpers rather
  than hard-coded SQL fixtures.
- Run build/test/clippy/fmt checks before opening a PR.

## Documentation Expectations

- Update `SPEC.md` when behavior, contracts, routes, or data semantics change.
- Keep `README.md` concise; avoid duplicating detailed content that belongs in
  `SPEC.md` or this file.

## Workflow Requirements

- Always obey `SPEC.md`.
- For any new work, require both `specs/features/<name>.md` and
  `plans/<name>.md`.
- If a change touches architecture, add or update an ADR under
  `specs/decisions/`.
