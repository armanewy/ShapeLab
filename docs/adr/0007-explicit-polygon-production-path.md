# ADR 0007: Explicit Polygon Production Path

## Decision

The part-aware modeling lane will produce explicit indexed polygon topology through `shape-poly` rather than using the implicit field mesher or a DCC tool for production geometry.

## Rationale

Production assets need deterministic topology, semantic element metadata, and stable provenance. A polygon contract can carry face regions, edge boundary roles, seam candidates, smoothing groups, and topology signatures directly. This gives later generators a target that can support mechanical parts, panels, trim, arrays, lathe, and sweep operations without depending on Blender.

## Consequences

- The existing `shape-mesh` marching tetrahedra path remains the implicit MVP backend.
- The explicit path owns polygon validation, adjacency, triangulation, normal computation, transforms, and mesh combination.
- Heavy primitive and operation generation is not implemented in Wave 0. Generator functions return typed unsupported errors.
- Reserved boolean and deformation operations serialize in recipes but must return unsupported during compilation until later waves define their deterministic semantics.
