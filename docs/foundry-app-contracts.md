# Foundry App Contracts

The native Foundry application boundary is implemented as the whole-model
Visual Foundry product app. Wave 31 removed the old Asset Modeling Lab wrapper,
legacy implicit mode, explicit Modeling Workspace switcher, and default
Advanced Recipe product surface.

## Modules

The native Foundry surface is rooted at `crates/shape-app/src/foundry`:

- `state.rs` owns `FoundryAppState`, the UI-independent session snapshot.
- `commands.rs` owns `FoundryAppCommand`, the app command boundary.
- `jobs.rs` owns `FoundryJobRequest` and `FoundryJobEvent`.
- `view_model.rs` owns `FoundryCandidateCard`, `FoundryControlView`,
  `FoundryOptionCard`, and `FoundryPackView`.
- `app.rs` owns the native egui host, background worker coordinator, file
  dialogs, and built-in catalog resolver.
- `panels/*` own toolkit-agnostic view helpers and command intent helpers.

## Command Boundary

`FoundryAppCommand` does not define semantic edits such as set-control,
provider selection, candidate acceptance, export, undo, or pack insertion.
Those remain `shape_foundry::FoundryCommand` values, either wrapped one at a
time or carried as an ordered command program.

UI-only commands are limited to app concerns:

- selection
- requesting build or preview jobs
- requesting project file save/load
- recording whether developer-only technical rows are expanded
- requesting app-hosted pack export with a destination directory

This keeps automation, replay, persistence, and native UI on the same generic
Foundry command contract.

## Job Boundary

`FoundryJobRequest` describes work that must run off the UI thread:

- exact document compilation
- whole-model preview rendering
- candidate generation, with `FoundryCandidateRequest` as the single source of
  search mode and deterministic proposal settings
- replayable edit application
- pack compilation
- current asset export
- pack export

`FoundryJobEvent` returns compiled outputs, previews, candidates, pack views,
export completion, pack export completion, or failure diagnostics with the same
app-local job ID so the state reducer can reject stale results
deterministically.

## View Models

The view models are whole-model first:

- `FoundryCandidateCard` represents unchanged parent and direction cards.
- `FoundryControlView` represents customizer controls and their state.
- `FoundryOptionCard` represents whole-model control samples, choices, and
  provider options.
- `FoundryPackView` represents the family-pack workspace.

Technical paths are optional fields for internal diagnostics and developer-only
inspection, not the primary novice-facing surface.

## Non-Goals

Current non-goals:

- native GPU viewport integration
- natural-language parsing
- materials, UVs, rigging, or animation
