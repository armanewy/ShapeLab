# Wave 43 Parallel GameKit Foundations Report

Wave 43 starts headless backend work toward static-prop surface and handoff readiness while the Visual Foundry UI refactor remains independently owned.

## Scope

- No `shape-app` UI integration.
- No default product copy changes.
- No novice-facing material, UV, rigging, or animation controls.
- No game-ready claim unless required evidence passes.

## Added

- `shape_gamekit::gltf`
  - deterministic static-prop GLB 2.0 encoder
  - one-node/one-mesh geometry-only portable handoff
  - material slot IDs recorded as metadata only
  - GLB validation report covering header, chunk table, JSON payload, BIN payload, and primitive references

- `shape_gamekit::surface`
  - Surface Lab v0 package contracts
  - procedural material recipe metadata
  - semantic material slot bindings
  - UV policy and blockers
  - texture requirement blockers
  - unsupported texture/material output report
  - swatch-sheet evidence validation

- `shape-cli game-ready-static-prop --profile sci-fi-crate`
  - emits `sci-fi-crate-static-prop.glb`
  - emits `glb-validation-report.json`
  - emits `surface-lab-package.json`
  - emits `surface-lab-validation-report.json`
  - emits `material-pack.json`
  - emits `texture-requirements.json`
  - emits `unsupported-texture-report.json`
  - emits `surface-evidence/swatch-sheet.png`
  - expands crate material slots to painted body, dark recessed panels, vents, handles, fasteners, and edge trim

## Readiness Truth

The generated Sci-Fi Crate package is still blocked from a full game-ready claim.

Static-prop readiness blockers:

- `uv_layout_not_implemented`
- `manual_review_pending`

Surface Lab readiness blockers:

- `uv_layout_not_implemented`
- `texture_payload_policy_only`
- `base_color_texture_not_authored`
- `normal_texture_not_authored`
- `orm_texture_not_authored`
- `occlusion_texture_not_authored`

GLB handoff is now validated as a portable static mesh handoff, but it does not include UVs, textures, skinning, animation, or engine-native import.
It also does not claim slot-assigned GLB primitives; material slots are recorded as metadata for later assignment.

## Verification

Commands run:

```text
cargo fmt -p shape-gamekit -p shape-cli --check
cargo test -p shape-gamekit
cargo test -p shape-cli game_ready_static
cargo clippy -p shape-gamekit --all-targets -- -D warnings
cargo clippy -p shape-cli --bin shape-cli --no-deps -- -D warnings
cargo run -p shape-cli -- game-ready-static-prop --profile sci-fi-crate --out-dir target/game-ready/sci-fi-crate-static-prop-v2
```

`cargo clippy -p shape-cli --all-targets -- -D warnings` was also attempted, but full dependency linting was blocked by a `shape-app` warning in the concurrently edited UI refactor files.
