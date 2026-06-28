# Candidate Fallback Policy

Candidate fallback suggestions are human-visible actions, not silent generator
fallbacks. The app should show them when `FoundryCandidateReliabilityReport`
marks a result as `NoUsefulCandidates` or `NoFocusedCandidates`.

## Actions

- `TryWholeAssetIdeas`: default whole-asset recovery when too few useful ideas
  survived.
- `ClearFocus`: focused request failed because the focused scope is too narrow
  or wrong for the generated changes.
- `UnlockControls`: focused or whole-asset generation is blocked by locks or
  search-protected controls.
- `TryAnotherPart`: the selected focused part depends on unavailable provider or
  validation support.
- `UseDetailMode`: the focused part is backed only by subtle/detail controls.
- `NoFocusedVariants`: the part exists in the product model but has no focused
  variants yet.

## Focused Part Capability Rows

Each known part group reports:

- `can_generate_shape_ideas`
- `likely_candidate_count`
- `blocked_reasons`
- `suggested_action`

These rows let the UI disable or explain Focus Part requests before a user sees
an empty candidate tray. The policy is conservative: if Shape ideas cannot be
produced for a focused part, the report must name the blocker and suggest one
visible next action.
