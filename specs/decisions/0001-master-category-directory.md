# ADR-0001: Canonical Category Directory for Products

Date: 2026-02-19  
Status: Accepted  
Related feature: `specs/features/master-category-directory.md`

## Context

The system currently stores product category as optional free-form text (`products.category`), which is not canonical and cannot support consistent category-level workflows.

We need:

- a single category source of truth,
- CRUD management for category records,
- product linkage to canonical categories,
- manual per-product override capability,
- background category assignment initiated via ZMQ.

## Decision

1. Introduce a dedicated `categories` table with at least `id`, `name`, and `embedding`.
   - Include `hub_id` so directories are tenant-isolated per hub.
2. Keep category hierarchy flat in storage; represent paths as slash-separated names (`A/B/C`) without parent-child DB relations.
3. Add nullable `products.category_id` foreign key referencing `categories.id`.
4. Treat category directories as hub-scoped; category CRUD and lookups are isolated by `hub_id`.
5. Extend ZMQ command contract with a dedicated message for product-category matching.
6. Track category assignment source per product (`automatic` or `manual`), and require automatic matching workers to skip `manual` assignments.

## Consequences

### Positive

- Category semantics become canonical and centrally managed.
- Category data is tenant-isolated per hub.
- Product classification becomes queryable and enforceable via foreign keys.
- Users can correct assignments explicitly while preserving curated values.
- Matching workflow is explicit and asynchronous, consistent with existing job dispatch model.

### Negative

- Requires coordinated ZMQ contract change with worker service.
- Adds migration complexity for SQLite (especially rollback paths).
- Introduces dual category fields during transition (`products.category` + `products.category_id`).
- Introduces additional assignment-source state that must be preserved across write paths.

### Neutral / Tradeoffs

- Not modeling a DB tree simplifies schema and CRUD but shifts hierarchy interpretation to naming convention.
- Keeping raw-text `products.category` is required because it remains an embedding input signal, while canonical assignment is introduced via `products.category_id`.

## Alternatives Considered

1. Tree-structured category table (`parent_id`):
   - Rejected for V1 due to higher complexity and no immediate requirement.
2. Replace `products.category` text directly (drop column):
   - Rejected for V1 to avoid migration/search disruption and reduce rollout risk.
3. Synchronous category matching in HTTP request:
   - Rejected due to latency and operational risk; async ZMQ job is consistent with existing architecture.

## Follow-Up

- Implement schema, repository, service, and route changes per plan.
- Update implementation-aligned `SPEC.md` after delivery.
