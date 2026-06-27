# Make Canvas State Machine

## Modes

`MakeCanvasMode` is derived each frame from app state:

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

`MakeCanvasViewState` carries the product-visible state:

- `mode`
- `primary_title`
- `primary_action_label`
- `primary_action_enabled`
- `primary_action_disabled_reason`
- `busy_label`
- `focused_part_label`
- `model_ready`
- `preview_ready`
- `candidate_count`
- `selected_candidate_present`
- `pack_drawer_visible`
- `export_drawer_visible`
- `local_error_message`

## Mode Rules

`NoAsset` is shown when no document is loaded.

`PreparingAsset` is shown when a document exists but the compiled model or
preview is not ready.

`GeneratingWholeAssetIdeas` is shown while candidate generation is active and no
part group is focused.

`GeneratingFocusedPartIdeas` is shown while candidate generation is active and a
part group is focused.

`ReviewingIdeas` is shown once candidate cards exist.

`FocusedPart` is shown when a part group is focused and the app is not currently
generating or reviewing candidates.

`PackDrawerOpen` and `ExportDrawerOpen` are shown when their drawers are open.

`Error` is reserved for local Make errors that must be shown near the workflow.

## Interaction Rules

Starting a template switches to Make and queues model preparation automatically.

Idea generation is disabled until the model and preview are ready.

While ideas are generating:

- the primary action reads `Trying ideas...`;
- the model stage shows a busy overlay;
- the candidate tray shows skeleton cards;
- focus switching is disabled;
- option actions are disabled;
- stale results are shown as local warnings.

When a part is focused:

- the focused label becomes the primary title;
- the primary action becomes part-specific;
- the model stage draws a part callout;
- the local part action tray appears;
- the control list is filtered to relevant controls.

When candidates exist:

- the first selected candidate is shown in Current vs Candidate comparison;
- Use This Idea and Reject are visible;
- the candidate count is shown near the tray;
- rejected-similar counts are shown locally when available.
