# Candidate Failure Taxonomy

Candidate generation reports structured, product-safe failure reasons through
`FoundryCandidateReliabilityReport`. The report is deterministic and intended
for UI copy, diagnostics, and tests.

## Reasons

- `TooSimilar`: proposals duplicated existing ideas or collapsed during visual
  duplicate selection.
- `HiddenChange`: proposals changed no visible authored control or produced no
  useful edit program.
- `WrongScope`: a focused request changed another part or failed to visibly
  affect the selected part.
- `LockedOut`: locks or search protection removed the useful controls.
- `NoBoundControls`: a focused part has no authored controls/provider roles for
  shape ideas.
- `ControlTooSubtle`: available controls are detail/subtle controls and are not
  suitable for focused Shape ideas.
- `ProviderUnavailable`: requested provider/channel support is unavailable.
- `RenderDeltaUnavailable`: descriptor or preview-delta evidence could not be
  produced.
- `ValidationFailed`: compile, validation, or conformance rejected the proposal.

## Minimum Useful Result

- Whole-asset requests with fewer than two survivors are reported as
  `NoUsefulCandidates`.
- Focused-part requests with zero survivors are reported as
  `NoFocusedCandidates`.
- Passing requests are reported as `Useful`.

Every weak or empty result includes `top_reasons`, a deterministic
`suggested_action`, and focused part capability rows when the profile exposes
part groups.
