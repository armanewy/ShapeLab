# Part-Aware Modeling Architecture

Shape Lab now has contracts for a second modeling lane alongside the existing implicit editor and the same-topology deformation decompiler. The new lane is explicit, polygon-based, part-aware, and DCC-independent. It does not rely on Blender for geometry generation.

## Crates

```text
crates/shape-asset      serializable asset recipes, semantic IDs, parameters, sockets, attachments, edits, validation
crates/shape-poly       explicit indexed polygon topology, semantic metadata, triangulation, normals, adjacency, validation
crates/shape-modeling   deterministic generator dispatch and modeling operation contracts
crates/shape-compile    recipe-to-artifact compilation, combined preview meshes, provenance, validation, export stubs
```

The existing `shape-core` `ShapeDocument` remains the implicit MVP editor model. The existing `shape-decompiler` package path remains the same-topology mesh-pair replay system. The explicit modeling lane is additive and does not change either behavior.

## Data Flow

```text
AssetRecipe
    |
    v
shape-modeling generator dispatch
    |
    v
GeneratedPart polygon meshes
    |
    v
shape-compile part transforms and combination
    |
    v
AssetArtifact with preview triangles and provenance
```

Wave 0 defines compileable contracts only. Primitive generators, paneling, trim, arrays, and production exporters are intentionally stubbed with typed unsupported errors until later waves implement deterministic geometry.

## Semantic Recipe Layer

`shape-asset` stores semantic parts rather than raw scene objects. A recipe contains reusable `PartDefinition` values, concrete `PartInstance` values, root instance order, editable parameters, locks, constraints, and next-ID counters.

Part definitions declare:

- a `GeometryRecipe` with one base `GeometrySource` and ordered `ModelingOperationSpec` values
- generic `SurfaceRegionSpec` values such as `PrimarySurface`, `Cap`, `Side`, `BevelBand`, `Panel`, `Trim`, `Attachment`, `Interior`, and `Detail`
- generic `SocketSpec` values with local frames for later attachment and articulation work
- a local pivot frame and optional variant or production hints

Part instances declare:

- the referenced definition
- optional parent instance
- local transform
- optional socket attachment
- enabled state
- semantic tags
- optional generating operation

## Polygon Layer

`shape-poly` owns explicit polygon topology. A `PolygonMesh` stores positions, stable vertex IDs, polygon faces, face metadata, edge metadata, a topology signature, and bounds. Face and edge metadata connect generated elements to part definitions, part instances, regions, operations, smoothing groups, boundary roles, and seam candidates.

`TriangulatedPolygonMesh` is a derived preview/export form. It preserves maps from triangles back to polygon faces, regions, parts, and stable vertex IDs.

## ID Stability Contract

Part, operation, region, and socket IDs are semantic IDs. They must remain stable when unrelated parameters change.

Vertex and face IDs are deterministic for a given topology signature. They are not promised to survive topology-changing parameters such as radial segment counts, array counts, or sweep profile resolution.

Provenance must always connect generated polygon elements back to semantic IDs. Later geometry generators should prefer stable operation and region attribution over positional inference.

## Boundaries

This lane deliberately avoids materials, textures, UV coordinates, rig data, animation, LLM integration, generic mesh booleans, arbitrary imported-mesh editing, and SDF production geometry. Surface regions and seam candidates are metadata for future systems, not material or UV implementations.
