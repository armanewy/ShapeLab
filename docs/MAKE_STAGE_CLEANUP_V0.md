# Make Stage Cleanup v0

Date: 2026-07-01

## Goal

Make the direct primitive Make stage feel like Object Orchard instead of a DCC
modeling viewport.

## Result

- Direct primitive previews now render over a warm studio stage by default.
- The default stage no longer draws the permanent coordinate grid.
- The default stage no longer draws red/green axes through the model.
- The center origin cursor is removed from the default Make preview.
- A small corner orientation cue remains so users can read view direction.
- Orbit and Reset view controls remain visible in the direct primitive stage.
- Axis view is present as an optional control label, but it is not active by
  default.

## Interaction Boundary

The model remains fixed in authored coordinates while secondary-button drag
orbits the camera around the model center. Reset view restores the authored
camera. No mesh transform gizmo, vertex editing, face editing, raw transform
editing, generated idea tray, runtime LLM workflow, material editor, UV editing,
rigging, or animation was added.

## Screenshot Evidence

Screenshots for this branch are generated under:

```text
target/make-stage-cleanup-v0/screenshots/
```

Expected screenshots:

- `box_stage_no_grid.png`
- `flat_panel_stage_no_grid.png`
- `sphere_stage_no_grid.png`
- `panel_knob_stage_no_grid.png`
- `orbit_after_drag_or_tool.png`
- `reset_view.png`

The pass condition is visual: the model should read against a warm studio stage,
no permanent grid or axis lines should dominate the preview, and no Blender-like
transform gizmo should appear.
