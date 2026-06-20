# Wave 3 Gallery, History, Menus, And Status

## Scope

Implemented the Wave 3 Prompt 3.5 panel layer in the assigned files only:

- candidate gallery card layout, progress display, cancellation, accept, and dismiss commands
- revision history path, current revision, child directions, branch-point labels, undo, and switch commands
- application menu command helpers and native file-dialog path selection
- bottom status line for project/preset, dirty state, phase, progress, last message, latest recoverable error, and mesh triangle count
- non-window tests for formatting and command helper behavior

## UI Decisions

- Candidate cards always require the explicit `Choose This Direction` button. Hover only draws a visible outline.
- The unchanged parent is shown as a control card so users can compare generated directions against the current shape.
- Candidate difference text uses the candidate edit program and current document parameter descriptors, capped at the top three changed values.
- History uses the current revision as the selected revision because `AppState` does not yet expose a separate history selection.
- Save, open, save-as, and OBJ export use `rfd` only in the menu panel and emit `AppCommand` values after a path is chosen.
- The gallery reserves thumbnail frames and reports preview readiness from `CandidatePreview::image`; it does not create `egui::TextureHandle` values.

## Contract Issues

- `AppCommand` has no `About` variant. The Help menu shows About content inline instead of emitting a command.
- `AppState` has no candidate texture handles. Per `docs/app-contracts.md`, texture upload belongs to the app coordinator, so the gallery avoids creating textures and leaves final image presentation wiring to the Wave 4 integration.
- `AppState` has no separate selected history revision. The history panel therefore displays children of the current revision.

## Commands

To be run before commit:

```bash
cargo fmt --all --check
cargo test -p shape-app
cargo clippy -p shape-app --all-targets -- -D warnings
```
