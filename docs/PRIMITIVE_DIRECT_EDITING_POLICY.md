# Primitive Direct Editing Policy

Date: 2026-06-30

Direct primitive editing means users manipulate bounded properties, not raw
geometry. The active product surface should show clear controls such as Width,
Height, Depth, Radius, Thickness, Edge Softness, and Flattening when the
primitive schema supports them.

## Allowed

- Edit values inside the primitive property schema.
- Clamp or reject invalid values before they become current state.
- Keep the previous valid preview while an update compiles.
- Export the current primitive with truthful limitations.
- Use deterministic property presets later when each preset validates.

## Not Allowed

- Vertex, face, loop, cage, sculpt, or boolean mesh operations.
- Raw mesh transforms or arbitrary object transform handles.
- Imported mesh editing.
- Material/surface editing, UV/texturing, rigging, or animation.
- Runtime LLM control of mesh generation.
- Freeform composition or unrestricted scene modeling.

## Composition Boundary

Future composition will use named anchors and bounded attachment policies.
LLMs may suggest property values, presets, or compositions later, but validators
decide legality and illegal suggestions cannot become current state.
