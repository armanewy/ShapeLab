# MVP Report

## Scope

Shape Lab is a native, offline desktop MVP for preference-guided procedural 3D modeling. The release proves that a user can start from a non-humanoid implicit shape graph, generate coherent candidate futures, choose one, and continue with branchable history.

The app has no browser frontend, server runtime, hosted AI dependency, or DCC dependency. The model/search core remains independent from the desktop shell.

## Delivered Capabilities

- Native `eframe`/`egui` desktop app with viewport, outliner, inspector, history, candidate gallery, menus, and status line.
- Four category-independent built-in presets: Desk Lamp, Toy Submarine, Alien Plant, and Sky Shrine.
- CPU implicit-field compiler for primitive and CSG shape graphs.
- CPU marching-tetrahedra meshing and deterministic OBJ export.
- CPU preview renderer with orbit camera, fit, pan, zoom, thumbnails, and cache-key infrastructure.
- Deterministic candidate generation with Refine/Explore modes, locks, target scopes, diagnostics, duplicate suppression, and fallback passes.
- Branchable project history with undo, branch switching, JSON save/load, future-schema rejection, and atomic project writes.
- Headless CLI demo generation for reproducible PNG/OBJ/project artifacts.
- Release infrastructure for CI checks, demo contact sheets, packaging notes, dependency documentation, and placeholder icons.

## Verification Summary

Final release verification is performed from `wave-6-mvp-release` on Windows.

Core commands:

```bash
cargo fmt --all --check
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo build --workspace --release
```

Demo commands generate release artifacts under `target/demo-release`:

```bash
cargo run -p shape-cli -- demo --preset desk-lamp --seed 100 --out-dir target/demo-release/lamp
cargo run -p shape-cli -- demo --preset toy-submarine --seed 200 --out-dir target/demo-release/submarine
cargo run -p shape-cli -- demo --preset alien-plant --seed 300 --out-dir target/demo-release/plant
cargo run -p shape-cli -- demo --preset sky-shrine --seed 400 --out-dir target/demo-release/shrine
```

The desktop app is release-built and smoke-started locally. Full interactive UX checks remain manual and are tracked in `docs/MANUAL_TEST_CHECKLIST.md`.

Additional release probes:

- Validated every `target/demo-release/*/project-after.json`.
- Confirmed OBJ outputs are present and nonempty for every release demo.
- Re-ran an identical Desk Lamp CLI demo twice and compared every output file by SHA-256.
- Confirmed malformed JSON, future schema-version JSON, and invalid export paths fail clearly.
- Visually inspected every release contact sheet for nonblank parent and candidate renders.

## Release Artifacts

- Release binary: `target/release/shape-app.exe`
- CLI demo outputs: `target/demo-release/*`
- Contact sheets: `target/demo-release/*/contact-sheet.png`
- Project JSON and OBJ exports: `target/demo-release/*`

## Platforms

Actually tested locally:

- Windows, PowerShell, native Rust toolchain.

Configured but not locally verified:

- Linux and macOS CI release builds.
