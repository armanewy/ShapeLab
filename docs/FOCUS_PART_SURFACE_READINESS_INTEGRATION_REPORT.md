# Focus Part Surface Readiness Integration Report

## Summary

Both branches were merged into `codex/focus-surface-readiness-integration`.

The merged branch keeps Focus Part modeling connected to the latest headless
surface evidence stack.

It does not expose unfinished Surface, rigging, skinning, or animation claims in
the novice UI.

## Merged Branches

- `codex/surface-preview-rig-motion-readiness`
- `codex/focus-part-perceptual-variation-v0`

## Boundary Results

- Focus Part remains semantic part-group targeting.
- Focus Part is not raw mesh editing.
- Sci-Fi Crate has a static surface package.
- Sci-Fi Crate has headless textured evidence.
- Visual Foundry Surface variation remains unavailable in the app.
- Focus Part Surface remains unavailable.
- Shape candidate legibility and Surface material delta reports stay separate.
- Material-only variants preserve frozen mesh identity.
- Material-only variants cannot claim shape changes.
- Rig artifacts remain contracts and validation metadata only.
- Motion artifacts remain contracts and validation metadata only.
- Full game-ready status remains blocked without manual review.
- Full game-ready status remains blocked without engine import proof.

## Integration Questions

- Were both branches merged?

  Yes.

- Did Focus Part still work after Surface Preview/Rig/Motion integration?

  The focused Foundry, search, render, app, and direction-board gates passed.

- Does Surface mode remain truthful in the UI?

  Yes.

  Surface package availability is separate from Visual Foundry Surface
  variation.

  Surface package availability is also separate from Focus Part Surface
  availability.

- Are shape and surface deltas kept separate?

  Yes.

  Foundry shape candidates use `CandidateVisibleDeltaReport`.

  Surface material variants use `SurfaceVisualDeltaReport`.

- Does the Sci-Fi Crate package include textured preview evidence?

  Yes.

- Does the Sci-Fi Crate package include material variants?

  Yes.

- Does the package remain blocked from full game-ready?

  Yes.

- Are rig/motion contracts hidden from novice UI?

  Yes.

  Product-visible copy tests reject rig, skinning, animation, and retargeting
  overclaims.

- Did manual screenshot verification pass?

  Not completed.

  The Windows automation helper failed before app launch.

- Did clippy pass?

  Yes.

- Did the release build pass?

  Yes.

- Are any failing workspace tests inherited from main?

  No failing gates remained in the commands run for this integration branch.

## Package Evidence

The integrated static prop package is expected to emit these files:

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

Manual screenshot verification was blocked in this run.

The Windows automation helper failed before app launch.

The error was:

```text
failed to write kernel assets: The system cannot find the path specified. (os error 3)
```

The helper was reset and retried once.

The retry failed with the same result.

No substitute PowerShell UI automation was used.

## Screens Still To Inspect

1. Choose screen
2. Sci-Fi Crate directions before generation
3. Generated directions
4. Selected candidate comparison
5. Focus Handles active
6. Focused handle candidates
7. Customize with focused control
8. Export with surface package copy

## Verification

These commands passed:

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

## Verification Notes

The combined prompt command below was split into separate Cargo invocations:

```text
cargo test -p shape-foundry focus variation surface_capability --jobs 1
```

Cargo accepts one test filter per invocation.

The first gate attempt filled the local disk during `shape-cli
game_ready_static`.

Disposable `target` directories in the two Shape Lab worktrees were deleted.

The gates were rerun after freeing disk space.

Clippy initially found two merge-line issues in `shape-render` surface preview
code.

Both issues were fixed before the final clippy and release build.

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

The generated `surface/surface-capabilities.json` reports
`surface_visual_evidence_ready: true`.

Foundry app Surface candidates remain unavailable.

Focus Part Surface remains unavailable.
