# Kit Capability Adapter v0

Status: contracts/adapter only. No app UI, runtime LLM integration, generated
candidate tray, public catalog publishing, UV editor, rigging, animation, or
game-ready claim is added by this milestone.

## Purpose

Capability cards are user-facing wrappers for "what can change" in a future
Direct Kit flow. They map plain labels and descriptions to existing primitive
property schemas, deterministic presets, future surface boundaries, safe
composition controls, and geometry-only export options.

## Sources

Primitive property cards map to:

- Box Primitive: Width, Depth, Height, Edge Softness
- Flat Panel Primitive: Width, Height, Thickness, Edge Softness
- Sphere Primitive: Width, Height, Depth, Front Flatten, Back Flatten
- Panel with Knob: panel dimensions, knob form, and bounded knob safe position

Preset cards appear as "Saved Shapes" and map to deterministic preset sets.
They must not be described as generated variations.

Material-look cards remain Later unless surface descriptors and evidence exist.
The adapter does not expose UV wording or editor behavior.

## Validation

Validation checks that:

- every card maps to a known property, preset set, surface boundary,
  composition control, or export option
- card copy hides internal technical terms
- Later and Blocked cards include plain reasons
- cards do not imply generated variations, public publishing, runtime LLM
  behavior, rigging, animation, or game-ready status

## Product Boundary

Capability cards are wrappers around existing contracts. They do not generate
assets, approve kits, publish kits, or add Family Studio Lite UI by themselves.
