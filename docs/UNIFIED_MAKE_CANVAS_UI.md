# Unified Make Canvas UI

Shape Lab now uses a two-step primary workflow:

1. Choose
2. Make

Make replaces the old separation between idea generation and control tuning. The
model is the workspace: the current asset stays central while the user tries
whole-asset ideas, focuses semantic parts, adjusts scoped controls, compares
candidate results, locks what should stay stable, and opens Pack or Export from
actions instead of primary tabs.

## Make Canvas

The Make screen contains:

- a large central model workspace
- semantic part chips near the model
- contextual generation copy such as "Try 6 whole-asset ideas" or "Try handle ideas"
- controls filtered to the current scope
- a candidate tray with selected Current vs Candidate comparison
- Add to Pack and Export as scoped app-bar actions

Semantic part chips are product nouns such as Body, Panels, Vents, Handles, Edge
Trim, and Fasteners. They are not mesh picking; they apply authored semantic
focus and make the active scope obvious.

## Pack And Export

Pack and Export are no longer primary workflow tabs. Add to Pack is available in
the app bar and opens the Pack drawer after adding the current asset. Export is a
top-right action that opens the Export drawer.

Export copy remains honest about surface support. Static package notes may be
shown, but material looks are not previewable yet and the UI must not claim
textured preview generation or game-ready output.

