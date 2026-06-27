# Make Canvas Screenshot Gate Results

## Branch

`codex/fix-make-canvas-state-flow`

## Screenshot Path

No passing screenshot set was captured for this branch.

Stale screenshots were found at:

```text
target/make-canvas-screenshots-valid
```

Those files are full-window sized, but they are not valid acceptance evidence
for this branch because the new sanity check fails: `05_make_focus_handles.png`,
`06_make_handle_ideas.png`, and `07_make_focus_vents.png` are byte-identical.

Computer Use was retried against the current installed plugin version and failed
before Windows app control could start:

```text
failed to write kernel assets: The system cannot find the path specified. (os error 3)
```

## Required Screenshots

| Screenshot | Gate |
| --- | --- |
| `01_choose.png` | Blocked: no new accepted capture |
| `02_make_initial_crate.png` | Blocked: no new accepted capture |
| `03_make_generated_whole_asset_ideas.png` | Blocked: no new accepted capture |
| `04_make_selected_comparison.png` | Blocked: no new accepted capture |
| `05_make_focus_handles.png` | Failed in stale set: identical to `06` and `07` |
| `06_make_handle_ideas.png` | Failed in stale set: identical to `05` and `07` |
| `07_make_focus_vents.png` | Failed in stale set: identical to `05` and `06` |
| `08_make_pack_drawer.png` | Blocked: no new accepted capture |
| `09_make_export_drawer.png` | Blocked: no new accepted capture |

## Automated Sanity Checks

Blocked.

The local sanity script was run against the stale full-window screenshot set:

```powershell
powershell -ExecutionPolicy Bypass -File crates/shape-app/check_make_canvas_screenshots.ps1 `
  -ScreenshotDir target/make-canvas-screenshots-valid
```

Result:

```text
Screenshots should differ but are identical:
06_make_handle_ideas.png and 05_make_focus_handles.png
```

## Manual Review

Blocked.

Computer Use could not launch or capture the release app in this session. The
gate therefore remains failed, even though the code-level visible-state tests
and release build pass.

Automated code gates passed before this report:

- `cargo fmt --all --check`
- `cargo test -p shape-app foundry --jobs 1`
- `cargo test -p shape-app --test foundry_direction_board --jobs 1`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo build --release --workspace`

The screenshot gate must still fail if a later capture shows:

- generated ideas are not visible above the fold;
- selected comparison is not visible;
- Focus Handles or Focus Vents still shows Whole asset as the active scope;
- Pack or Export drawers do not visibly open;
- status text is the only evidence that state changed.

## Remaining UI Blockers

The manual screenshot gate is blocked by the Computer Use helper failure. Prompt
B must not start from this branch until a new release-app screenshot set is
captured and passes `crates/shape-app/check_make_canvas_screenshots.ps1`.
