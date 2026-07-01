# Prototype Pack Brief Contracts v0

Status: contracts only. This is not Prototype Pack Mode.

Prototype Pack briefs define a future batch asset request format for Draft
outputs. They are intended to sit above ObjectPlan batch review once the product
is ready for a larger asset-creation workflow.

## Contract

`PrototypePackBrief` includes:

- `brief_id`
- `display_name`
- `purpose`
- `asset_requests`
- `supported_primitive_scope`
- `output_policy`
- `review_policy`

`AssetRequest` includes:

- `request_id`
- `display_name`
- `intended_use`
- `allowed_primitives`
- `allowed_compositions`
- `desired_count`
- `style_hint`
- `must_have_capabilities`
- `blocked_capabilities`

## V0 Scope

Current supported primitives are:

- Box Primitive
- Flat Panel Primitive
- Sphere Primitive

Current supported composition scope is:

- Panel with Knob

Current supported capabilities are Draft ObjectPlan output, review image
evidence, and geometry-only export evidence. Requested outputs remain Draft and
human review is required.

## Validation

Validation rejects:

- unsupported primitive or composition requests
- unsupported capabilities
- desired counts outside the V0 bound
- automatic approval
- public catalog publishing
- game-ready claims

## Product Boundary

This milestone does not add batch generation UI, runtime LLM integration,
automatic asset approval, public catalog publishing, materials, UV/texturing,
rigging, animation, or game-ready output.

ObjectPlan batch review remains the likely backend for a later Prototype Pack
workflow, but this branch only defines the brief contract and product-safe
summary format.
