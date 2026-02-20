# Product Category Cell Dual Render

## Summary
Render two pieces of category information in product rows:
- original product category extracted from source data,
- associated canonical category name (when product has `category_id`).

## Problem
`product.category` was being overwritten during repository hydration with the
canonical category name. This removed the original category value from UI and
API flows.

## Requirements
- Keep original parsed category in `product.category`.
- Populate associated category name separately.
- Render both values in product row category cell.
- If neither value exists, render a placeholder.

## Non-goals
- No changes to category assignment workflows.
- No schema migration.

## Acceptance Criteria
- Product rows show original category and associated category (if present).
- Existing tests continue to pass.
