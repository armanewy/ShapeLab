# Make Canvas Screenshot Gate Results

## Branch

`codex/make-canvas-interaction-recovery`

## Status

`BLOCKED`

The release build completed, and `target/release/shape-app` launched as a
process, but the visual gate could not capture usable Make canvas screenshots.
The app appeared only as the active macOS menu-bar app (`Shape Lab`) with no
visible window. Computer Use could not attach to it by app name or executable
path, and full-screen `screencapture` captured only the desktop/menu bar.

This branch does not claim visual success.

## Screenshot Path

Attempted local capture directory:

```text
target/make-canvas-screenshots-recovery
```

## Screenshot Files

| Screenshot | Scenario file value | Expected state | Result |
| --- | --- | --- | --- |
| `01_choose.png` | none | Choose screen | Captured, but invalid: desktop/menu bar only |
| `02_make_ready.png` | `make_initial_crate` | `Ready` | Missing |
| `03_generating_ideas.png` | `generating_whole_asset_ideas` | `local_busy_visible = true` | Missing |
| `04_generated_ideas.png` | `generated_whole_asset_ideas` | `candidate_tray_visible = true` | Missing |
| `05_selected_comparison.png` | `selected_comparison` | `selected_comparison_visible = true` | Missing |
| `06_focus_handles.png` | `focus_handles` | `focused_part_label = Handles` | Missing |
| `07_generating_handle_ideas.png` | `generating_handle_ideas` | Handles generation busy | Missing |
| `08_handle_ideas.png` | `handle_ideas` | Handles focused and tray visible | Missing |
| `09_focus_vents.png` | `focus_vents` | `focused_part_label = Vents` | Missing |
| `10_pack_drawer.png` | `pack_drawer` | `pack_drawer_visible = true` | Missing |
| `11_export_drawer.png` | `export_drawer` | `export_drawer_visible = true` | Missing |

## State Assertion Output

Expected assertion log:

```text
<system-temp>/shape-lab-screenshot-state-assertions.txt
```

Result:

```text
Not produced. The release app did not expose a visible window for scenario
capture, so no scenario advanced to a completed state assertion.
```

Code-level unit coverage was added for the same assertions:

- `02_make_ready.png`: mode `Ready`, model ready, preview ready;
- `03_generating_ideas.png`: local busy whole-asset generation;
- `04_generated_ideas.png`: candidate tray visible;
- `05_selected_comparison.png`: selected comparison visible;
- `06_focus_handles.png`: focused part `Handles`;
- `07_generating_handle_ideas.png`: local busy Handles generation;
- `08_handle_ideas.png`: candidate tray visible with Handles focus;
- `09_focus_vents.png`: focused part `Vents`;
- `10_pack_drawer.png`: pack drawer visible;
- `11_export_drawer.png`: export drawer visible.

## Image Sanity Check

Script added:

```text
crates/shape-app/tests/check_make_canvas_screenshots.sh
```

Command run:

```bash
bash crates/shape-app/tests/check_make_canvas_screenshots.sh target/make-canvas-screenshots-recovery
```

Output:

```text
01_choose.png 2940x1912 870c31cb5330fa94b20cf9ddf1d5fdef730e906c2179045691e8caef0ee30a53
Missing screenshot: 02_make_ready.png
```

Result: `FAIL`, because the visual capture is blocked and the required
screenshots are missing.

## Code Gate Output

| Command | Result |
| --- | --- |
| `cargo fmt --all --check` | Pass |
| `cargo test -p shape-app foundry --jobs 1` | Pass |
| `cargo test -p shape-app --test foundry_direction_board --jobs 1` | Pass |
| `cargo clippy --workspace --all-targets -- -D warnings` | Pass |
| `cargo build --release --workspace` | Pass |

## Manual Review

Manual screenshot review is blocked. The only captured image is not a valid
Shape Lab UI capture.

## Remaining Blockers

- A visible release app window is required before the screenshot scenario can be
  captured.
- Computer Use currently reports `Invalid app` for both `shape-app` and the
  full release executable path.
- Screenshot state assertions must produce a pass log before capture.
- All eleven screenshots must exist and pass image sanity checks.
- The screenshots must be manually inspected before this gate can pass.
