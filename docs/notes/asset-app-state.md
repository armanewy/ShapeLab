# Asset App State

Prompt 5.3 adds a UI-independent explicit asset state layer under
`crates/shape-app/src/asset`.

- `commands.rs` defines asset authoring intents and app effects without egui
  types.
- `state.rs` owns the current `AssetRecipe`, selection, lock snapshots,
  current artifact and timeline, camera, candidate slots, active generation,
  branchable revision history, path/dirty state, validation issues, template
  metadata, and active/stale job IDs.
- `jobs.rs` defines deterministic asset job requests and reducible events for
  compiling, rendering previews, generating candidates, compiling candidate
  previews, and package export.

The reducer rejects stale results by matching both `AssetJobId` and, for
generation work, `AssetGenerationId` against the active job slot. Recipe changes
mark outstanding jobs stale before scheduling fresh compile work.
