# Orchard Control Grammar

Date: 2026-07-01

## Purpose

Object Orchard should feel like a visual construction tool, not a DCC editor.

The product must not expose every internal value as a slider, and it must not
replace sliders with one-off bespoke widgets for every parameter. The scalable
rule is:

> Every user-facing tunable must map to a small, finite control grammar.

If a parameter cannot be expressed through this grammar, it should remain
internal, become a discrete preset, or stay out of the novice editing surface.

## User-facing rule

Users should edit objects through object-anchored controls:

- direct handles on or near the model;
- compact option chips near the affected feature;
- safe attachment anchors for composition;
- occasional precision popovers off the main path.

Users should not edit:

- vertices;
- faces;
- loops;
- cages;
- raw transforms;
- Boolean operations;
- UV islands;
- shader graphs;
- skeletons;
- animation curves.

## Control family 1: Stretch handles

Use stretch handles for size-like properties:

- Width
- Height
- Depth
- Length
- Thickness
- Radius-like extents when linear

Visual behavior:

- A small grip floats just outside the affected side or axis.
- Dragging outward/inward changes the bounded property.
- A label chip says words such as "Wider", "Taller", or "Thicker".
- The exact numeric value is not the main workflow.

Examples:

- Box Primitive: Width, Depth, Height
- Flat Panel: Width, Height, Thickness
- Sphere Primitive: Width, Height, Depth

## Control family 2: Profile handles

Use profile handles for edge and silhouette behavior:

- Edge Softness
- Bevel amount
- Corner profile
- Chamfer strength
- Lip radius

Visual behavior:

- A small corner puck or edge ring appears near the affected boundary.
- Dragging changes softness/profile within authored bounds.
- Tapping can open discrete choices such as Sharp, Soft, Rounded.

## Control family 3: Band handles

Use band handles for seam, trim, and belt-like features:

- Lid seam height
- Trim band position
- Trim band thickness
- Reinforcement band placement

Visual behavior:

- A thin floating band wraps around the object.
- Dragging up/down moves the band.
- Dragging outward changes thickness when supported.
- Tapping switches authored band style when supported.

## Control family 4: Pattern handles

Use pattern handles for count, spacing, and repeated elements:

- Panel divisions
- Slat count
- Rib spacing
- Vent density
- Repeated decorative modules

Visual behavior:

- A small repeated strip appears on the relevant surface.
- Dragging changes count or spacing through authored stops.
- Pattern edits must be discrete or snapped, not arbitrary noise.

## Control family 5: Attachment anchors

Use attachment anchors for composition:

- Add knob to panel
- Add foot to box
- Add handle to side
- Add hinge to edge

Visual behavior:

- Anchors are visible only in Add/Attach mode.
- Anchors appear only where attachment is legal.
- Tapping an anchor opens compatible additions.
- Position offsets stay bounded by the parent anchor policy.

## Control family 6: Option chips

Use option chips for non-spatial discrete choices:

- Sharp / Soft / Rounded
- Flat / Raised
- Hinge left / Hinge right
- Knob / Pull handle
- Preset shapes

Visual behavior:

- Small image or text chips appear near the affected feature or in a compact
  tray.
- Option chips never become a large inspector wall.

## Precision fallback

Object Orchard may eventually expose exact values, but only as a fallback:

- long-press a handle label;
- open a small precision popover;
- type an exact value within the property domain.

Precision is not the primary workflow.

## Authored stops

Controls should favor authored meaningful stops:

- Narrow / Standard / Wide
- Sharp / Soft / Rounded
- Low / Mid / High seam
- Two / Three / Four panel divisions

The interaction may feel fluid, but values should gently snap to coherent
family states. This keeps outputs consistent, reviewable, and easier for
offline ObjectPlan tooling to reproduce.

## Internal mapping

Each visible control maps to:

- a primitive property;
- a composition anchor offset;
- a deterministic preset;
- a feature capability;
- or a surface/material look once surface support is approved.

No visible control may mutate arbitrary mesh state.

## Blocked from the main workflow

The main workflow must not expose:

- free transform gizmos;
- arbitrary object translations/rotations;
- mesh edit mode;
- UV editing;
- generated variation trays;
- runtime LLM generation;
- public catalog publishing;
- game-ready claims.
