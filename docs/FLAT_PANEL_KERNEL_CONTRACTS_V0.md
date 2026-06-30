# Flat Panel Kernel Contracts v0

Flat Panel is the proposed second kernel proof after Box Primitive. It is not a
door yet. It is an upright, flat, clay panel with front/back orientation and
future attachment zones.

## Why Flat Panel comes before Door

The Box ladder taught an important naming rule: the model name must match what
the user can see. A plain panel should not be called a door until it has visible
door cues such as hinge-side detail, handle/knob detail, frame/inset panels, or
open/close behavior.

Flat Panel lets Object Orchard prove a non-box kernel without overclaiming.

## User-facing identity

Users should see language like:

- Flat Panel
- upright panel
- front and back
- panel ideas
- later: hinge edge, handle, inset panel

Users should not see:

- kernel
- module
- provider
- slot
- placement zone
- topology
- rigging
- animation

## Internal identity

The internal `FlatPanelKernel` defines:

- one upright flat panel
- readable width, height, and thickness
- meaningful front/back orientation
- support edge / bottom contact
- placement zones for future features
- blocked capabilities for material looks, rigging, animation, and open/close
  motion

## Placement zones

The primitive emits deterministic placement zones:

- front face
- back face
- left hinge-candidate edge
- right handle candidate area
- inset panel candidate area
- bottom support edge

These are not user-visible mesh selections. They are internal evidence for
future feature modules.

## Feature modules

Initial flat-panel feature module contracts include:

- Panel Body
- Hinge Edge
- Panel Handle

Only `Panel Body` belongs to the primitive baseline. Hinge Edge and Panel Handle
are future features. They must pass separate visual gates before any product
profile claims to be a door.

## Boundaries

Flat Panel v0 does not implement:

- a product-visible Door profile
- hinge geometry in the app
- handle geometry in the app
- open/close motion
- collision or gameplay metadata
- UVs or textures
- material looks
- rigging or animation

## Next proof

After Box Primitive, Lidded Box, and Trimmed Box are stable, Flat Panel should be
implemented as the second kernel proof. If Flat Panel passes, the next single
feature should be Hinge Edge. Only after that should Object Orchard consider a
Door Panel / Door Primitive product profile.
