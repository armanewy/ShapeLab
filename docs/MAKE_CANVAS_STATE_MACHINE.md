# Make Canvas State Machine

## Purpose

The Make canvas derives product-visible UI state from one
`MakeCanvasViewState` snapshot each frame. The bottom status strip is secondary;
the model stage, inspector, candidate tray, and drawers carry the primary state.

## Modes

`MakeCanvasMode` is derived from the current app state:

- `NoAsset`
- `PreparingAsset`
- `Ready`
- `GeneratingWholeAssetIdeas`
- `GeneratingFocusedPartIdeas`
- `ReviewingIdeas`
- `FocusedPart`
- `PackDrawerOpen`
- `ExportDrawerOpen`
- `Error`

## View State

`MakeCanvasViewState` carries the product-level state required by the recovery
prompt:

- `mode`
- `asset_name`
- `primary_title`
- `primary_action_label`
- `primary_action_enabled`
- `primary_action_disabled_reason`
- `local_busy_label`
- `local_busy_visible`
- `focused_part_label`
- `focused_part_visible`
- `model_ready`
- `preview_ready`
- `candidate_tray_visible`
- `candidate_count`
- `selected_candidate_present`
- `selected_comparison_visible`
- `pack_drawer_visible`
- `export_drawer_visible`
- `local_warning_message`
- `local_error_message`

## Mode Rules

`NoAsset` is shown when no document is loaded.

`PreparingAsset` is shown when a document exists but the current model or
preview is not ready, or when compile/edit/preview work is active. Old compiled
output does not make Pack or Export available while current build work is still
active.

`Ready` is shown when model and preview are ready, no local work is active, no
part is focused, and no candidate cards are present.

`GeneratingWholeAssetIdeas` is shown while whole-asset candidate generation or
candidate preview filtering is active.

`GeneratingFocusedPartIdeas` is shown while candidate generation or candidate
preview filtering is active for a focused part.

`ReviewingIdeas` is shown once candidate cards exist.

`FocusedPart` is shown when a part group is focused and the app is not currently
generating or reviewing candidates.

`PackDrawerOpen` and `ExportDrawerOpen` are shown when their drawers are open.

`Error` is shown when a local Make-level error should be displayed near the
workflow.

## Interaction Rules

Starting a template switches to Make and queues model preparation automatically.
After compile completion, the host requests the current preview automatically.

Idea generation is disabled until both `model_ready` and `preview_ready` are
true.

While ideas are generating:

- the primary action reads `Trying ideas...`;
- `local_busy_visible` is true;
- the model stage shows a local busy overlay;
- the candidate tray shows skeleton cards;
- focus switching is disabled;
- option actions are disabled;
- candidate acceptance is disabled;
- Pack and Export are disabled when the current build is stale.

When an old result is ignored, `local_warning_message` is set to:

```text
An older result was ignored because you changed the asset.
```

When a part is focused:

- `focused_part_visible` is true;
- the focused label becomes the primary title;
- the primary action becomes part-specific, such as `Try handle ideas`;
- the model stage draws a part callout;
- the local part action tray appears;
- the control list is filtered to relevant controls.

When candidates exist:

- `candidate_tray_visible` is true;
- the first candidate is selected by reducer state when generation completes;
- `selected_comparison_visible` is true when a selected candidate and current
  preview are both renderable;
- the candidate count is shown near the tray;
- rejected-similar counts are shown locally when available.

Pack and Export drawers are visible state, not status-only state:

- `pack_drawer_visible` is true when the Pack drawer is open;
- `export_drawer_visible` is true when the Export drawer is open.

## Screenshot Assertions

The screenshot scenario driver marks a scenario complete only after these state
assertions pass:

| Screenshot | Required state |
| --- | --- |
| `02_make_ready.png` | mode `Ready`, model ready, preview ready |
| `03_generating_ideas.png` | `local_busy_visible = true` and whole-asset generation |
| `04_generated_ideas.png` | `candidate_tray_visible = true` |
| `05_selected_comparison.png` | `selected_comparison_visible = true` |
| `06_focus_handles.png` | `focused_part_label = Handles` |
| `07_generating_handle_ideas.png` | `local_busy_visible = true` and focus Handles |
| `08_handle_ideas.png` | `candidate_tray_visible = true` and focus Handles |
| `09_focus_vents.png` | `focused_part_label = Vents` |
| `10_pack_drawer.png` | `pack_drawer_visible = true` |
| `11_export_drawer.png` | `export_drawer_visible = true` |

Assertion output is appended to:

```text
<system-temp>/shape-lab-screenshot-state-assertions.txt
```
