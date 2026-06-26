# Candidate Legibility Engine v1

Candidate Legibility Engine v1 rejects candidates before diversity selection
when the user cannot perceive the difference at direction-card size.

## Evidence

The v1 report is `PerceptualCandidateReport`. It records:

- `candidate_id`
- `render_delta_by_camera`
- `max_delta`
- `average_delta`
- `silhouette_delta`
- `bbox_delta`
- `changed_part_groups`
- `changed_controls`
- `legibility_class`
- optional `reject_reason`
- `human_summary`

The search crate currently derives candidate evidence from fixed-camera
projected mesh descriptors because dependency reconciliation is owned by the
integration wave. The render crate exposes a matching rendered-preview
classifier for real pixel evidence.

## Selection Order

Candidate selection is ordered as:

1. hard validation
2. legibility rejection
3. duplicate-looking collapse
4. diversity selection among visually legible survivors

The generator must not select mathematical diversity before legibility. A
candidate with weak or unavailable visual evidence is rejected before it can
compete for the six-card board.

## Threshold Rules

Whole-asset Shape and Complete Look candidates must be at least Clear. They
cannot pass on a hidden scalar, internal recipe fingerprint, unsupported
surface change, or detail-only change.

Focused part candidates must change the selected product-facing part group and
must not visibly change unrelated groups.

Surface and Wear remain unavailable until Shape Lab has textured preview and
material candidate evidence. Shape cannot pass with surface-only evidence.

Detail may be subtler, but it must be requested as Detail and produce detail
evidence or a detail label.

## Honest Counts

Shape Lab returns fewer than six candidates when fewer than six candidates are
visually legible. Diagnostics use copy such as:

```text
Generated 3 visually distinct ideas. Rejected 5 that looked too similar.
```

Padding the board with weak candidates is a product failure.

## Endpoint Gate

Control endpoint reports compile endpoint samples and classify the strongest
visible delta for each visible control. Major Sci-Fi Crate controls must be at
least Clear:

- Body Proportions
- Structural Heft
- Panel Depth
- Vent Density
- Handle Style
- Detail Density

Edge Softness may be SubtleButExplainable. Endpoint reports preserve measured
preview deltas and also record authored part-group endpoint evidence for
controls whose current descriptor evidence is not sensitive to an internal
depth or detail change.
