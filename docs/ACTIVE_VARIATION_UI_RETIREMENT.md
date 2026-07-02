# Active Variation UI Retirement

Date: 2026-06-30

The active primitive Make workflow is direct-edit only. Box Primitive, Flat
Panel Primitive, Sphere Primitive, and Panel with Knob show model preview, view
controls, property controls, Add to Pack, and Export. They do not show Try
ideas, generated idea trays, selected-candidate comparisons, candidate
acceptance, or candidate survivor/rejected copy.

## Active Make Contract

Primitive editing is property-schema based.

Current primitive Make screens expose direct property panels:

- Box Primitive: Width, Depth, Height, Edge Softness.
- Flat Panel Primitive: Width, Height, Thickness, Edge Softness.
- Sphere Primitive: Width, Height, Depth, Front Flatten, Back Flatten.
- Panel with Knob: bounded panel, knob form, and knob position properties.

The approved action copy is direct:

- Edit Box Primitive
- Edit Flat Panel
- Adjust dimensions
- Export current primitive

## Backend Boundary

Candidate generation is inactive in the current primitive product flow. Backend
candidate-like machinery may remain for internal quality evidence, contact-sheet
outputs, legacy tests, and later cleanup branches, but it is not a
product-visible primitive Make operation.

Future suggestions may return only as deterministic property presets. A preset
must be a named set of legal property values validated against primitive
property schemas, not random candidate generation.

## Still Blocked

This retirement does not approve material/surface editing, UV/texturing,
rigging, animation, runtime LLM integration, broad Family Studio UI, arbitrary
mesh transforms, vertices, faces, or Blender-like modeling controls.
