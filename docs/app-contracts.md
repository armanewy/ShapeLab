# App Contracts

Wave 3 UI modules must communicate through these contracts rather than doing geometry or I/O directly.

## Commands And Effects

Panels emit `AppCommand` values from `crates/shape-app/src/commands.rs`.

`AppState` applies lightweight deterministic changes and returns `AppEffect` values for heavy work:

- geometry and rendering become `AppEffect::StartJob(Box<JobRequest>)`
- save/load/export remain effects
- UI modules must not call `shape-field`, `shape-mesh`, `shape-search`, or filesystem APIs directly

## State Ownership

`AppState` owns the `Project`, selected node, target scope, enabled parameter groups, exploration mode, seed, generation counter, preview caches, candidate slots, orbit camera, quality settings, active preset, file path, dirty flag, status text, recoverable errors, active job IDs, and active generation ID.

State methods must never leave a partially mutated project. Parameter edits go through `shape-core`; accepting a candidate goes through `shape-project`.

## Background Jobs

`JobRequest` and `JobEvent` are defined in `crates/shape-app/src/jobs.rs`.

Worker threads may return:

- `TriangleMesh`
- `RenderedImage`
- `CandidatePreview`
- status/progress/error events

Worker threads must never touch `egui::Context`, `egui::TextureHandle`, or any other egui type.

Every request has a `JobId`. Candidate generation also has a `GenerationId`. The UI/state layer must ignore stale events whose IDs do not match the current active job or generation.

## Texture Ownership

`RenderedImage` is the boundary between workers and egui. The app coordinator creates and updates `egui::TextureHandle` values only on the UI thread.

## Candidate Preview

`CandidatePreview` contains a stable slot index, the `Candidate`, its mesh, and a CPU-rendered thumbnail image. Slot ordering must remain stable even if worker completion order differs.

## Viewport

The viewport emits `ViewportAction` values from `crates/shape-app/src/viewport.rs`.

The viewport may update camera math locally for interaction, but rerendering is requested through actions/effects. It must not mutate the project or run geometry work.
