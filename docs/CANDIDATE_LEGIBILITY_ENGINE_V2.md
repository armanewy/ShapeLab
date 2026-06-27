# Candidate Legibility Engine v2

Candidate Legibility Engine v2 prevents generated Foundry candidates from
reaching the UI unless the change is readable at decision-card size.

## Pipeline Order

The search path uses this fixed order:

1. Compile and conformance validation.
2. Lock and search-protection validation.
3. Visible-delta classification.
4. Duplicate-looking collapse.
5. Diversity selection among visible survivors.

Mathematical diversity never rescues a visually unreadable candidate.

## Evidence

Each accepted compiled proposal receives `CandidateVariationMetadata` with a
strict `PerceptualCandidateReport`:

- `candidate_id`
- `render_delta_by_camera`
- `max_delta`
- `average_delta`
- `silhouette_delta`
- `bbox_delta`
- `changed_part_groups`
- `changed_controls`
- `legibility_class`
- `reject_reason`
- `human_summary`

Search-side evidence is deterministic fixed-camera mesh evidence: primary
three-quarter, front, side, and top/detail-style projections. The render crate
also exposes public rendered-preview classification for real pixel comparisons
at deterministic preview sizes such as 256 px, and now rejects single-view
evidence as `Unsupported`.

## Strict Thresholds

Whole-asset Complete Look and Shape candidates must be `Clear` or `Strong`.
They cannot pass from unsupported surface/material deltas, hidden scalar edits,
or detail-only changes unless Detail was explicitly requested.

Focused part candidates must visibly affect the selected product part group.
Changes outside the selected part group are counted as wrong-scope rejections.

Surface and Wear remain unavailable until Shape Lab has visual surface evidence.
Shape candidates cannot pass from surface-only evidence, and Surface candidates
cannot pass from shape-only evidence.

Detail candidates may be subtle only when the request is explicitly Detail, and
the report labels the result as detail evidence.

## Honest Counts

The generator returns fewer than six candidates when fewer candidates survive.
Diagnostics use separate counts:

- generated `N` clear ideas;
- rejected `M` that looked too similar;
- rejected `K` because changes were hidden/internal;
- rejected `L` because changes were wrong scope.

The board must never be padded with weak candidates.

## Endpoint Reports

`generate_foundry_control_endpoint_visibility_report` samples visible control
endpoints, compiles valid endpoint documents, classifies the strongest endpoint,
and emits an explicit row for unsupported endpoints instead of dropping the
control from the report.

Major starter-template controls for Sci-Fi Crate, Roman Bridge, and Stylized
Lamp are tested to be at least `Clear`. Edge and softness controls may be
`SubtleButExplainable` only when they are explicitly labeled that way.
