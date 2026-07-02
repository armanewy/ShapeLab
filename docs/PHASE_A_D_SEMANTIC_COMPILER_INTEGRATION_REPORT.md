# Phase A-D Semantic Compiler Integration Report

Date: 2026-07-01

Status: Passed.

## Merged Branches

| Order | Branch | Tip |
| --- | --- | --- |
| 1 | `codex/phase-a-contract-boundaries-docs` | `158730b` |
| 2 | `codex/shape-core-legacy-boundary-guard` | `aea042f` |
| 3 | `codex/asset-recipe-v8-semantic-shells` | `2fab6b0` |
| 4 | `codex/authoring-op-log-v0` | `0955800` |
| 5 | `codex/relationship-pattern-contract-shells-v0` | `86c7239` |
| 6 | `codex/product-claim-gate-report-includes` | `6cf0848` |
| 7 | `codex/kernel-registry-property-bridge-v0` | `1853b92` |
| 8 | `codex/direct-make-authoring-op-bridge-v0` | `c8ccfe4` |
| 9 | `codex/panel-knob-relationship-migration-v0` | `b1213eb` |
| 10 | `codex/pattern-contract-evaluation-proof-v0` | `992a3ec` |
| 11 | `codex/export-realization-report-v0` | `e3dc34a` |

## Proof Questions

| Question | Answer |
| --- | --- |
| Is `shape-asset` / `AssetRecipe` documented as canonical? | Yes. `docs/CONTRACT_BOUNDARIES.md`, `docs/CURRENT_PRODUCT_STATUS.md`, and `README.md` describe `AssetRecipe` / Orchard IR as the canonical future semantic lane. |
| Is `shape-core` documented as legacy/implicit for A-J work? | Yes. `docs/SHAPE_CORE_LEGACY_BOUNDARY.md` and crate-level docs keep `ShapeDocument` in the legacy/implicit compatibility lane. |
| Can `AssetRecipe` carry relationship/pattern/surface/collision/motion/terrain/export/authoring shells? | Yes. AssetRecipe v8 shells carry these semantic placeholders and validation boundaries without making them product-facing features. |
| Does `AuthoringOpLog` exist and replay? | Yes. `shape-authoring` provides typed operation logs and replay tests. |
| Does at least one product-visible primitive edit use `AuthoringOp`? | Yes. Box Primitive width is bridged through `AuthoringOp::SetProperty` while preserving current Direct Make behavior. |
| Can Panel with Knob be represented via `RelationshipContract`? | Yes. Panel with Knob materialization produces a `SurfaceMounted` relationship from `front_handle_zone` to `back_mount_point`. |
| Are fixed-distance and proportional placement tested? | Yes. Panel-with-Knob relationship tests cover fixed edge distance and proportional placement behavior. |
| Does `PatternContract` evaluate deterministically for a minimal pattern? | Yes. A minimal linear pattern evaluator emits deterministic occurrence IDs and reports invalid count/spacing blockers. |
| Does export report relationship realization? | Yes. Geometry export reports include `relationship_realizations`; Panel with Knob reports combined-mesh output, `baked: false`, and sidecar/report semantic preservation. |
| Are game-ready/material/collision/motion/terrain claims still blocked? | Yes. Product-claim gates and export/Godot report includes keep these capabilities false or explicitly blocked. |

## Current Product Regression

Automated Foundry regression remains the integration gate for current product behavior:

- Box Primitive opens and edits through existing foundry tests.
- Flat Panel opens and edits through existing foundry tests.
- Sphere opens and edits through existing foundry tests.
- Panel with Knob opens and remains valid through composition/foundry tests.
- Generated idea/candidate UI must not reappear in active Direct Make tests.
- No game-ready or Godot-ready claim may appear without proof.

Native screenshots are not required for this integration gate. If a visual gate is needed later, use deterministic UI state snapshots rather than stale macOS capture as the hard assertion.

## Hard Gate Results

| Gate | Result |
| --- | --- |
| `cargo fmt --all --check` | Passed |
| `python3 scripts/check_source_hygiene.py` | Passed |
| `cargo test -p shape-asset --jobs 1` | Passed |
| `cargo test -p shape-authoring --jobs 1` | Passed |
| `cargo test -p shape-modeling --jobs 1` | Passed |
| `cargo test -p shape-compile --jobs 1` | Passed |
| `cargo test -p shape-cli object_plan --jobs 1` | Passed |
| `cargo test -p shape-cli godot --jobs 1` | Passed |
| `cargo test -p shape-app foundry --jobs 1` | Passed |
| `cargo clippy --workspace --all-targets -- -D warnings` | Passed |
| `cargo build --release --workspace` | Passed |

## Next Allowed Work

If these gates pass, the next allowed work is:

- Orchard Stretch Handles as `AuthoringOp` emitters;
- Relationship UI for attachment policies;
- Surface V0 over AssetRecipe shells;
- TerrainPatch contract implementation;
- Collision contract implementation.

Still blocked:

- runtime LLM inside the app;
- public catalog publishing;
- game-ready claims;
- material editor UI;
- UV editor UI;
- arbitrary rigging/animation UI.
