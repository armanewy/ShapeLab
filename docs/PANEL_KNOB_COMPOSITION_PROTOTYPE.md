# Panel with Knob Composition Prototype

Date: 2026-06-30

## Status

`IMPLEMENTED`

Panel with Knob proves the first constrained composition milestone: one Flat
Panel Primitive plus one knob-like Sphere form attached through a safe anchor.

This is not a Door claim. It has no open/close motion, rigging, animation,
material/surface editing, UV/texturing, free transform gizmo, or generated idea
workflow.

## Composition

The prototype is represented by a validated `PrimitiveCompositionDocument`:

- root node: Flat Panel Primitive
- child node: Sphere Primitive knob-like form
- parent anchor: `right_side_handle_zone`
- child anchor: `back_mount_point`
- offset policy: bounded normalized handle-zone position
- orientation policy: derived from the parent anchor
- scale policy: child keeps its schema-controlled size

Invalid anchors, invalid property values, raw transforms, and out-of-bounds
position values are rejected before they can become current state.

## Direct Controls

Panel controls:

- Panel Width
- Panel Height
- Panel Thickness
- Panel Edge Softness

Knob controls:

- Knob Width
- Knob Height
- Knob Depth
- Knob Front Flatten
- Knob Back Flatten
- Knob Horizontal Position
- Knob Vertical Position

The first seven controls remain primary for the novice kit limit. The remaining
visible controls appear in the Make inspector overflow so users can still tune
edge softness, flattening, and bounded knob position without exposing generated
variation UI.

## Validation

The branch adds tests that verify:

- the composition document validates
- the knob attaches to the panel handle zone
- the knob remains attached when panel proportions change
- knob position stays inside bounded handle-zone ranges
- export remains clean
- no Door, open/close, motion, material, rigging, animation, vertex editing, or
  free-transform claim appears in product copy
