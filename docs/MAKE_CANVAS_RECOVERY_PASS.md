# Make Canvas Recovery Pass

## Purpose

This branch recovers the Make screen from a form-like prototype into a stateful
asset workspace.

The pass is scoped to the native Shape Lab app surface. It does not add new
modeling, catalog, surface, rigging, animation, texturing, server, browser, or
LLM behavior.

## Product Rule

The Make screen must always make these facts visible:

- what asset is being edited;
- whether the asset is ready;
- what action is running;
- what changed after an action;
- what the user can do next.

Bottom status text is allowed as secondary feedback only. The model stage,
inspector, candidate tray, and drawers must carry the primary state.

## Implemented Changes

- Added a derived `MakeCanvasViewState` and `MakeCanvasMode`.
- Removed the full-page Make scroll as the main layout.
- Kept the model stage visible while the right-side inspector scrolls.
- Kept the idea tray visible at the bottom.
- Hid normal novice Build Asset and Refresh Preview actions.
- Kept template start auto-build and auto-preview behavior.
- Added local busy overlays on the model stage.
- Added skeleton idea cards while generation is active.
- Disabled focus and option actions while idea generation is active.
- Added local stale-result warning copy near the Make workflow.
- Added visible focus callouts for focused semantic part groups.
- Added selected Current vs Candidate comparison with Use This Idea and Reject.
- Made core Make actions use visible button tones instead of `ButtonTone::Quiet`.
- Expanded the screenshot sanity script to the eleven-shot recovery gate.

## Out Of Scope

- True mesh hit-testing for part picking.
- New candidate generation algorithms.
- New catalog art or mesh provider work.
- Surface candidate rendering.
- UV unwrapping or texturing.
- Rigging, skinning, animation, or game-ready export claims.

## Manual Acceptance

The branch is not merge-ready until the screenshot gate captures and passes:

```text
01_choose.png
02_make_ready.png
03_generating_ideas.png
04_generated_ideas.png
05_selected_comparison.png
06_focus_handles.png
07_generating_handle_ideas.png
08_handle_ideas.png
09_focus_vents.png
10_pack_drawer.png
11_export_drawer.png
```

The screenshots must show visible state changes, not only changed status text.
