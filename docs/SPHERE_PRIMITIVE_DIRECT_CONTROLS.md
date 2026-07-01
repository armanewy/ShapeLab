# Sphere Primitive Direct Controls

Date: 2026-06-30

## Status

`IMPLEMENTED`

Sphere Primitive is now a direct editable primitive in the Make workflow. It is
a closed round clay volume controlled by bounded properties, not a door knob,
not a composition, and not a free sculpting surface.

## Direct Properties

Sphere Primitive exposes:

- Width
- Height
- Depth
- Front Flatten
- Back Flatten

Width, Height, and Depth scale the round volume through the authored primitive
profile. Front Flatten and Back Flatten adjust bounded profile endpoints to
create flatter round forms while preserving the direct primitive contract.

Every control is a bounded numeric property with visible domain text, stepper
controls, validation before state changes, and reset support.

## Knob-Like Form Preset

The Knob-like form preset is a deterministic property preset:

- Width: 0.72
- Height: 0.72
- Depth: 0.38
- Front Flatten: 0.18
- Back Flatten: 0.62

The preset is not generated variation. It is a named set of legal property
values applied through the same validation and rebuild path as manual edits.

## Product Boundary

Sphere Primitive does not expose vertex editing, face selection, sculpting,
boolean cutting, arbitrary transforms, material looks, UV/texturing, rigging,
animation, generated ideas, candidate trays, or part chips.

The branch does not claim Door, Door Knob, or composition behavior. Attachment
to a panel remains blocked until the primitive composition contract and
composition prototype branches.

## Validation

The branch adds focused catalog tests for:

- fixture validation and export
- property schema alignment
- Width, Height, and Depth geometry changes
- Front Flatten and Back Flatten geometry changes
- Knob-like form preset legality
- absence of generated idea UI and unsupported product claims
