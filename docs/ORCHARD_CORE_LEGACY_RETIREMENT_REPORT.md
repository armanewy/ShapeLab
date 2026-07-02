# Orchard Core Legacy Retirement Report

Date: 2026-07-02

Branch: `codex/orchard-core-legacy-retirement-pass`

## Summary

This pass removed the only direct `orchard-core-legacy` dependency from the native
product app. `orchard-app` had used `orchard-core-legacy` only for the low-level `Aabb`
type; it now imports that type through `orchard-render`, which already owns the
preview APIs that accept render bounds.

No `ShapeDocument` product dependency was added or expanded. The remaining
direct `orchard-core-legacy` dependents are either low-level geometry crates or legacy
implicit-document compatibility crates that still expose public workspace APIs.

## Dependency Map

| Crate | Current orchard-core-legacy use | Classification | Decision |
| --- | --- | --- | --- |
| `orchard-app` | Previously imported `Aabb` directly for preview bounds. | Removable now / low-level type only. | Removed direct dependency; app uses `orchard_render::Aabb`. |
| `orchard-render` | Uses `Aabb` in camera fitting, render request bounds, and foundry preview bounds. | Still necessary low-level type. | Kept; re-exports `Aabb` as the adapter for app/render callers. |
| `orchard-mesh` | Uses `Aabb` for `TriangleMesh::bounds`, mesh construction, and mesh-quality tests. | Still necessary low-level type. | Kept until bounds move to a neutral geometry crate. |
| `orchard-field` | Compiles and validates legacy `ShapeDocument`, `NodeKind`, `PrimitiveKind`, and transforms into implicit scalar fields. | Legacy implicit document dependency. | Blocked until implicit/SDF compatibility is retired or converted to semantic asset input. |
| `orchard-search-internal` | Legacy search APIs mutate `ShapeDocument` through `EditProgram`, `ParamDescriptor`, `ParamGroup`, and scalar helpers. Foundry search also uses low-level `Aabb`, `Scalar`, and `Transform3`. | Mixed: legacy implicit document plus necessary low-level type. | Blocked until legacy document search is split/retired and low-level math types move out of `orchard-core-legacy`. |
| `orchard-project` | Root project API stores legacy `ShapeDocument`, `RevisionId`, `EditProgram`, and validation reports. Foundry project modules already use `orchard-asset` revisions. | Legacy implicit document dependency. | Blocked because removing it would break public project contracts and CLI legacy project behavior. |
| `orchard-presets` | Produces old `ShapeDocument` primitive/CSG presets and validates them with `orchard-core-legacy`. | Obsolete dependency. | Blocked until legacy preset CLI paths are removed or replaced by semantic asset presets. |
| `orchard-cli` | Uses `ShapeDocument`, `ParamGroup`, validation, and old preset/search/project paths; also uses `Aabb` for newer object-plan output paths. | Mixed: product-facing legacy implicit document plus low-level type. | Blocked because removing it would change CLI behavior. Next pass should split legacy commands from object-plan/foundry commands. |

## Transitive Product Exposure

`orchard-app` still receives `orchard-core-legacy` transitively through `orchard-render`,
`orchard-mesh`, `orchard-search-internal`, and `orchard-project`. This pass removed the app's
direct import and manifest dependency only. The remaining transitive exposure is
blocked by public APIs in those lower-level crates rather than by app code.

## Public Surface Changes

- `orchard-render` now publicly re-exports `orchard_core_legacy::Aabb` so product code can
  use render bounds without depending on `orchard-core-legacy` directly.
- `orchard-core-legacy` rustdoc now labels `Scalar`, `ParamPath`, `ShapeDocument`, and
  `EditProgram` as legacy/low-level compatibility concepts rather than
  authoritative product semantics.
- No `orchard-core-legacy` modules or fixtures were deleted. The crate is currently a
  single public module, and all legacy document APIs are still used by workspace
  crates or tests. Deleting them here would break shared public contracts.

## Tests Added

- `orchard-core-legacy` docs must not claim `ShapeDocument` is the canonical A-J IR.
- Product architecture docs must keep `orchard-asset::AssetRecipe` / Orchard IR
  as the canonical semantic lane.
- `docs/CURRENT_PRODUCT_STATUS.md` allowed product claims must not point users
  to `orchard-core-legacy` or `ShapeDocument`.

## Next Retirement Steps

1. Move low-level `Scalar`, `Aabb`, and transform conventions to a neutral
   geometry/common crate in an integration-owned dependency reconciliation
   branch.
2. Split `orchard-search-internal` into semantic asset search and legacy document search,
   then retire the legacy `ShapeDocument` candidate API when no CLI or tests
   require it.
3. Split or feature-gate legacy `orchard-project` root APIs away from the active
   foundry project modules.
4. Replace `orchard-presets` with semantic asset/foundry presets, then remove the
   old ShapeDocument preset crate from CLI paths.
5. Retire `orchard-field` and implicit `ShapeDocument` meshing once all product
   preview/export paths compile from `orchard-asset` artifacts.
