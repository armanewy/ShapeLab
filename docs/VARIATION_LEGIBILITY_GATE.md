# Variation Legibility Gate

Candidate generation must distinguish control-space edits from user-visible differences. A candidate is not a useful direction just because it changes controls, providers, hidden vertices, recipe values, or internal roles.

## Evidence

The legibility report tracks:

- control-space difference
- provider or role difference
- visible shape difference
- silhouette difference
- screen-space rendered difference
- surface/material difference when real surface evidence exists
- detail-only difference

Scores are finite and clamped to `0..1`. The product class is one of:

- `Strong`
- `Clear`
- `SubtleButExplainable`
- `DetailOnly`
- `TooSubtle`
- `DuplicateLooking`
- `Unsupported`

Only Strong, Clear, SubtleButExplainable, and DetailOnly may be returned as normal selectable directions.

## Channel Rules

Complete Looks must have visible shape delta or supported visible surface delta. They must not rely on unsupported surface evidence.

Shape candidates must pass shape, silhouette, structure, or screen-space evidence. They cannot pass by changing only materials.

Surface candidates must pass surface/material/wear evidence. If no surface payload exists, they are unavailable rather than padded into the board.

Detail candidates may be subtle, but must be labeled DetailOnly or SubtleButExplainable and should not replace normal Explore or Silhouette directions unless Detail was requested.

When fewer than six candidates survive the gate, the result count is smaller. Shape Lab must not pad with weak, hidden-only, duplicate-looking, unsupported, or explanation-mismatched candidates.

## v0 Implementation Claims

Focus Part v0 uses semantic part groups and requires visible selected-part
evidence for focused shape candidates. Whole-asset and focused candidates are
filtered again after CPU preview rendering so hidden/control-only changes do
not count as shown directions.

The product may report smaller boards, for example "Generated 4 visually
distinct directions." and "Rejected 2 subtle candidates that looked too
similar." Surface Focus remains unavailable until textured previews and
material candidate support exist.
