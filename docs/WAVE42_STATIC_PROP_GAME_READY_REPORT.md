# Wave 42 Static Prop Game-Readiness Report

Wave 42 adds the first static-prop game-readiness package for the Sci-Fi Crate
Foundry profile.

## Added

- `shape_gamekit::export` static-prop manifest and readiness validation
  contracts.
- `shape-cli game-ready-static-prop --profile sci-fi-crate`.
- Frozen canonical model-package output plus grouped OBJ handoff.
- Deterministic proxy LOD OBJs:
  - LOD0: exact canonical model package.
  - LOD1: compiled-bounds proxy OBJ.
  - LOD2: collision-proxy OBJ.
- Runtime-neutral `game-asset-pack.json` with collision, footprint, readability,
  triangle budget, and internal dogfood export profile.
- Material-slot assignments, UV policy, Blender handoff marker, GLB blocker,
  visual evidence PNGs, and manual review marker.

## Truth Boundary

This wave does not claim a finished game-ready asset. The generated
`validation-report.json` is expected to be blocked until:

- UV layout exists.
- Direct GLB handoff exists.
- Manual DCC/runtime import review is completed.

The package is useful for handoff and verification while preserving the release
truth gate.

## Command

```bash
cargo run -p shape-cli -- game-ready-static-prop --profile sci-fi-crate --out-dir target/game-ready/sci-fi-crate-static-prop-v1
```

## Verification

Focused verification for this wave:

```bash
cargo fmt --all --check
cargo test -p shape-gamekit
cargo test -p shape-cli game_ready_static
cargo run -p shape-cli -- game-ready-static-prop --profile sci-fi-crate --out-dir target/game-ready/sci-fi-crate-static-prop-v1
cargo test -p shape-foundry-catalog scifi_crate
cargo test -p shape-cli release_readiness
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace --no-fail-fast
cargo build --release --workspace
```
