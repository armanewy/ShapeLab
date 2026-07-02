# Cleanup Plan

Date: 2026-07-01

Status: Starting cleanup baseline freeze.

## Baseline Assertion

Cleanup starts from `main` after the Phase A-D Semantic Compiler Integration
passed and was pushed. This baseline includes:

- `docs/PHASE_A_D_SEMANTIC_COMPILER_INTEGRATION_REPORT.md`
- `docs/CONTRACT_BOUNDARIES.md`
- AssetRecipe semantic shells in `shape-asset`
- `shape-authoring::AuthoringOpLog`
- `RelationshipContract` and `PatternContract`
- product claim gates and report include checks
- Direct Make Box width bridge through `AuthoringOp::SetProperty`
- Panel with Knob relationship migration
- export realization reporting

If any of these disappear, cleanup must stop until the semantic compiler
baseline is restored.

## Cleanup Rules

No backward compatibility is required because Shape Lab / Object Orchard has not
shipped. Obsolete pivots should be deleted, not preserved through compatibility
shims.

`shape-core` is legacy/implicit unless active code still requires a low-level
type or compatibility convention. New product semantics belong in the explicit
semantic asset lane:

- `shape-asset::AssetRecipe` / Orchard IR
- `shape-authoring::AuthoringOpLog`
- `RelationshipContract`
- `PatternContract`
- export/proof includes and realization reports

Cleanup must not add product capability. It must not introduce material,
texture, UV, collision, motion, terrain, runtime LLM, public catalog, Godot-ready,
or game-ready claims.

## Priority Order

1. Functional
2. Accurate
3. Performant
4. Elegant

Correct behavior and truthful product claims take precedence over aesthetic
refactors. Elegance is valuable only after functionality and accuracy are
preserved.

## Wave Order

1. Freeze cleanup baseline and add hygiene gates.
2. Purge obsolete docs.
3. Remove legacy candidate/search product paths.
4. Split oversized app and semantic core modules.
5. Purge dead code, fixtures, dependencies, and stale scripts.
6. Retire shape-core product dependencies where safe.
7. Integrate cleanup before any rename.
8. Rename product-facing strings to Object Orchard.
9. Rename Rust crates/folders and command examples.
10. Clean repository-local paths, scripts, and environment variables.
11. Purge final legacy name references.
12. Run final cleanup and rename integration gate.
