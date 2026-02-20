# Plan: Product Category Cell Dual Render

1. Add a dedicated `associated_category` field to domain `Product`.
2. Update model-to-domain conversion to preserve original `category`.
3. Update product repository hydration to fill `associated_category` from
   `category_id` lookup.
4. Update shared product row template to render:
   - original category,
   - associated category label when present.
5. Fix test fixtures that construct `Product`.
6. Run `cargo test --all-features --verbose` and `cargo fmt --all -- --check`.
