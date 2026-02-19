# Plan: Master Category Directory

Status: stable  
Feature spec: `specs/features/master-category-directory.md`  
ADR: `specs/decisions/0001-master-category-directory.md`

## 1. Scope

Implement master category directory CRUD, product optional `category_id`, manual per-product category overrides, assignment locking semantics, and ZMQ-driven product category matching trigger.

## 2. Work Breakdown

### Phase 1: Data and Domain

- Add Diesel migration for:
  - new `categories` table with `hub_id`,
  - `products.category_id` nullable foreign key,
  - `products.category_assignment_source` metadata for `automatic`/`manual`,
  - preserve existing `products.category` raw-text column (required embedding input signal),
  - per-hub category name uniqueness constraints/indexes,
  - required indexes and uniqueness constraints.
- Regenerate `src/schema.rs`.
- Add domain entity/types:
  - `Category`, `NewCategory`,
  - `CategoryId` value object,
  - category embedding representation.
- Add Diesel model conversions for categories and product category link.

### Phase 2: Repository Layer

- Introduce repository traits:
  - `CategoryReader`,
  - `CategoryWriter`.
- Implement Diesel repository CRUD for categories with mandatory hub scoping.
- Add repository write operations for manual set/clear of product category assignments.
- Extend product reads to include canonical category information where needed.
- Update `TestRepository` for service tests.

### Phase 3: Forms and Services

- Add category forms and payload validation.
- Implement category services:
  - list/create/update/delete.
  - Add services under `src/services/categories.rs`.
- Enforce hub-isolation in category service operations (read/write only within current user hub).
- Add forms/services for manual product category set/clear operations with hub scoping checks.
- Add service to enqueue category matching ZMQ command.
- Enforce assignment lock semantics in service/repository contracts:
  - manual assignments must not be overwritten by automatic matching results.
- Keep service-level authorization and hub scoping rules aligned with existing patterns.

### Phase 4: Routes and Templates

- Add routes under `src/routes/categories.rs`.
- Register routes in `src/lib.rs`.
- Add templates for category page/actions.
- Extend navigation component to include categories.
- Ensure product category display prefers canonical category name.
- Add manual assignment controls to product-facing screens.

### Phase 5: ZMQ and Contracts

- Extend `src/domain/zmq.rs` with category matching command.
- Wire route/service action to publish new command.
- Document worker-facing requirement that automatic matching skips `manual` assignments.
- Validate serialization/deserialization via tests.

### Phase 6: Verification and Documentation

- Run:
  - `cargo build --all-features --verbose`
  - `cargo test --all-features --verbose`
  - `cargo clippy --all-features --tests -- -Dwarnings`
  - `cargo fmt --all -- --check`
- Update `SPEC.md` after implementation is complete.
- Update this plan/spec as needed for scope changes.

## 3. Risks and Mitigations

- SQLite migration limitations for `down.sql`.
  - Mitigation: use table recreation strategy where needed.
- Potential worker incompatibility with new ZMQ message variant.
  - Mitigation: coordinate contract rollout and validate in staging.
- Category uniqueness edge cases (case, whitespace).
  - Mitigation: normalize at form/domain boundary and enforce DB uniqueness.
- Cross-hub category leakage in queries/writes.
  - Mitigation: require `hub_id` in all category repository filters and cover with tests.
- Assignment lock drift (manual records unintentionally overwritten).
  - Mitigation: enforce source checks in update queries and verify via tests.

## 4. Exit Criteria

- All acceptance criteria in `specs/features/master-category-directory.md` satisfied.
- Quality gates pass.
- No regressions in crawler, product, benchmark workflows.
