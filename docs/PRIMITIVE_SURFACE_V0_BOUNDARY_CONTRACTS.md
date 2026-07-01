# Primitive Surface V0 Boundary Contracts

Status: contracts only; no user-facing Surface UI.

Primitive Surface V0 defines future primitive-aware surface policy boundaries.
It does not add a material editor, UV editing UI, texture files, textured
export, rigging, animation, runtime LLM integration, public catalog publishing,
or game-ready status.

## Capability Contract

`PrimitiveSurfaceCapability` includes:

- `primitive_kind`
- `supported`
- `uv_policy`
- `material_slot_policy`
- `allowed_surface_properties`
- `blocked_reasons`
- `review_required`

V0 keeps `supported: false`, `allowed_surface_properties: []`, and
`review_required: true` for every policy.

## UV Policies

Defined policy candidates:

- `None`
- `BoxProjection`
- `PlanarProjection`
- `SphericalProjection`
- `CylindricalProjection`
- `PerNodePrimitivePolicy`

Initial disabled candidates:

- Box Primitive: `BoxProjection`
- Flat Panel Primitive: `PlanarProjection`
- Sphere Primitive: `SphericalProjection`
- Panel with Knob: `PerNodePrimitivePolicy`

These are future policy candidates only. They do not produce UV evidence yet.

## Material Slot Policies

Defined policy candidates:

- `NeutralClayOnly`
- `SingleMaterialSlot`
- `PerPrimitiveSlot`
- `PerFaceGroupSlot`

Primitive Surface V0 uses `NeutralClayOnly`. Other slot policies are defined so
future work has names, but they remain disabled until validation and evidence
exist.

## Enablement Blockers

Primitive Surface cannot enable unless:

- geometry export remains stable
- slot policy validation exists
- UV policy evidence exists
- texture/contact-sheet evidence exists
- export reports stay truthful

## Explicit Non-Goals

- no user-facing surface UI
- no material editor
- no UV editing
- no texture path emission
- no texture export
- no material-look export
- no collision/gameplay metadata
- no rigging or animation
- no Godot-ready or game-ready claim

Primitive-aware UV policies are the future path, but this contract only names
and blocks them until evidence exists.
