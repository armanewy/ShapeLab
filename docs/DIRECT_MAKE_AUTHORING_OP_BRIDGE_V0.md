# Direct Make AuthoringOp Bridge v0

Status: Wave 2 implementation slice.

This branch adds the first bridge from a product-visible direct primitive edit
to the canonical `AuthoringOp` lane. It does not add visual handles, change the
Make UI, add materials, add collision, add motion, add terrain, or change export
claims.

## Covered Edit

V0 covers one edit path:

- Box Primitive
- Width control
- `FoundryCommand::SetControl`
- `AuthoringOp::SetProperty`

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
- one-entry `AuthoringOpLog`
- `AuthoringOpLogEntry`
- replay validation report

For the current Box Primitive fixture, the product width value maps to the
compiled recipe half extent on `geometry.rounded_box.half_extents.x`, so the
authored recipe scalar is half of the visible width value.

## Tests

Tests prove:

- Box width emits one `AuthoringOp::SetProperty` breadcrumb.
- Replaying the breadcrumb log updates the recipe deterministically.
- The existing direct Make UI behavior still schedules and applies the current
  Foundry edit path.
- Simulated drag samples can coalesce into one committed set-property operation.

## Boundaries

Still blocked:

- visual handles
- routing every direct primitive property through `AuthoringOp`
- relationship authoring UI
- surface/material behavior
- UV editing
- collision, rigging, animation, terrain, or game-ready claims
