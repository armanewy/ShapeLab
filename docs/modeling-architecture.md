# Part-Aware Modeling Architecture

Shape Lab now has contracts for a second modeling lane alongside the existing implicit editor and the same-topology deformation decompiler. The new lane is explicit, polygon-based, part-aware, and DCC-independent. It does not rely on Blender for geometry generation.

## Crates

```text
crates/shape-asset      serializable asset recipes, semantic IDs, parameters, sockets, attachments, edits, validation
crates/shape-poly       explicit indexed polygon topology, semantic metadata, triangulation, normals, adjacency, validation
crates/shape-modeling   deterministic generator dispatch and modeling operation contracts
crates/shape-compile    recipe-to-artifact compilation, validation, provenance, OBJ and Blender script export
crates/shape-modeling-assets benchmark explicit AssetRecipe constructors and checked-in JSON recipes
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
shape-modeling assembly evaluation
    |
    v
shape-compile validation, triangulation, provenance, statistics
    |
    v
AssetArtifact with grouped OBJ, preview triangles, and Blender reconstruction script
```

The explicit lane compiles complete authored assets. `RoundedBox`, `Cylinder`, `Frustum`, `Plate`, `Sweep`, `Lathe`, and `LiteralMesh` sources dispatch to deterministic generators. `Plate` supports constrained semantic cuts on planar faces: `RecessedPanelCut`, `RectangularThroughCut`, and `CircularThroughCut`. Multiple cuts can compose on one plate face when their frames are separated and their rectangular projections do not split another cut window. These cuts generate boundary-loop IDs, rim/wall/floor regions, hard-edge and bevel-eligibility metadata, and operation provenance. Reserved arbitrary boolean and deformation-program sources remain typed unsupported paths.

## Semantic Recipe Layer

`shape-asset` stores semantic parts rather than raw scene objects. A recipe contains reusable `PartDefinition` values, concrete `PartInstance` values, root instance order, editable parameters, locks, constraints, and next-ID counters.

Part definitions declare:

- a `GeometryRecipe` with one base `GeometrySource` and ordered `ModelingOperationSpec` values
- generic `SurfaceRegionSpec` values such as `PrimarySurface`, `Cap`, `Side`, `BevelBand`, `Panel`, `Trim`, `Rim`, `CutWall`, `Attachment`, `Interior`, and `Detail`
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

## Compile Pipeline

`compile_asset` now performs the explicit production path:

1. validate the `AssetRecipe`
2. generate one local mesh per referenced definition
3. evaluate parent transforms, socket attachments, mirrored instances, and linear/radial arrays through the assembly runtime
4. transform meshes into asset/world space
5. validate local and world polygon topology
6. triangulate every part and the combined preview mesh
7. build face and triangle provenance
8. compute deterministic statistics and source recipe hash
9. return typed contextual errors from asset, modeling, assembly, polygon, and JSON layers

The compiler rejects invalid indices, degenerate faces, nonmanifold topology, inconsistent winding, unexpected closed-part boundaries, non-finite split normals, and missing face provenance. Declared open boundaries are accepted only when edge metadata marks them as open or seam candidates.

## Export Outputs

`shape-cli model-demo` writes:

- `recipe.json`
- `asset.obj` with one named OBJ group per part occurrence
- `provenance.json`
- `validation.json`
- `statistics.json`
- `preview.png`
- `blender_reconstruct.py`

The Blender script creates one object per part occurrence from canonical arrays, preserves object/group names, writes semantic custom properties, assigns simple debug colors, verifies object topology and finite positions, saves `reconstructed.blend`, and supports `--verify-reopen`.

## ID Stability Contract

Part, operation, region, boundary-loop, and socket IDs are semantic IDs. They must remain stable when unrelated parameters change.

Vertex and face IDs are deterministic for a given topology signature. They are not promised to survive topology-changing parameters such as radial segment counts, array counts, or sweep profile resolution.

Provenance must always connect generated polygon elements back to semantic IDs. Later geometry generators should prefer stable operation and region attribution over positional inference.

## Boundaries

This lane deliberately avoids materials, textures, UV coordinates, rig data, animation, LLM integration, generic mesh booleans, arbitrary imported-mesh editing, and SDF production geometry. The current cut operations are controlled topology generators for supported plate faces, not arbitrary triangle-mesh subtraction. Surface regions, boundary loops, and seam candidates are metadata for future systems, not material or UV implementations.
