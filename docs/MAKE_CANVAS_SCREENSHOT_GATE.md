# Make Canvas Screenshot Gate

This gate is required for Unified Make Canvas v0. It is not complete unless the
release app is launched and the screenshots below are captured.

Run:

```bash
cargo run -p shape-app --release
```

Capture:

- `01_choose.png`
- `02_make_initial_crate.png`
- `03_make_generated_whole_asset_ideas.png`
- `04_make_selected_comparison.png`
- `05_make_focus_handles.png`
- `06_make_handle_ideas.png`
- `07_make_focus_vents.png`
- `08_make_pack_drawer.png`
- `09_make_export_drawer.png`

Acceptance questions:

- Is the model the center of the screen?
- Are Directions and Customize effectively one Make screen?
- Are mode labels replaced by plain user actions?
- Can a reviewer tell where to click to change Handles?
- Can Current vs Candidate be compared without squinting?
- Are candidate differences easier to read than small cards alone?
- Are Pack and Export actions scoped drawers rather than workflow tabs?
- Is surface/material availability stated honestly?
- Are technical terms absent from the default UI?

If screenshots cannot be captured, the branch must report:

```text
BLOCKED - screenshot verification not completed.
```

