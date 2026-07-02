# Shape Core Retirement Report

Date: 2026-07-02

Branch: `codex/shape-core-legacy-retirement-pass`

## Summary

This pass removed the only direct `shape-core` dependency from the native
product app. `shape-app` had used `shape-core` only for the low-level `Aabb`
type; it now imports that type through `shape-render`, which already owns the
preview APIs that accept render bounds.

No `ShapeDocument` product dependency was added or expanded. The remaining
direct `shape-core` dependents are either low-level geometry crates or legacy
implicit-document compatibility crates that still expose public workspace APIs.

## Dependency Map

| Crate | Current shape-core use | Classification | Decision |
| --- | --- | --- | --- |
| `shape-app` | Previously imported `Aabb` directly for preview bounds. | Removable now / low-level type only. | Removed direct dependency; app uses `shape_render::Aabb`. |
| `shape-render` | Uses `Aabb` in camera fitting, render request bounds, and foundry preview bounds. | Still necessary low-level type. | Kept; re-exports `Aabb` as the adapter for app/render callers. |
| `shape-mesh` | Uses `Aabb` for `TriangleMesh::bounds`, mesh construction, and mesh-quality tests. | Still necessary low-level type. | Kept until bounds move to a neutral geometry crate. |
| `shape-field` | Compiles and validates legacy `ShapeDocument`, `NodeKind`, `PrimitiveKind`, and transforms into implicit scalar fields. | Legacy implicit document dependency. | Blocked until implicit/SDF compatibility is retired or converted to semantic asset input. |
| `shape-search` | Legacy search APIs mutate `ShapeDocument` through `EditProgram`, `ParamDescriptor`, `ParamGroup`, and scalar helpers. Foundry search also uses low-level `Aabb`, `Scalar`, and `Transform3`. | Mixed: legacy implicit document plus necessary low-level type. | Blocked until legacy document search is split/retired and low-level math types move out of `shape-core`. |
| `shape-project` | Root project API stores legacy `ShapeDocument`, `RevisionId`, `EditProgram`, and validation reports. Foundry project modules already use `shape-asset` revisions. | Legacy implicit document dependency. | Blocked because removing it would break public project contracts and CLI legacy project behavior. |
| `shape-presets` | Produces old `ShapeDocument` primitive/CSG presets and validates them with `shape-core`. | Obsolete dependency. | Blocked until legacy preset CLI paths are removed or replaced by semantic asset presets. |
| `shape-cli` | Uses `ShapeDocument`, `ParamGroup`, validation, and old preset/search/project paths; also uses `Aabb` for newer object-plan output paths. | Mixed: product-facing legacy implicit document plus low-level type. | Blocked because removing it would change CLI behavior. Next pass should split legacy commands from object-plan/foundry commands. |

## Transitive Product Exposure

`shape-app` still receives `shape-core` transitively through `shape-render`,
`shape-mesh`, `shape-search`, and `shape-project`. This pass removed the app's
direct import and manifest dependency only. The remaining transitive exposure is
blocked by public APIs in those lower-level crates rather than by app code.

## Public Surface Changes

- `shape-render` now publicly re-exports `shape_core::Aabb` so product code can
  use render bounds without depending on `shape-core` directly.
- `shape-core` rustdoc now labels `Scalar`, `ParamPath`, `ShapeDocument`, and
  `EditProgram` as legacy/low-level compatibility concepts rather than
  authoritative product semantics.
- No `shape-core` modules or fixtures were deleted. The crate is currently a
  single public module, and all legacy document APIs are still used by workspace
  crates or tests. Deleting them here would break shared public contracts.

## Tests Added

- `shape-core` docs must not claim `ShapeDocument` is the canonical A-J IR.
- Product architecture docs must keep `shape-asset::AssetRecipe` / Orchard IR
  as the canonical semantic lane.
- `docs/CURRENT_PRODUCT_STATUS.md` allowed product claims must not point users
  to `shape-core` or `ShapeDocument`.

## Next Retirement Steps

1. Move low-level `Scalar`, `Aabb`, and transform conventions to a neutral
   geometry/common crate in an integration-owned dependency reconciliation
   branch.
2. Split `shape-search` into semantic asset search and legacy document search,
   then retire the legacy `ShapeDocument` candidate API when no CLI or tests
   require it.
3. Split or feature-gate legacy `shape-project` root APIs away from the active
   foundry project modules.
4. Replace `shape-presets` with semantic asset/foundry presets, then remove the
   old ShapeDocument preset crate from CLI paths.
5. Retire `shape-field` and implicit `ShapeDocument` meshing once all product
   preview/export paths compile from `shape-asset` artifacts.
