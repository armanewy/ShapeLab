# Asset UI

Prompt 5.4 adds a contract-level, novice-facing asset UI surface inside
`shape-app`.

The new `asset` module does not mutate geometry directly and does not replace
the existing implicit editor. Panels consume an `AssetUiState` snapshot and emit
`AssetAppCommand` values compatible with `docs/asset-app-contracts.md`, with
local extensions for locks, revision switching, undo, and wireframe display.

Implemented panels:

- part tree with hierarchy, selected part, shared definition, optional part,
  mirror/array generation, socket count, and validation badges
- inspector with the beginner groups Size, Proportions, Placement, Curvature,
  Edge Softness, Repetition, Part Presence, and Detail Density
- lock controls for parameter, part, subtree, and topology locks
- candidate gallery with unchanged parent card, six candidate slots, structural
  and numeric summaries, validation state, choose, dismiss, and generation
  progress
- history panel with a branchable revision tree, concise operation summaries,
  switch revision, and undo
- asset viewport wrapper that reuses the current CPU preview widget and adds
  selected part, bounds, socket markers, validation, and wireframe overlay
  controls

The implementation intentionally omits direct vertex editing, gizmos,
materials, and speech. Wave 6 can connect these DTOs to the durable asset app
state reducer without duplicating the existing implicit editor state logic.
