# Wave 3 App State Notes

## Implemented State Transitions

- `AppState` now owns project history, selection, search scope, enabled parameter groups, exploration mode, seed/generation counters, preview caches, candidate slots, camera, quality settings, file path, dirty state, status, recoverable errors, active jobs, and active generation IDs.
- `AppCommand` handling is UI-independent and returns `AppEffect` values for geometry, rendering, save, load, export, and exit work.
- Direct scalar parameter edits go through `shape_core::apply_edit`, validate atomically, update the current revision snapshot in place, mark the project dirty, clear stale candidates, and schedule a current-preview rebuild.
- Candidate acceptance uses `shape_project::Project::accept_candidate`, creates a new revision, clears ephemeral candidates, marks the project dirty, and schedules a current-preview rebuild.
- Undo and branch switching move the project current revision, clear stale preview/candidate data, and schedule a rebuild.
- Preset load/reset replaces the project with a validated built-in preset, clears stale jobs and preview data, and schedules a rebuild.
- Job events are accepted only when their job and generation IDs still match current state; stale preview and candidate events are ignored.
- Save, load, export, and exit are emitted as effects only. This state layer does no filesystem work and spawns no threads.

## Assumptions

- Direct scalar edits are document edits on the current revision, not new history revisions. The existing project API exposes candidate acceptance as the revision-creation path.
- Cancelling a generation marks its job/generation IDs stale. A later worker event for that job is ignored by state.
- The state layer stores search proposal/result counts so later UI panels can adjust them without owning state internals.
- `Save` without an existing path returns a state error; the menu layer should use `SaveAs` when no path is known.
- Export requires a current preview mesh to exist before emitting the export effect.

## Contract Issues

- No blocking contract issue found.
- `AppEffect::SaveProject`, `LoadProject`, and `ExportCurrentObj` carry only paths. The integrator will need to pair these effects with the current `AppState` project/preview data when performing I/O.
