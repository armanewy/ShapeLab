# Make Canvas Screenshot Gate Results

## Status

`AUTOMATED SCREENSHOT GATE PASSED; HUMAN DOGFOOD NOT PASSED`

This document supersedes the older branch-local BLOCKED report from
`codex/make-canvas-interaction-recovery`. That first attempt could not capture
the raw `target/release/shape-app` process. A later integration retry used a
macOS `.app` wrapper and captured the required Make Canvas screenshots.

The current repository also includes `scripts/package_macos_app.sh` and
`packaging/macos/Info.plist`, so local macOS smoke tests can create
`target/release/Shape Lab.app` without hand-building a temporary wrapper.

Important: this is not a human dogfood pass. The screenshot gate proves that
the app can reach required states and that screenshot files are not
byte-identical. It does not prove that Make is clear to a novice, that candidate
differences are readable, or that the next action is obvious.

## Automated Evidence

Required screenshots:

- `01_choose.png`
- `02_make_ready.png`
- `03_generating_ideas.png`
- `04_generated_ideas.png`
- `05_selected_comparison.png`
- `06_focus_handles.png`
- `07_generating_handle_ideas.png`
- `08_handle_ideas.png`
- `09_focus_vents.png`
- `10_pack_drawer.png`
- `11_export_drawer.png`

Known local evidence paths from the recovery work:

```text
target/visual-retry/make-canvas-screenshots/
target/visual-demo/screenshots/
```

The `target/visual-demo/` run also produced:

```text
target/visual-demo/shape-lab-demo.mov
```

## State Assertions

The app records scenario assertions in:

```text
<system-temp>/shape-lab-screenshot-state-assertions.txt
```

Code-level assertions cover:

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

Script:

```text
crates/shape-app/tests/check_make_canvas_screenshots.sh
```

The script verifies:

- all required screenshots exist;
- screenshots meet minimum dimensions;
- selected adjacent state screenshots have different hashes.

Weakness: hash differences are not product comprehension. They do not prove
that buttons look clickable, local busy state is visually dominant, candidate
differences read at decision size, focus feels direct, or Pack/Export are clear
as a workflow.

## Human Verdict

`NO-GO`

The latest human video audit says the Make tab still feels like a model beside
a control form, exposes build/preview sequencing, relies too much on status
text, and does not make generated differences or focused-part interaction clear
enough.

## Remaining Blockers

- Replace screenshot existence/hash checks with a stronger visual-state gate.
- Record a clean dogfood video for Sci-Fi Crate, Roman Bridge HQ, and Stylized
  Lamp.
- Do not mark Make Canvas as product-stable until a human can complete the
  flow without implementation docs.
