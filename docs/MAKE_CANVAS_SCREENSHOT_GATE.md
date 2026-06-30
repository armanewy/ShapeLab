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

- `box_direct_make_ready.png`
- `box_property_edit.png`
- `flat_panel_direct_make_ready.png`
- `flat_panel_property_edit.png`
- `pack_drawer.png`
- `export_drawer.png`

Acceptance questions:

- Is the model the center of the screen?
- Is Make a clear asset workspace rather than a settings form beside a model?
- Are build/preview internals hidden from the novice flow?
- Are action priorities clear and visibly clickable?
- Can a reviewer tell where to edit Width, Height, Depth, Thickness, and Edge
  Softness?
- Is generated-idea UI absent from active primitive Make?
- Is selected-candidate comparison absent from active primitive Make?
- Are Pack and Export actions scoped drawers rather than workflow tabs?
- Is export truthful about the current primitive?
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
