# Direct Make AuthoringOp Bridge v0

Status: Post-cleanup foundation hard gate coverage.

This bridge records product-visible Direct Make scalar edits in the canonical
`AuthoringOp` lane. It does not add visual handles, change the Make UI, add
materials, add collision, add motion, add terrain, or change export claims.

## Covered Edit

The bridge covers active Direct Make scalar controls for:

- Box Primitive
- Flat Panel Primitive
- Sphere Primitive
- Panel with Knob

Covered commands are `FoundryCommand::SetControl` scalar values that compile to
one or more changed recipe scalar parameters. Each changed scalar is recorded as
an `AuthoringOp::SetProperty` entry.

The existing Foundry command job remains the source of current UI behavior. The
bridge records a replayable authoring breadcrumb against the current compiled
`AssetRecipe` so the product-visible edit can be inspected and replayed through
the canonical authoring lane.

## Breadcrumb

The internal breadcrumb records:

- source control id
- semantic property id
- target recipe parameter
- recipe path
- requested control value
- authored recipe scalar value
- changed scalar parameters
- replayable `AuthoringOpLog`
- `AuthoringOpLogEntry`
- replay validation report

Some controls compile to one scalar path; controls such as Panel with Knob knob
depth may compile to multiple scalar paths. The breadcrumb log stores every
changed scalar so replay reproduces the direct edit deterministically.

## Tests

Tests prove:

- Box Width, Depth, Height, and Edge Softness emit `AuthoringOp::SetProperty`
  breadcrumbs.
- Flat Panel Width, Height, Thickness, and Edge Softness emit
  `AuthoringOp::SetProperty` breadcrumbs.
- Sphere Width, Height, Depth, Front Flatten, and Back Flatten emit
  `AuthoringOp::SetProperty` breadcrumbs.
- Panel with Knob panel dimensions, knob form controls, and knob placement
  controls emit replayable `AuthoringOp::SetProperty` breadcrumbs.
- Replaying each breadcrumb log updates the recipe deterministically.
- The existing direct Make UI behavior still schedules and applies the current
  Foundry edit path.
- Simulated drag samples can coalesce into one committed set-property operation.

## Boundaries

Still blocked:

- visual handles
- relationship authoring UI
- surface/material behavior
- UV editing
- collision, rigging, animation, terrain, or game-ready claims
