# Make Canvas Interaction Recovery v2

## Status

`AUTOMATED STATE RECOVERY IMPLEMENTED; HUMAN VISUAL GATE REQUIRED`

This pass makes Make state more explicit and keeps product-visible state local
to the Make workspace. It does not claim final dogfood success until a human
video/screenshot pass confirms the flow.

## State Contract

`MakeCanvasViewState` is the product-level Make contract. It now includes:

- asset identity and readiness;
- active mode;
- primary title/action/enabled state/disabled reason;
- local busy state;
- focused-part label and action visibility;
- candidate tray count, selected comparison, and rejected-candidate summary;
- Pack/Export drawer visibility;
- local warning/error messages;
- `next_action_hint`.

Render paths for the model stage, inspector, candidate tray, Pack, and Export
derive from this state instead of asking the user to infer state from the bottom
status strip.

## Interaction Recovery

Implemented or preserved behavior:

- starting a template switches to Make and queues model/preview preparation;
- while preparing, Try ideas and build-dependent actions are disabled with a
  local plain-language reason;
- generating ideas disables conflicting actions, changes the primary label, and
  shows a local model-stage overlay plus candidate skeletons;
- ignored stale results appear as a local Make warning;
- focused parts change title, primary action, callout, tray actions, and
  filtered controls;
- generated candidates show a selected comparison when a selected candidate and
  current preview are renderable;
- rejected-similar summaries are shown locally near the candidate tray;
- Pack and Export drawers are represented in visible Make state;
- core workflow actions use filled button tones, not quiet/text-only styling.

## Screenshot Gate

Required scenario files:

- `01_choose.png`
- `02_make_ready.png`
- `03_generating_ideas.png`
- `04_generated_ideas.png`
- `05_selected_comparison.png`
- `06_focus_handles.png`
- `07_generating_handle_ideas.png`
- `08_handle_ideas.png`
- `09_focus_vents.png`
- `10_pack_drawer.png`
- `11_export_drawer.png`

The screenshot driver records state assertions before claiming scenario
completion. The image sanity script verifies required files, minimum size, and
adjacent screenshot hash changes, including `08_handle_ideas.png` versus
`07_generating_handle_ideas.png`.

## Remaining Human Gate

The branch is not product-stable until a reviewer can confirm:

- the next action is obvious in Make;
- local running state is visible without reading the bottom strip;
- generated ideas and selected comparisons are readable;
- focused Handles/Vents visibly change the workspace;
- Pack and Export drawers are clear;
- unsupported surface/rig/motion/game-ready claims do not appear.
