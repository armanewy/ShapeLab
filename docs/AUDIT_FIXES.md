# Audit fixes

This pass focused on release-blocking issues that prevented the native MVP from matching the intended architecture: local desktop app, deterministic candidate search, safe background jobs, no browser/Blender/LLM dependency, and category-independent implicit shape editing.

## Fixed

- Verified and preserved the `shape-app` integration against the pinned `eframe`/`egui` versions in this repository. `eframe 0.34.3` requires the `ui(...)` app hook here, and the app uses the non-deprecated `egui::Panel::*().show_inside(...)` APIs for native layout.
- Corrected the workspace `rust-version` to `1.92`, matching the pinned `eframe`/`egui` 0.34.3 dependency family's declared minimum supported Rust version.
- Confirmed the duplicate `#[derive(...)]` issue on `GridSamples` is not present in the integrated tree.
- Moved OBJ export onto the existing background job path from the state layer, so export no longer requires the UI coordinator to perform synchronous mesh file I/O.
- Removed the stale synchronous export app effect so exports have one application path: `AppCommand::ExportCurrentObj` schedules `JobRequest::ExportCurrent`.
- Made per-candidate compile/mesh/render failures non-fatal during generation. A single bad survivor no longer tears down the whole generation job and causes later valid candidate events to be treated as stale.

## Still intentionally out of scope

- Imported mesh semantic editing.
- Blender adapter.
- LLM/natural-language modeling.
- GPU renderer/compute backend.
- UVs, materials, rigging, animation.
- Structural graph mutation.

## Validation note

The original audit sandbox did not include `cargo`/`rustc`, but the integrated fixes were reconciled and validated locally with the normal project gate:

```bash
cargo fmt --all --check
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo build --workspace --release
cargo run -p shape-cli -- demo --preset desk-lamp --seed 100 --out-dir target/demo-release/lamp
cargo run -p shape-app --release
```
