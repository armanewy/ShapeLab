# Contract Changelog

## 2026-07-01 - AssetRecipe v8 Semantic Shells

- Bumped `ASSET_RECIPE_SCHEMA_VERSION` from 7 to 8.
- Added defaulted `AssetRecipe.semantic` shells for relationship, pattern,
  surface, material, collision, motion, terrain, export, authoring,
  validation, review, lineage, effect-hash, and export-include contracts.
- Added migration from schema versions 1 through 7 to schema version 8.
- Added validation for shell ID stability, populated reference targets, review
  boundaries, publish blockers, and unsupported export include claims.
- Added fixtures for old minimal v7, new minimal v8 shell, Box-like v8, and
  Panel-with-Knob-like v8 recipes.
- No product-facing feature behavior was added.
