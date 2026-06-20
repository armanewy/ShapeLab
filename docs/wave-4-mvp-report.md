# Wave 4 MVP Report

## Delivered Native Loop

- Replaced the bootstrap `shape-app` shell with an `eframe` coordinator that owns `AppState`, background workers, render textures, viewport interaction state, and panel state.
- Built the default Desk Lamp preview on startup without blocking the UI.
- Wired top menus, status bar, viewport, outliner, inspector, history, and candidate gallery into one desktop workspace.
- Routed preview rebuilds, camera rerenders, candidate generation, save/open, and OBJ export through app commands and effects.
- Loaded rendered preview images into egui textures on the UI thread.
- Displayed current-shape and candidate thumbnails in the gallery, with loading placeholders while generation is active.
- Preserved recoverable error reporting in the UI instead of crashing on worker, file, or export failures.

## User Workflow

The release app can be started with:

```bash
cargo run -p shape-app --release
```

The main loop is:

1. Load or reset to a built-in preset.
2. Inspect and edit scalar parameters in the inspector.
3. Use the viewport to orbit, pan, zoom, or fit the current shape.
4. Generate directions from the current semantic graph.
5. Compare candidate thumbnails and parameter differences.
6. Accept a candidate to create a new project revision.
7. Save/open project JSON or export the current preview mesh as OBJ.

## Verification

Wave 4 verification:

```bash
cargo fmt --all --check
cargo check -p shape-app
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo build -p shape-app --release
```

A short release-binary startup check was also run by launching `target\release\shape-app.exe` and stopping it after startup.

## Known Limitations

- The native startup check is process-level; this wave does not include automated desktop-window pixel inspection.
- Candidate generation is deterministic and visible, but search quality still needs later tuning for semantic breadth and usefulness.
- Rendering remains CPU-based and preview-oriented.
- Imported meshes are still not semantically editable.
