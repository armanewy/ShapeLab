# Make Canvas Screenshot Gate

This gate is required when the Make canvas changes. Automated screenshots are
evidence only; they do not replace the human dogfood gate. Current result
details live in
[`docs/MAKE_CANVAS_SCREENSHOT_GATE_RESULTS.md`](MAKE_CANVAS_SCREENSHOT_GATE_RESULTS.md).

Run:

```bash
cargo run -p shape-app --release
```

Capture:

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

Acceptance questions:

- Is the model the center of the screen?
- Is Make a clear asset workspace rather than a settings form beside a model?
- Are build/preview internals hidden from the novice flow?
- Are running actions visible locally, not only in the bottom status strip?
- Are action priorities clear and visibly clickable?
- Can a reviewer tell where to click to change Handles?
- Can Current vs Candidate be compared without squinting?
- Are candidate differences easier to read than small cards alone?
- Does focused part state visibly change the model stage, tray, and actions?
- Are Pack and Export actions scoped drawers rather than workflow tabs?
- Is surface/material availability stated honestly?
- Are technical terms absent from the default UI?

If screenshots cannot be captured, the branch must report:

```text
BLOCKED - automated screenshot verification not completed.
```

If automated screenshots pass but the human flow is still confusing, the branch
must report:

```text
AUTOMATED SCREENSHOT GATE PASSED; HUMAN DOGFOOD NOT PASSED.
```
