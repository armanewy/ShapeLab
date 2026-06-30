# Primitive Property Schema Contracts

Date: 2026-06-30

Primitive editing is property-schema based. A primitive exposes a fixed,
immutable list of product-facing properties with bounded domains. Users edit
legal property values; validators decide whether those values can become current
state.

## Contract Shape

`PrimitivePropertySchema` defines:

- primitive kind
- display name
- identity summary
- properties
- constraints
- preview policy
- export policy

`PrimitiveProperty` defines:

- stable property ID
- product-facing display name
- value kind: Length, Ratio, Boolean, Choice, or Angle
- bounded domain
- default value
- geometry impact
- topology behavior
- product-facing description
- advanced flag

Continuous sliders are allowed only for geometry-preserving edits. Any
topology-changing primitive property must be a discrete choice, so invalid
continuous topology edits cannot become current state.

## Initial Schemas

- Box Primitive: Width, Depth, Height, Edge Softness.
- Flat Panel Primitive: Width, Height, Thickness, Edge Softness.
- Sphere Primitive: Width, Height, Depth, Front Flatten, Back Flatten.

Cylinder Primitive remains optional later work and is not active in the product
flow.

## Validation

The validator rejects missing required properties, non-finite domains, defaults
outside their domains, raw internal terms in product labels, continuous
topology-changing properties, and current values outside legal domains.

Property IDs are stable and product-safe. Labels and user-facing descriptions do
not expose scalar paths, provider IDs, internal operation IDs, vertices, faces,
loops, cages, boolean mesh operations, raw mesh transforms, or Blender-like
modeling concepts.

## Future Assistants

Future LLMs may propose property values or deterministic property presets, but
validators enforce domains. Mesh generation stays native and offline, and
runtime LLM integration remains blocked.

Future composition uses anchors and constrained attachments, not freeform scene
modeling or arbitrary mesh manipulation.
