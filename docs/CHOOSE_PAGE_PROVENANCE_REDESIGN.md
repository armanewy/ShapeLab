# Choose Page Provenance Redesign

Date: 2026-07-01

## Status

Choose page provenance grouping is now the default direction for Object
Orchard starting points. The page is no longer treated as a generic template
browser. It groups startable profiles by their source primitive and displays
derived entries under that source.

## Default Grouping

Default Choose shows:

- Box Primitive
  - Lidded Box, derived from Box Primitive + Lid Seam
- Flat Panel Primitive
  - Hinged Panel, derived from Flat Panel Primitive + Hinge Edge
  - Panel with Knob, derived from Flat Panel Primitive + Sphere attachment
- Sphere Primitive
  - Knob-like Form, shown as a preset rather than an asset family

Handled Panel remains historical evidence and is hidden from default Choose.
It can appear only in preview/internal mode as historical proof.

## Product Rules

- Primitives appear before derived entries.
- Derived entries must say what they are derived from.
- Presets are labeled as presets, not new asset families.
- The Start action still starts one selected profile.
- Broad category chips and template counts remain absent from starter mode.
- Generated variation trays remain blocked.
- No material editor, UV editor, rigging, animation, runtime LLM, public
  publishing, or game-ready claims are introduced.

## Known Limits

This branch does not add Make handles or a new Make stage. It only changes the
Choose page structure and copy. The Knob-like Form preset is visible as
provenance information under Sphere Primitive; it is not a new public asset
family.
