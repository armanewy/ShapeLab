# ObjectPlan DSL Contracts

ObjectPlan is the structured input format for offline primitive planning. It is
not plain text, raw mesh generation, imported mesh editing, or Blender-like
scene editing.

An ObjectPlan contains:

- `schema_version`
- `plan_id`
- `display_name`
- `intent_summary`
- primitive `nodes`
- safe-anchor `attachments`
- `validation_policy`
- `review_tier`
- `provenance`

Each node names one supported primitive kind and supplies bounded primitive
property values. The current supported node kinds are Box Primitive, Flat Panel
Primitive, and Sphere Primitive. Property values must match the primitive
property schemas and remain inside each property domain.

Attachments connect existing nodes through named anchors. They reuse the safe
primitive composition rules: the parent anchor must allow the child primitive
kind, the child anchor must be a supported mount point, offsets must stay
bounded, orientation must be derived, and scale remains controlled by primitive
properties.

ObjectPlan validation rejects:

- unsupported primitive kinds
- unknown primitive properties
- property values outside the approved domain
- attachments that reference missing nodes
- incompatible parent or child anchors
- arbitrary matrix or raw transform payloads
- raw mesh payloads
- absolute file paths
- direct public catalog publishing

Review tiers are Draft, Personal, and Reviewed. Missing review-tier fields
deserialize as Draft. Offline LLM drafts remain Draft until reviewed by a human
or trusted local workflow.

The product-safe summary formatter may describe primitives used, adjustable
properties, safe attachments, and review tier. It must not expose internal
terms such as kernel, module, provider, slot, topology, fingerprint, raw
transform, or mesh payload.
