# Kernel Registry Property Bridge v0

Status: Wave 2 contract bridge, implemented without UI behavior changes.

This bridge connects current direct primitive property schemas to the future
semantic AssetRecipe / AuthoringOp lane. It does not change Make UI behavior,
export behavior, relationship behavior, surface behavior, collision, motion, or
terrain.

## Contracts

Shared descriptor contracts live in `orchard-asset`:

- `KernelKind`
- `KernelDescriptor`
- `PropertyDescriptor`
- `OrchardControlFamily`
- `PropertyAuthoringEffect`
- `PropertyAffect`

`orchard-modeling` owns the current kernel registry. `orchard-foundry` owns the
bridge from existing primitive schemas into shared property descriptors.

## Current Kernel Entries

The registry includes:

- Box Primitive
- Flat Panel Primitive
- Sphere Primitive
- Panel with Knob composition

Panel with Knob remains a composition-backed profile. The registry describes
its bounded Panel, Knob, and Placement properties, but does not add new UI or
relationship migration behavior.

## Control Family Mapping

Current beginner-facing properties map to the finite Orchard control grammar:

- Width, Depth, Height, and Thickness map to `Stretch`.
- Edge Softness maps to `Profile`.
- Sphere Front Flatten and Back Flatten map to `Profile`.
- Panel with Knob placement maps to `Attachment`.

No property exposes a raw scalar, mesh, transform, vertex, face, or freeform
modeling path to product UI. Descriptor paths are semantic bridge paths for
authoring, not UI labels.

## Authoring Boundary

Every current descriptor maps to `PropertyAuthoringEffect::SetProperty`.
Prompt 8 may route one direct Make edit through `AuthoringOp::SetProperty`, but
this prompt only creates the descriptor bridge and tests the mapping.

## Blocked Work

Still blocked in this prompt:

- app UI changes
- visual handles
- export implementation changes
- surface/material workflow
- UV editing
- collision or gameplay metadata
- rigging or animation
- terrain behavior
- runtime LLM integration
- public catalog publishing
