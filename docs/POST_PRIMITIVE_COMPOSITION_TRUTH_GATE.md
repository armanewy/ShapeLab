# Post-Primitive Composition Truth Gate

Date: 2026-07-01

This gate freezes the product truth after direct primitive editing, Sphere
Primitive controls, safe primitive composition contracts, and the Panel with
Knob composition prototype.

## Active Primitives

The default product surface exposes these clay starting points:

- Box Primitive
- Lidded Box
- Flat Panel Primitive
- Sphere Primitive
- Hinged Panel
- Handled Panel
- Panel with Knob

The current direct primitive baselines are Box Primitive, Flat Panel Primitive,
and Sphere Primitive. Lidded Box, Hinged Panel, and Handled Panel remain feature
proofs. Panel with Knob is a safe-anchor composition proof.

## Direct Property Schemas

Direct property schemas exist for:

- Box Primitive: Width, Depth, Height, Edge Softness
- Flat Panel Primitive: Width, Height, Thickness, Edge Softness
- Sphere Primitive: Width, Height, Depth, Front Flatten, Back Flatten

Panel with Knob composes Flat Panel and Sphere properties with bounded knob
position controls.

## Active Compositions

The active composition proof is Panel with Knob. It attaches a knob-like Sphere
Primitive form to a Flat Panel handle zone through validated named anchors and
bounded offsets.

Object attachments are constrained by anchor contracts. The product does not
support arbitrary transforms, matrix editing, vertex editing, face editing, or
Blender-like scene editing.

## Variation UI

Active variation generation is retired from primitive Make. Generated ideas,
candidate trays, selected-candidate comparison, and candidate acceptance are
hidden from the active primitive workflow. Internal candidate-like code may
remain for legacy tests, quality evidence, and future deterministic preset work.

## ObjectPlan State

ObjectPlan contract and CLI groundwork exists for structured offline validation.
The visible app workflow for ObjectPlan review, offline LLM drafting, batch
review, and broad family generation is not implemented yet. Runtime LLM
integration is not implemented.

## Blocked Claims

No default product UI may claim support for:

- material or surface authoring workflow
- UV or texturing workflow
- rigging or skinning workflow
- animation workflow
- runtime LLM integration
- public catalog publishing
- game-ready packages

The export limitation may state that a result is not textured, rigged,
animated, or game-ready. That is a negative boundary statement, not a support
claim.

## Gate Result

The product truth is ready for the next ObjectPlan v0 work only if automated
tests keep active primitive UI free of generated-variation, unsupported mesh
editing, UV/texturing, rigging, animation, runtime LLM, and public publishing
claims.
