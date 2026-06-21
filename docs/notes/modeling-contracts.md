# Modeling Contracts Notes

## Wave 0 Scope

This branch adds compileable contracts for the explicit part-aware modeling lane:

- `shape-asset` for serializable recipes, typed IDs, parameters, sockets, attachments, edits, and validation
- `shape-poly` for indexed polygon topology, metadata, triangulation, normals, adjacency, transforms, combination, and validation
- `shape-modeling` for deterministic generator dispatch and explicit unsupported errors
- `shape-compile` for recipe validation, part compilation orchestration, preview combination, provenance, and export stubs

No existing implicit editor behavior or schema-2 deformation decompiler behavior is changed.

## Unsupported Work Is Intentional

Primitive geometry generation is not implemented yet. The public generator entry points return `UnsupportedGeometry` for concrete source families. Reserved boolean and deformation operation specs serialize through `shape-asset` and return `UnsupportedOperation` from `shape-modeling` when compilation reaches them.

This keeps downstream waves from mistaking placeholder output for production topology.

## ID and Provenance Rules

Semantic IDs are stable across unrelated parameter changes:

- part definitions
- part instances
- operations
- regions
- sockets
- parameters
- revisions

Polygon vertex and face IDs are deterministic for a given topology signature. They are allowed to change when a topology-changing parameter changes.

Compiled artifacts must keep provenance from polygon elements to semantic IDs. Later systems should use that provenance for material assignment, UV seam selection, diagnostics, and export grouping instead of inferring intent from geometry alone.
