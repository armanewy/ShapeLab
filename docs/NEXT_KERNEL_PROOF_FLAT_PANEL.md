# Next Kernel Proof: Flat Panel

The next non-box proof should be Flat Panel, not a full Door.

## Reason

Object Orchard needs to prove that its family-authoring protocol can support a
second shape category before continuing deeper into the box ladder.

Flat Panel is a better immediate proof than Door because it avoids overclaiming.
A door requires recognizable door cues. A simple upright panel does not.

## Sequence

1. Flat Panel kernel contracts. Completed.
2. Flat Panel Primitive clay profile. Completed.
3. Flat Panel Make baseline. Completed.
4. Hinge Edge feature module. Completed as internal visual evidence.
5. Hinged Panel Make baseline. Completed.
6. Second-kernel Flat Panel integration. Next.
7. Handle/knob feature module.
8. Door Panel or Door Primitive only after door cues are visible.

## Flat Panel Primitive baseline

The default Visual Foundry catalog now includes Flat Panel Primitive after Box
Primitive and Lidded Box. The profile is one upright clay panel with readable
width, height, and thickness.

It exposes only:

- Proportions
- Edge Softness

It does not expose Door, hinge, handle, open/close, material, UV/texturing,
rigging, animation, or game-ready copy.

## Hinge Edge and Hinged Panel

Hinge Edge has passed as an internal feature-module proof. It adds one visible
clay side-edge strip to Flat Panel Primitive and produces Hinged Panel evidence
under `target/hinge-edge-feature-module-v0/`.

Hinged Panel has passed its Make baseline gate and is now app-visible. It keeps
the same simple Make loop as Flat Panel Primitive, adds the Hinge Edge control,
and remains a clay panel asset.

Hinged Panel is not Door, open/close behavior, rigging, animation, or a
material/textured asset.

## Still blocked

- Door naming before the visual gate passes.
- Open/close motion.
- Rigging/animation.
- Collision/gameplay behavior.
- UV/texturing/material looks.

## Why this supports arbitrary families

Box Primitive proves a volumetric closed-object kernel. Flat Panel proves an
upright planar-object kernel. If both work through the same authoring protocol,
Object Orchard is no longer only a box generator.
