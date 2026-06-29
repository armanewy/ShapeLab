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
- `preparation_phase`
- `preparation_timed_out`
- `preparation_fallback_visible`
- `preview_updating`
- `preview_update_required`
- `local_banner_title`
- `local_banner_message`
- `local_banner_tone`
- `primary_title`
- `primary_action_label`
- `primary_action_enabled`
- `primary_action_disabled_reason`
- `local_busy_label`
- `local_busy_visible`
- `focused_part_label`
- `focused_part_visible`
- `focused_part_actions_visible`
- `model_ready`
- `preview_ready`
- `candidate_tray_visible`
- `candidate_tray_state`
- `candidate_count`
- `candidate_search_finished_empty`
- `rejected_candidate_summary`
- `selected_candidate_present`
- `selected_comparison_visible`
- `pack_drawer_visible`
- `export_drawer_visible`
- `local_warning_message`
- `local_error_message`
- `next_action_hint`

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
The visible preparation phase is `Preparing model`, then `Rendering preview`,
then `Ready`.

Before the first full build finishes, Make may show a deterministic quick
template preview so the stage is not blank.

Make does not expose novice-facing `Build Asset` or `Refresh Preview` actions.
When the current preview is stale, missing, or rendering, the visible copy is
`Preview is updating...`; the manual recovery action is `Update preview`.

If preparation times out locally, `preparation_fallback_visible` is true and the
copy is:

```text
Still preparing. You can keep waiting or retry.
```

The fallback actions are `Retry preparation`, `Choose another template`, and
`Open Project`.

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

If idea generation exceeds the local timeout, `idea_generation_fallback_visible`
is true. The banner copy is:

```text
Still trying ideas.
```

The visible recovery actions are `Cancel` and `Keep waiting`. Canceling records
the candidate job as canceled in the trace and shows:

```text
Canceled earlier idea search.
```

The candidate tray renders from `candidate_tray_state`:

- `EmptyReady` shows the ready-to-try-ideas empty state;
- `GeneratingSkeletons` shows skeleton cards;
- `HasCandidates` shows comparison and candidate cards;
- `NoCandidatesWithRecovery` shows no-survivor copy and recovery actions;
- `ErrorWithRecovery` shows local error copy and recovery actions.

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
- `rejected_candidate_summary` shows rejected-similar counts locally when
  available;
- `next_action_hint` tells the user whether to select, compare, use, reject, or
  continue waiting.

When a focused search returns zero clear candidates, the local banner title is
`No clear focused ideas survived`. The recovery card explains whether candidates
were hidden, too subtle, duplicate-looking, or outside the focused part. Vents
use limited-variation copy when applicable. Recovery actions include `Try again`,
`Choose another part`, and `Unlock controls`.

When stale background work is ignored, the local banner title is `Older result
ignored`, the copy is product-safe, and the local region offers `Try again`.

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

For generating scenarios, screenshot mode holds completed background job events
after the assertion passes so the captured frame still shows the local busy
overlay and skeleton tray. Normal app sessions continue to consume job events
immediately.
