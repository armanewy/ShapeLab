# ObjectPlan Materialization Contracts v1

Date: 2026-07-01

## Scope

ObjectPlan materialization converts a validated ObjectPlan into an internal
draft graph:

- ObjectPlan nodes become primitive instances.
- ObjectPlan attachments become constrained composition placements.
- Property values are applied only through primitive property schemas.
- Validation remains mandatory.
- Output remains Draft and review-required.

Materialization is not approval, runtime LLM integration, public catalog
publishing, material/surface work, UV/texturing, rigging, animation, imported
mesh editing, or raw mesh intake.

## Request Contract

`ObjectPlanMaterializationRequest` contains:

- `plan`
- `materialization_policy`
- `target_preview_profile`
- `output_mode`

`MaterializationPolicy` contains:

- `require_valid_plan`
- `require_supported_primitives`
- `require_supported_attachments`
- `preserve_node_labels`
- `forbid_catalog_publish`

The default policy requires valid plans, supported primitives, supported
attachments, preserved labels, and forbidden catalog publishing.

## Draft Contract

`MaterializedObjectDraft` contains:

- `draft_id`
- `source_plan_id`
- `status`
- `primitive_instances`
- `composition_document`
- `unresolved_nodes`
- `unresolved_attachments`
- `validation_report`
- `review_tier`
- `user_review_required`
- `publish_allowed`

`publish_allowed` is always false in v1. `review_tier` is Draft.

## Supported v1 Shape Scope

Supported primitives:

- Box Primitive
- Flat Panel Primitive
- Sphere Primitive

Supported composition:

- zero attachments
- Flat Panel Primitive plus Sphere Primitive attached from
  `right_side_handle_zone` to `back_mount_point`

Unsupported nodes or attachments must be reported honestly in unresolved lists.

## Status

`MaterializationStatus` values:

- `Passed`: all supported nodes and attachments materialized.
- `Partial`: some content materialized and unresolved content was reported.
- `Failed`: validation failed or policy requirements were violated.

## Review Summary

`MaterializedObjectSummary` gives product-safe review counts and the next
action:

- Review
- Simplify
- Regenerate
- Blocked

The summary must not expose internal terms such as kernel, module, provider,
slot, topology, fingerprint, conformance, artifact, or raw transform.

## Safety Rules

Materialization fails or reports unresolved content when:

- the plan is invalid
- a primitive kind is unsupported
- a property is unsupported or out of domain
- an attachment is unsupported
- anchors are incompatible
- raw mesh payloads are present
- arbitrary transform payloads are present
- public catalog publishing is requested

Review and contact-sheet evidence remain required before any draft can be kept.
