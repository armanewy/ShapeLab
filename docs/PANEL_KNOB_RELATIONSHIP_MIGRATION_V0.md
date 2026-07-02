# Panel with Knob Relationship Migration v0

Status: Wave 2 implementation slice.

Panel with Knob is now represented as relationship-backed composition
semantics in addition to the existing primitive composition document used by
the current product workflow.

## Relationship Shape

The supported relationship is:

- parent node: Flat Panel
- child node: knob-like Sphere
- relationship type: `SurfaceMounted`
- parent anchor: `front_handle_zone`
- child anchor: `back_mount_point`
- contact policy: surface contact with zero clearance
- orientation policy: align child to parent surface normal
- scale policy: preserve child scale
- export realization: preserve semantic sidecar

`right_side_handle_zone` remains accepted as a compatibility alias for older
draft plans, but new fixtures and generated ObjectPlans use `front_handle_zone`.

## Placement Policies

V0 treats fixed-distance placement and proportional placement as distinct
first-class policies.

- `FixedOffsetFromEdge` preserves distance from a named panel edge as the panel
  resizes.
- `ProportionalUv` preserves normalized horizontal and vertical placement on
  the panel surface.

The current app UI remains unchanged. This branch does not add visual handles,
free transforms, materials, collision, motion, terrain, or game-ready claims.

## ObjectPlan

Supported Panel with Knob ObjectPlans still use safe-anchor attachments, not raw
transforms. Materialization now also emits a relationship contract sidecar so
review/export tools can inspect the intended composition semantics.

Unknown anchors and raw transform payloads remain rejected by the closed
ObjectPlan and primitive composition schemas.

## Export Boundary

This migration does not decide whether export preserves nodes, submeshes,
merged meshes, or baked geometry. Export realization reporting is a later gate.
