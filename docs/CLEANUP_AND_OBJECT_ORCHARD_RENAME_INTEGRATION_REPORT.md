# Cleanup and Object Orchard Rename Integration Report

Date: 2026-07-01

Branch: `codex/cleanup-rename-final-integration`

## Result

The cleanup and rename phase is integrated in-repo. The product identity,
workspace package names, crate folders, command examples, environment variables,
project suffixes, generated evidence paths, metadata namespaces, and product
status docs now use Object Orchard naming.

BLOCKED-MANUAL:
Repository still named ShapeLab on GitHub.
Manual step required:
Settings -> Repository name -> ObjectOrchard or object-orchard.

Current remote:

```text
https://github.com/armanewy/ShapeLab.git
```

`git ls-remote origin` succeeds for `main`, so the remote is reachable, but the
GitHub repository setting has not been renamed.

## Expected Branches

| Branch | Integration Status |
| --- | --- |
| `codex/cleanup-baseline-freeze` | Merged before this gate |
| `codex/obsolete-documentation-purge` | Merged before this gate |
| `codex/remove-legacy-candidate-search-paths` | Merged before this gate |
| `codex/split-shape-app-foundry-modules` | Merged before this gate |
| `codex/split-semantic-core-crates` | Merged before this gate |
| `codex/workspace-dead-code-dependency-purge` | Merged before this gate |
| `codex/shape-core-legacy-retirement-pass` | Merged before this gate |
| `codex/cleanup-wave-1-integration` | Merged before this gate |
| `codex/product-facing-object-orchard-rename` | Merged before this gate |
| `codex/rust-crate-folder-object-orchard-rename` | Merged before this gate |
| `codex/repository-path-script-rename-cleanup` | Merged before this gate |
| `codex/final-legacy-name-purge` | Merged before this gate |

## Architecture Verification

| Question | Result |
| --- | --- |
| Canonical semantic lane remains AssetRecipe / Orchard IR | Pass. `docs/CONTRACT_BOUNDARIES.md`, `docs/ARCHITECTURE_STATUS.md`, and `README.md` keep AssetRecipe / Orchard IR as the explicit semantic lane. |
| AuthoringOpLog remains canonical mutation boundary | Pass. `orchard-authoring` defines typed product-visible authoring logs, and the Direct Make bridge emits AuthoringOp changes. |
| RelationshipContract remains composition boundary | Pass. Semantic shells own relationship contracts, and Panel with Knob materialization uses constrained composition. |
| Product claim gate remains active | Pass. `orchard-app` product gate tests reject old identity, candidate, and overclaim strings from default UI copy. |
| `orchard-core-legacy` is low-level/legacy only | Pass. The retained crate is named `orchard-core-legacy`, documented as non-canonical, and no longer a direct app dependency. |
| Generated candidate UI remains out of product path | Pass. The old candidate UI is behind `OBJECT_ORCHARD_LEGACY_CANDIDATE_UI`; direct primitive workflows disable it, and default product tests assert no candidate tray/comparison in Direct Make. |

## Product Verification

Focused app/product smoke passed with:

```text
cargo test -p orchard-app foundry --jobs 1
```

The smoke includes deterministic coverage for:

- Box Primitive, Flat Panel Primitive, Sphere Primitive, and Panel with Knob
  baseline Make flows.
- Direct Make reaching ready state.
- Add to Pack / Export readiness from the exact-value panel.
- Default product copy excluding generated candidate UI.
- Direct primitive stale-background warnings being suppressed.
- Product docs and default UI copy keeping unsupported material, UV, rigging,
  animation, public catalog, Godot-ready, and game-ready claims blocked.

## Export and ObjectPlan Verification

| Area | Result |
| --- | --- |
| ObjectPlan materialization | Pass. `orchard-cli` and `orchard-foundry` tests cover supported plans, invalid plans, raw mesh rejection, publish rejection, and deterministic outputs. |
| Geometry-only GLB export | Pass. CLI tests cover box, flat panel, sphere, panel with knob, blocked texture requests, blocked game-ready requests, unresolved plans, and deterministic GLB output. |
| Godot proof harness | Pass for V0 contract. CLI tests cover missing Godot binary as `Blocked`, invalid GLB as `Failed`, deterministic unavailable output, and unit classification for `Passed` / `Blocked` / `Failed`. Local import proof remains blocked until a Godot binary is available. |
| Export realization report | Pass. Tests cover combined mesh reporting without false relationship-bake claims. |
| Product claim tests | Pass. Foundry/product tests and foundry product-claim tests keep later capabilities explicitly blocked. |

## Final Crate Mapping

| Previous package/folder | Current package/folder |
| --- | --- |
| `shape-app` | `orchard-app` |
| `shape-cli` | `orchard-cli` |
| `shape-asset` | `orchard-asset` |
| `shape-authoring` | `orchard-authoring` |
| `shape-core` | `orchard-core-legacy` |
| `shape-modeling` | `orchard-modeling` |
| `shape-modeling-assets` | `orchard-modeling-assets` |
| `shape-compile` | `orchard-compile` |
| `shape-foundry` | `orchard-foundry` |
| `shape-foundry-catalog` | `orchard-foundry-catalog` |
| `shape-render` | `orchard-render` |
| `shape-project` | `orchard-project` |
| `shape-search` | `orchard-search-internal` |
| `shape-field` | `orchard-field` |
| `shape-mesh` | `orchard-mesh` |
| `shape-family` | `orchard-family` |
| `shape-family-compile` | `orchard-family-compile` |
| `shape-poly` | `orchard-poly` |
| `shape-presets` | `orchard-presets` |

Current workspace packages:

```text
orchard-app
orchard-asset
orchard-authoring
orchard-cli
orchard-compile
orchard-core-legacy
orchard-family
orchard-family-compile
orchard-field
orchard-foundry
orchard-foundry-catalog
orchard-mesh
orchard-modeling
orchard-modeling-assets
orchard-poly
orchard-presets
orchard-project
orchard-render
orchard-search-internal
```

## Deleted Crates, Docs, Scripts, and Fixtures

Cleanup Wave 1 removed obsolete pivots and dead workspace members:

- Deleted obsolete `shape-decompiler`.
- Deleted obsolete `shape-program`.
- Deleted obsolete `shape-program-verify`.
- Deleted stale legacy Shape Asset JSON fixtures tied to removed coverage.
- Deleted the obsolete Box Primitive UI cleanup script.
- Purged obsolete Sci-Fi Crate, Cargo Case, crate-family pivot, and generated
  variation docs from the active documentation index.

Wave 2 then renamed product-facing docs, Rust packages/folders, script entry
points, environment variables, target evidence paths, project suffixes, and DCC
metadata namespaces to Object Orchard forms.

## Final Line-Count Report

`python3 scripts/check_rust_file_size.py` passes. Files over 1000 non-test lines
remain only as documented temporary exceptions in `docs/RUST_FILE_SIZE_EXCEPTIONS.md`.

Current exceptions:

| Path | Non-test lines | Owner |
| --- | ---: | --- |
| `crates/orchard-app/src/foundry/jobs.rs` | 1349 | App cleanup |
| `crates/orchard-app/src/foundry/panels/directions.rs` | 1105 | App cleanup |
| `crates/orchard-app/src/foundry/panels/history.rs` | 1207 | App cleanup |
| `crates/orchard-app/src/foundry/state.rs` | 2319 | App cleanup |
| `crates/orchard-cli/src/main.rs` | 3295 | CLI cleanup |
| `crates/orchard-cli/src/object_plan_cli.rs` | 2378 | CLI cleanup |
| `crates/orchard-compile/src/export/package.rs` | 1045 | Compile cleanup |
| `crates/orchard-compile/src/lib.rs` | 1485 | Compile cleanup |
| `crates/orchard-compile/src/validation/mod.rs` | 2028 | Compile cleanup |
| `crates/orchard-core-legacy/src/lib.rs` | 1432 | Legacy boundary cleanup |
| `crates/orchard-family/src/lib.rs` | 2560 | Family cleanup |
| `crates/orchard-family-compile/src/lib.rs` | 3258 | Family compile cleanup |
| `crates/orchard-family-compile/src/remap/assembly.rs` | 1210 | Family compile cleanup |
| `crates/orchard-family-compile/src/remap/ports.rs` | 1133 | Family compile cleanup |
| `crates/orchard-foundry-catalog/src/box_primitive.rs` | 1051 | Catalog cleanup |
| `crates/orchard-foundry-catalog/src/lib.rs` | 1095 | Catalog cleanup |
| `crates/orchard-poly/src/lib.rs` | 1888 | Polygon cleanup |
| `crates/orchard-project/src/asset.rs` | 1017 | Project cleanup |
| `crates/orchard-project/src/foundry.rs` | 1465 | Project cleanup |
| `crates/orchard-render/src/foundry/mod.rs` | 1496 | Render cleanup |
| `crates/orchard-render/src/lib.rs` | 1540 | Render cleanup |
| `crates/orchard-search-internal/src/asset/mod.rs` | 2818 | Search cleanup |
| `crates/orchard-search-internal/src/asset/scoring.rs` | 1087 | Search cleanup |
| `crates/orchard-search-internal/src/foundry/mod.rs` | 4323 | Search cleanup |
| `crates/orchard-search-internal/src/lib.rs` | 1935 | Search cleanup |

## Final Product-Name Audit

Exact legacy name search results are limited to intentional migration notes:

- `docs/OBJECT_ORCHARD_NAMING_TRANSITION.md`
- `docs/OBJECT_ORCHARD_REPOSITORY_RENAME_GUIDE.md`
- this integration report

`SHAPE_LABELS` hits are ordinary geometry label constants, not old product
identity.
Broad `shape-` / `shape_` hits remain only as historical cleanup records,
previous crate mapping records, or ordinary geometry/domain vocabulary such as
`shape_delta`, `shape_ready`, and `shape_vec3_is_finite`.

## Cleanup Inventory

`python3 scripts/audit_cleanup_inventory.py` reports:

- no potentially unused workspace crates;
- known temporary line-count exceptions;
- legacy candidate/try-ideas strings only in the explicitly gated legacy
  candidate UI path and tests;
- unsupported product-claim strings only in blocked/negative claim contexts or
  tests that assert those claims stay blocked.

The legacy candidate UI path is not certified as future product direction; it is
retained only as env-gated internal recovery/evidence code and remains outside
direct primitive product workflows.

## Manual Steps Still Required

1. Rename the GitHub repository in Settings.
2. Update local remotes after the GitHub rename:

   ```bash
   git remote set-url origin git@github.com:armanewy/ObjectOrchard.git
   ```

   or the HTTPS equivalent.

3. Verify:

   ```bash
   git remote -v
   git ls-remote origin
   ```

4. Rerun the Godot proof on a machine with Godot installed if Godot-ready
   geometry claims are desired later. Until then, the proof remains Blocked, not
   Passed.

## Final Gates

| Gate | Result |
| --- | --- |
| `cargo fmt --all --check` | Passed |
| `python3 scripts/check_source_hygiene.py` | Passed |
| `python3 scripts/check_rust_file_size.py` | Passed |
| `python3 scripts/audit_cleanup_inventory.py` | Passed |
| `cargo metadata --format-version 1` | Passed |
| `cargo test --workspace --jobs 1` | Passed |
| `cargo clippy --workspace --all-targets -- -D warnings` | Passed |
| `cargo build --release --workspace` | Passed |
