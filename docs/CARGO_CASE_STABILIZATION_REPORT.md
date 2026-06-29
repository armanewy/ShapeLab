# Cargo Case Proof Stabilization Report

Date: 2026-06-29

Status: PASS for stabilization.

This branch stabilizes the Cargo Case architecture proof after integration. It
does not add product features, broaden Surface mode, add UV/texturing support,
add rigging/animation support, or claim full game-ready status.

## Source Hygiene

See `docs/CARGO_CASE_SOURCE_HYGIENE_REPORT.md`.

Checked files are audit-friendly and under the 180-character line threshold:

- `crates/shape-app/src/foundry/app.rs`
- `crates/shape-foundry-catalog/src/cargo_case.rs`
- `crates/shape-foundry-catalog/src/scifi_crate.rs`
- `crates/shape-foundry-catalog/src/lib.rs`
- `README.md`

The requested Cargo Case status reports were also checked for line hygiene.
None has a line longer than 180 characters after stabilization.

## Surface Evidence Compatibility

Default Sci-Fi Crate material-look evidence was regenerated against the Cargo
Case output:

```bash
cargo run -p shape-cli -- game-ready-static-prop \
  --profile sci-fi-crate \
  --out-dir target/surface-candidate-evidence-v0/sci-fi-crate
```

Evidence hash:

- `target/surface-candidate-evidence-v0/sci-fi-crate/validation-report.json`
  SHA-256 `702c692b7ea079f98e2f622f9d6b8df758e60b8cda8e01171264e7f949103c97`

Release-app dogfood also verified that after applying an idea, material looks are
not silently reused against changed geometry. The app showed:

- `SURFACE ONLY Material looks Geometry unchanged`
- `Material looks unavailable`
- `Material looks do not match this crate build.`

That behavior is an honest stale-disabled state for the changed build.

## Static Package Check

Command:

```bash
cargo run -p shape-cli -- game-ready-static-prop \
  --profile sci-fi-crate \
  --out-dir target/cargo-case-stabilization/scifi-crate-static-prop
```

Result: package generation completed, but full game-ready remains blocked.

Validation hash:

- `target/cargo-case-stabilization/scifi-crate-static-prop/validation-report.json`
  SHA-256 `702c692b7ea079f98e2f622f9d6b8df758e60b8cda8e01171264e7f949103c97`

Recorded blocker codes:

- `engine_import_proof_missing`
- `engine_native_package_not_implemented`
- `manual_review_pending`
- `surface_manual_review_required`

## Release-App Dogfood

App under review:

- `target/release/shape-app`

Manual UI path recorded:

1. Choose `Sci-Fi Industrial Crate` from the novice catalog.
2. Start the asset and wait for Make to reach `Ready`.
3. Run `Try ideas`.
4. Use one idea.
5. Open material looks.
6. Open export.

Evidence files:

- `target/cargo-case-stabilization/dogfood/cargo-case-stabilization-dogfood.mov`
  SHA-256 `cb6f65aac89ceb2dd67966b1e8baa47c1c37a3fb7b172c1cf614433b01689bf9`
- `target/cargo-case-stabilization/dogfood/12_reset-before-foreground-recording.png`
  SHA-256 `9db95724c048a3d60c88d9cd9139ed976325a64efc1b4d32f066ee0945b312e7`
- `target/cargo-case-stabilization/dogfood/13_after-video-final-state.png`
  SHA-256 `d3e7fc9cf32e39e54fa6fbeadef1b3863e04c1ff5898562c69387055f08f5f7f`
- `target/cargo-case-stabilization/dogfood/06_ideas-visible.png`
  SHA-256 `830279234748d2f903df2ae1b5494bcb6d107e2b2d98f5705faabe5ebe8116e1`
- `target/cargo-case-stabilization/dogfood/07_idea-selected-ready.png`
  SHA-256 `b3022129f69e646078ac254149f5e21e62c024820a7b3c0828ee0d9ff5e910ed`
- `target/cargo-case-stabilization/dogfood/08_material-looks.png`
  SHA-256 `ee92910d3662591c551e0f5256074b551fc30300914bde384c933c9ea4aecefa`
- `target/cargo-case-stabilization/dogfood/09_export-truth.png`
  SHA-256 `bb8053f7ee607078b818525956648b4eefa018b0f698c7e1bb37aa99d6ba9398`

Dogfood findings:

- Sci-Fi Industrial Crate opens from the novice catalog.
- Make reaches `Ready`.
- `Try ideas` found 6 clear ideas.
- One idea could be applied and returned to `Ready`.
- Material looks are visible as surface-only and are stale-disabled after the
  changed build.
- Export copy remains scoped to current asset readiness and does not claim full
  game-ready output.
- The app does not expose broad Surface, UV/Texturing, rigging, or animation
  claims in this path.
