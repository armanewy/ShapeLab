# Wave 5 UX Presets Notes

## Scope

- Added the `Sky Shrine` built-in preset as a fourth non-humanoid model category.
- Renamed preset parts to beginner-facing object names and added outliner tags such as `#focus`, `#top`, `#base`, `#window`, `#pillar`, and `#roof`.
- Updated owned panels to describe options, parts, locks, branches, reset, and progress without requiring 3D modeling vocabulary.
- Added a beginner walkthrough for three generations and one branch.

## UX Decisions

- The unchanged gallery card is labeled as an `Unchanged control` so users know it is the comparison baseline, not a generated result.
- Candidate cards now say `Option`, use more distinct Refine/Explore wording, and show beginner summaries such as width, height, edge softness, and blend amount.
- The inspector explains that `Keep` locks a value so generated options leave it unchanged.
- The inspector shows which part or scope the next options may change.
- `Reset to Preset` is exposed in the inspector when the current project came from a built-in preset, using the existing app command.
- History describes branches as possible paths created by going back and choosing a different option.
- Status and error display translate common state messages into action-oriented wording while preserving details for unexpected errors.

## Preset Audit

- Existing presets were renamed to avoid geometry words in part names while keeping their underlying primitives unchanged.
- Focus tags identify useful starting parts for visible edits: lamp shade/support, submarine body/tower, plant stalk/pods, and shrine roof/ring/bead.
- Small decorative pieces retain conservative dimensions and roundness so their parameter ranges stay valid under existing core limits.
- Per-preset default selected nodes are still controlled by app state, which selects the document root on load. Within owned files, roots were renamed as whole-model controls and focus tags were added for sensible manual selection.

## Verification Plan

Run before commit:

```bash
cargo fmt --all --check
cargo test -p shape-presets
cargo test -p shape-app panel_tests inspector_tests
cargo clippy -p shape-presets --all-targets -- -D warnings
cargo clippy -p shape-app --all-targets -- -D warnings
```
