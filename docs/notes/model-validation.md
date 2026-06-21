# Model Validation Notes

## Scope

This branch adds additive model validation for generated static assets in
`shape_compile::validation`. Export packages carry model validation issues in
their validation sidecar. The implicit editor, schema-2 deformation decompiler
behavior, and bend/deformation contracts are unchanged.

## Reports

`validate_model` returns a `ModelValidationReport` with deterministic
`ValidationIssue` ordering and aggregate `QualityMetrics`.

Issues include severity, stable code, involved part instances, optional
operation provenance, a message, and an optional world-space location. Metrics
cover part, polygon, and triangle counts; quad fraction; closed-manifold part
fraction; minimum edge length; maximum face aspect ratio; hard-edge count;
part/region count; provenance coverage; and accidental intersection count.

## Part Validation

Part-local checks run against each compiled world mesh and report all
discoverable issues without repairing geometry. The validator wraps existing
polygon validation and adds explicit checks for non-finite values, coincident
duplicate vertices, duplicate faces, nonmanifold edges, undeclared or missing
expected open boundaries, inconsistent winding, minimum edge length, minimum
face area, extreme aspect ratio, inward normals for closed manifold parts, and
isolated face components inside a part.

## Assembly Validation

Assembly checks use `ModelValidationConfig` metadata supplied by the caller or
derived from an `AssetRecipe` with `validation_config_from_recipe`.
Required parts and expected socket attachments detect missing attachments,
detached required parts, and invalid socket alignment. Pairwise AABB checks find
suspicious part pairs; disallowed positive-overlap pairs then run narrow-phase
triangle intersection checks. Authored clearance and containment relationships
are checked where declared, and part/triangle budgets are enforced from the same
config.

Intentional overlaps are allowed through explicit `PartRelationship` metadata,
with `MinimumClearance`, `MustTouch`, and `Containment` checks available for
declared pairs. Asset recipe relationship selectors can expand concrete
instances, generated operation occurrences, prototype occurrence families, part
tags, and definition role tags into concrete compiled part IDs.

## Limits

Clearance and containment are conservative static-asset checks. Containment is
currently bounds-based, and clearance uses bounds as a fast reject before
triangle distance. The validator is intended to flag generated-asset quality
problems deterministically; it does not mutate or repair meshes.
