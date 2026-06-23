# Foundry App Contracts

Wave 8 freezes the native application boundary for the Foundry workflow without
implementing the reducer, workers, or panels.

## Modules

The native Foundry surface is rooted at `crates/shape-app/src/foundry`:

- `state.rs` owns `FoundryAppState`, the UI-independent session snapshot.
- `commands.rs` owns `FoundryAppCommand`, the app command boundary.
- `jobs.rs` owns `FoundryJobRequest` and `FoundryJobEvent`.
- `view_model.rs` owns `FoundryCandidateCard`, `FoundryControlView`,
  `FoundryOptionCard`, and `FoundryPackView`.
- `panels/*` are empty panel boundaries reserved for Wave 9.

## Command Boundary

`FoundryAppCommand` does not define semantic edits such as set-control,
provider selection, candidate acceptance, export, undo, or pack insertion.
Those remain `shape_foundry::FoundryCommand` values, either wrapped one at a
time or carried as an ordered command program.

UI-only commands are limited to app concerns:

- selection
- requesting build or preview jobs
- requesting project file save/load
- opening or closing Advanced Recipe

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
- export

`FoundryJobEvent` returns compiled outputs, previews, candidates, pack views,
export completion, or failure diagnostics with the same app-local job ID so the
future state reducer can reject stale results deterministically.

## View Models

The view models are whole-model first:

- `FoundryCandidateCard` represents unchanged parent and direction cards.
- `FoundryControlView` represents customizer controls and their state.
- `FoundryOptionCard` represents whole-model control samples, choices, and
  provider options.
- `FoundryPackView` represents the family-pack workspace.

Technical paths are optional fields intended for tooltips or Advanced Recipe,
not the primary novice-facing surface.

## Non-Goals

This wave intentionally does not implement:

- state reduction
- background worker execution
- candidate search wiring
- egui panel rendering
- project persistence execution
- native GPU viewport integration

Those are Wave 9 responsibilities built on the contracts frozen here.
