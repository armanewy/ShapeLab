# Focus Part Surface Readiness Integration Report

## Summary

Both branches were merged into `codex/focus-surface-readiness-integration`:

- `codex/surface-preview-rig-motion-readiness`
- `codex/focus-part-perceptual-variation-v0`

The merged branch keeps Focus Part modeling and perceptual variation connected
to the latest headless surface evidence stack without exposing unfinished
Surface, rigging, skinning, or animation claims in the novice UI.

## Boundary Results

- Focus Part remains semantic part-group targeting, not raw mesh editing.
- Sci-Fi Crate has a static surface package and headless textured evidence.
- Visual Foundry Surface variation remains unavailable in the app.
- Focus Part Surface remains unavailable.
- Shape candidate legibility and Surface material delta reports stay separate.
- Material-only variants preserve frozen mesh identity and cannot claim shape
  changes.
- Rig and motion artifacts remain contracts and validation metadata only.
- Full game-ready status remains blocked without manual review and engine import
  proof.

## Package Evidence

The integrated static prop package is expected to emit:

- `surface/surface-artifact.json`
- `surface/surface-capabilities.json`
- `surface/uv-layout.png`
- `surface/material-swatch-sheet.png`
- `surface/texture-contact-sheet.png`
- `surface/textured-preview.png`
- `surface/textured-contact-sheet.png`
- `surface/variants/candidates.json`
- `surface/variants/contact-sheet.png`
- `surface/variants/*/surface-delta.json`
- `validation-report.json`

The package must remain blocked from full game-ready status until manual review
and engine import proof are present.

## Manual Screenshot Gate

Blocked in this run by the Windows automation helper before app launch:

```text
failed to write kernel assets: The system cannot find the path specified. (os error 3)
```

The helper was reset and retried once with the same result. No substitute
PowerShell UI automation was used.

Screens to inspect:

1. Choose screen
2. Sci-Fi Crate directions before generation
3. Generated directions
4. Selected candidate comparison
5. Focus Handles active
6. Focused handle candidates
7. Customize with focused control
8. Export with surface package copy

## Verification

Passed:

- `cargo fmt --all --check`
- `cargo test -p shape-foundry focus --jobs 1`
- `cargo test -p shape-foundry variation --jobs 1`
- `cargo test -p shape-foundry surface_capability --jobs 1`
- `cargo test -p shape-search foundry --jobs 1`
- `cargo test -p shape-render foundry --jobs 1`
- `cargo test -p shape-render surface --jobs 1`
- `cargo test -p shape-app foundry --jobs 1`
- `cargo test -p shape-app --test foundry_direction_board --jobs 1`
- `cargo test -p shape-gamekit surface --jobs 1`
- `cargo test -p shape-gamekit rig --jobs 1`
- `cargo test -p shape-gamekit motion --jobs 1`
- `cargo test -p shape-cli game_ready_static --jobs 1`
- `cargo test -p shape-foundry-catalog --test scifi_crate --jobs 1`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo build --release --workspace`
- `cargo run -p shape-cli -- game-ready-static-prop --profile sci-fi-crate --out-dir target/game-ready/sci-fi-crate-static-prop-v2`

Notes:

- The combined prompt command `cargo test -p shape-foundry focus variation surface_capability --jobs 1`
  was run as three valid Cargo test filters because Cargo accepts one test
  filter per invocation.
- The first gate attempt filled the local disk during `shape-cli
  game_ready_static`; disposable `target` directories in the two Shape Lab
  worktrees were deleted and the gates were rerun.
- Clippy initially found two merge-line issues in `shape-render` surface
  preview code; both were fixed before the final clippy and release build.

## Package Result

`target/game-ready/sci-fi-crate-static-prop-v2` was generated successfully.

Expected artifacts were present:

- `surface/surface-artifact.json`
- `surface/surface-capabilities.json`
- `surface/uv-layout.png`
- `surface/material-swatch-sheet.png`
- `surface/texture-contact-sheet.png`
- `surface/textured-preview.png`
- `surface/textured-contact-sheet.png`
- `surface/variants/candidates.json`
- `surface/variants/contact-sheet.png`
- six `surface/variants/*/surface-delta.json` files
- `validation-report.json`

Package status remained `Blocked`.

Blockers:

- `engine_import_proof_missing`
- `engine_native_package_not_implemented`
- `manual_review_pending`
- `surface_manual_review_required`

The generated `surface/surface-capabilities.json` reports headless
`surface_visual_evidence_ready: true`, while Foundry app Surface candidates and
Focus Part Surface remain unavailable.
