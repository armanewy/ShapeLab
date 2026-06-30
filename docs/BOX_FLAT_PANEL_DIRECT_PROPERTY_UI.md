# Box / Flat Panel Direct Property UI

Date: 2026-06-30

## Status

`IMPLEMENTED_FOR_ACTIVE_PRIMITIVES`

Box Primitive and Flat Panel Primitive now expose direct bounded property
controls in the Make workflow. The active surface no longer asks users to pick
generated ideas or choose a Proportions option before editing the primitive.

## Box Primitive

The Box Primitive Make panel exposes:

- Width
- Depth
- Height
- Edge Softness

Each property is shown as a bounded numeric stepper with visible domain text,
current value, and reset support. Width, Depth, and Height are product
dimensions mapped to the rounded box half-extents during compilation. Edge
Softness remains a bounded softness value mapped into the rounded radius.

## Flat Panel Primitive

The Flat Panel Primitive Make panel exposes:

- Width
- Height
- Thickness
- Edge Softness

Each property follows the same direct behavior: visible domain, bounded
increment/decrement, reset to authored default, and validation before the value
can become current state.

## Live Feedback

Direct edits request an exact rebuild through the existing Make job pipeline.
When a prior valid preview exists, the model stage keeps that preview visible
and labels it as updating while the rebuild runs. Pack and export actions stay
disabled until the exact updated build is ready.

## View Controls

The Make stage exposes view-only inspection copy:

- Orbit view
- Reset view
- Axis view

These controls are for navigation and orientation only. They do not introduce
mesh transform gizmos, vertex editing, face selection, object handles, or
freeform modeling controls.

## Product Boundary

This branch does not add generated ideas, candidate trays, selected-candidate
comparison, composition, material looks, UV tools, rigging, animation, runtime
LLM behavior, or Family Studio UI. Future suggestions may return only as named,
deterministic property presets.
