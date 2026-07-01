# Relationship Contract v0

Date: 2026-07-01

Status: schema/contracts only.

## Goal

Relationship contracts define composition semantics before adding more
composition UI or visual handles. Composition is relationship semantics, not
welding, not arbitrary transforms, and not an export-time bake by default.

## Contract Shape

`RelationshipContract` has:

- stable `RelationshipId`;
- `RelationshipType`;
- optional parent and child endpoints;
- placement policy;
- orientation policy;
- scale policy;
- contact policy;
- edit, selection, and reset policy;
- export realization policy shell.

Supported relationship kinds include rigid child, surface-mounted,
embedded-feature, socketed-accessory, joint-attached, intentional-offset, VFX
child, pattern-instance, collision-proxy, render-only-decoration, and baked
union.

## Placement Policies

Fixed distance and proportional placement are distinct first-class policies:

- `FixedOffsetFromEdge` preserves an authored offset from a named edge.
- `ProportionalUv` preserves normalized surface placement.
- `CenteredInZone` reserves a named placement zone.
- `PreserveCurrentOnDetach` keeps detach behavior explicit.

## Validation

Validation checks:

- populated endpoints must exist;
- relationship graphs must not form cycles;
- fixed offsets must be finite;
- proportional U/V values must be in `[0, 1]`;
- scale ranges must be positive and ordered;
- contact clearances must be non-negative.

## Export Boundary

Export realization is separate from authoring relationship semantics. A
relationship can later be preserved as nodes, sidecar semantics, or baked only
when the export gate proves that behavior. This contract does not implement
export, material, collision, motion, terrain, or game-ready behavior.
