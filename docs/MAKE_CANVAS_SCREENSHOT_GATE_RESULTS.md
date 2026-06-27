# Make Canvas Screenshot Gate Results

## Branch

`codex/make-canvas-recovery-pass`

## Screenshot Path

Blocked.

Expected local capture directory:

```text
target/make-canvas-screenshots-recovery
```

No screenshots were written during this run because the Windows Computer Use
helper connection closed before the first app-control call. The branch remains
not merge-ready until the screenshots are captured and inspected.

## Scenario Mapping

| Screenshot | Scenario file value | Expected state |
| --- | --- | --- |
| `01_choose.png` | none | Choose screen |
| `02_make_ready.png` | `make_initial_crate` | `Ready` |
| `03_generating_ideas.png` | `generating_whole_asset_ideas` | `GeneratingWholeAssetIdeas` |
| `04_generated_ideas.png` | `generated_whole_asset_ideas` | `ReviewingIdeas` |
| `05_selected_comparison.png` | `selected_comparison` | `ReviewingIdeas` |
| `06_focus_handles.png` | `focus_handles` | `FocusedPart` |
| `07_generating_handle_ideas.png` | `generating_handle_ideas` | `GeneratingFocusedPartIdeas` |
| `08_handle_ideas.png` | `handle_ideas` | `ReviewingIdeas` with Handles focused |
| `09_focus_vents.png` | `focus_vents` | `FocusedPart` with Vents focused |
| `10_pack_drawer.png` | `pack_drawer` | `PackDrawerOpen` |
| `11_export_drawer.png` | `export_drawer` | `ExportDrawerOpen` |

## State Assertions

Manual screenshot assertions are blocked until Computer Use can capture the
release app window.

Code-level state assertions were added for:

- preparing asset disables idea generation with a local reason;
- active candidate job switches the primary action to `Trying ideas...`;
- active candidate job sets a local busy state;
- stale background result status becomes a local Make warning;
- focused part changes the primary title and action;
- generated candidates create a reviewing state;
- pack and export drawers set visible drawer state;
- core Make actions do not use `ButtonTone::Quiet`.

## Code Gate Output

Run on the recovery branch:

| Command | Result |
| --- | --- |
| `cargo fmt --all --check` | Pass |
| `cargo test -p shape-app foundry --jobs 1` | Pass |
| `cargo test -p shape-app --test foundry_direction_board --jobs 1` | Pass |
| `cargo test -p shape-app --jobs 1` | Pass |
| `cargo clippy --workspace --all-targets -- -D warnings` | Pass |
| `cargo build --release --workspace` | Pass |

## Image Sanity Check

Failed because screenshot capture is blocked.

Run after capture:

```powershell
powershell -ExecutionPolicy Bypass -File crates/shape-app/check_make_canvas_screenshots.ps1 `
  -ScreenshotDir target/make-canvas-screenshots-recovery
```

The script checks:

- all eleven screenshots exist;
- all screenshots are full-window sized;
- generating differs from ready;
- generated ideas differ from generating;
- selected comparison differs from generated ideas;
- focus and handle idea states differ;
- pack and export drawers visibly differ.

Current run:

```text
Missing screenshot: 01_choose.png
```

## Manual Review

Blocked.

The manual gate fails if:

- the model is not the visual center;
- generating feedback is only in the bottom status strip;
- focus part state is only a selected chip with no model callout;
- candidates appear only as thumbnails with no comparison;
- Pack or Export drawer state is not visible;
- Surface copy overclaims textured or game-ready output;
- raw backend or technical terms appear in the default workflow.

## Remaining Blockers

The recovery branch is not merge-ready until:

- the eleven screenshots are captured from `target/release/shape-app.exe`;
- state assertions pass before each capture;
- `check_make_canvas_screenshots.ps1` passes;
- the screenshots are manually inspected.
