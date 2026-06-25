# Surface Channel Integration Report

## Summary

Both requested branches were merged in order:

1. `codex/foundry-variation-scope-channel`
2. `codex/surface-lab-static-prop-v1`

The integration adds a product-facing Foundry surface capability bridge and
keeps Surface Lab as a headless static-prop artifact provider.

## Answers

- Were both branches merged? Yes.
- Does Sci-Fi Crate expose surface package availability? Yes. It reports a
  static-prop surface package with UVs, material slots, texture files, and
  evidence.
- Is Surface visual variation still disabled unless evidence supports it? Yes.
  The current blocker is textured preview rendering plus material candidate
  support.
- Are shape and surface deltas kept separate? Yes. Existing candidate metadata
  keeps shape and surface delta scores separate, and tests cover the boundary.
- Does Export describe Surface Lab honestly? Yes. Export copy says surface
  package, describes the concrete sidecars, and keeps full game-ready status
  blocked.
- Do full workspace tests and clippy pass? Clippy passes. Full workspace tests
  fail only in two targets that were reproduced on `origin/main`: `shape-cli
  --test cli_demo` and `shape-foundry-catalog --test moba_hero`.
- Are any previously unrelated failures still present? Yes. The inherited
  failures are:
  - `release_readiness_verifies_product_ui_gate_when_requested` exits with
    `Some(10)` instead of `Some(0)` because `--enable-bend requires
    --package-schema 3`.
  - `moba_hero_candidate_modes_return_six_survivors_with_human_explanations`
    returns four silhouette candidates instead of six.
- Did this branch avoid UI overclaiming and runtime LLM integration? Yes. It
  adds no textured Visual Foundry viewport, material editor, Surface candidate
  generator, Focus Part Surface editor, engine exporter, or runtime LLM path.

## Implemented

- Added `FoundrySurfaceCapabilityView` in `shape-foundry`.
- Added a parser for `surface/surface-capabilities.json` that rejects malformed
  data and absolute local paths.
- Mapped Surface Lab sidecar evidence into product-facing capability state
  without enabling Surface UI mode.
- Updated Directions disabled Surface copy for Sci-Fi Crate.
- Updated Export copy to say "Static prop surface package available" and keep
  the full-ready blocker.
- Added tests for surface package availability, disabled Surface mode, malformed
  sidecar rejection, product-safe unavailable reasons, and export copy.

## Verification

- `cargo fmt --all --check`: passed.
- `cargo test -p shape-foundry surface_capability --jobs 1`: passed.
- `cargo test -p shape-app --test foundry_direction_board --jobs 1`: passed.
- `cargo test -p shape-search --test foundry_candidates --jobs 1`: passed.
- `cargo test --workspace --no-fail-fast --jobs 1` with
  `CARGO_PROFILE_TEST_DEBUG=0`: failed only in the two inherited baseline
  targets listed above.
- `cargo clippy --workspace --all-targets -- -D warnings`: passed.
- `cargo build --release --workspace`: passed.
- `cargo run -p shape-cli -- game-ready-static-prop --profile sci-fi-crate
  --out-dir target/game-ready/sci-fi-crate-static-prop-v1`: passed and reported
  `game_ready: false`.

The generated Sci-Fi Crate package includes `surface/surface-artifact.json`,
`surface/surface-capabilities.json`, `surface/uv-layout.png`,
`surface/material-swatch-sheet.png`, `surface/texture-contact-sheet.png`, and
20 PNG texture files under `surface/textures/`. Its validation report remains
`Blocked` for engine import proof, engine-native package handoff, manual
review, and surface manual review.
