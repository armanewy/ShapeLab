# Part-Aware Modeling Architecture

Shape Lab has contracts for an explicit modeling lane alongside strict
reconstruction and same-topology deformation decompiler research paths. The
lane is polygon-based, part-aware, and DCC-independent. It does not rely on
Blender for geometry generation. Wave 31 removed the old native editor surfaces;
this document now describes core/headless modeling architecture rather than a
current product UI.

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

The explicit lane compiles complete authored assets. `RoundedBox`, `Cylinder`, `Frustum`, `Plate`, `Sweep`, `Lathe`, and `LiteralMesh` sources dispatch to deterministic generators. `Plate` and `RoundedBox` support constrained semantic cuts on planar primary faces: `RecessedPanelCut`, `RectangularThroughCut`, and `CircularThroughCut`. Multiple cuts can compose on one selected face when their frames are separated and their active rectangular projections do not split another cut window. These cuts generate boundary-loop IDs, rim/wall/floor regions, hard-edge and bevel-eligibility metadata, and operation provenance. `BevelBoundaryLoop` can consume supported cut entry, exit, or recess-floor loops and emit two replacement loops plus bevel-band faces with their own region and operation provenance. Reserved arbitrary boolean and deformation-program sources remain typed unsupported paths.

## Semantic Recipe Layer

`shape-asset` stores semantic parts rather than raw scene objects. A recipe contains reusable `PartDefinition` values, concrete `PartInstance` values, root instance order, editable parameters, locks, constraints, and next-ID counters.

Part definitions declare:

- a `GeometryRecipe` with one base `GeometrySource` and ordered `ModelingOperationSpec` values
- generic `SurfaceRegionSpec` values such as `PrimarySurface`, `Cap`, `Side`, `BevelBand`, `Panel`, `Trim`, `Rim`, `CutWall`, `Attachment`, `Interior`, and `Detail`
- generic `SocketSpec` values with local frames for later attachment and articulation work
- a local pivot frame and optional variant or production hints

Modeling operations are grouped into coarse phases: source configuration, local topology, boundary treatment, local transform, and assembly generation. Structural edits may reorder operations only within phase-compatible positions until the runtime supports true cross-phase sequential execution. Operation removal is explicit: callers either reject removal while parameter descriptors or authored variation hints still reference the operation, or request cascade cleanup of operation-owned metadata.

Recipe validation also checks operation phase order on loaded or hand-authored recipes. This keeps external JSON from placing boundary treatments before the local-topology cuts that produce their target loops.

Cut operations can be reflected as editable controls in tooling without
permanently authoring a `ParameterDescriptor` for every operation field.
Descriptor-free operation edits still flow through the recipe reducer,
topology-lock checks, validation, history, and compile jobs. Plate and
RoundedBox-backed controls derive their feasible position, size, rim, depth,
and segment ranges from the host face and topology locks instead of using fixed
global slider limits.

Recipes can also author semantic cut groups in variation metadata. These groups name repeated operation sets such as mounting holes or ventilation slots, validate that every member is a supported cut on the declared definition, and allow candidate search to vary repeated dimensions or spacing as one semantic proposal rather than unrelated one-off edits. Duplicating a grouped cut preserves its source group by default, and group metadata validates both member count and role-to-cut-family consistency.

Boundary-loop metadata has an explicit lifecycle. Cut operations produce live loops, while boundary-treatment operations may reference a loop or consume it and emit replacement loops. Operations distinguish direct boundary-loop outputs from dependency replacement outputs, and shared validation uses the combined declared-output set for uniqueness, next-ID checks, insertion availability, and serialization diagnostics. Compile validation checks the final mesh against the live loop set while still retaining consumed loops as historical provenance, so a bevel can replace an entry loop with outer/inner replacement loops without making the original cut invalid. Replacement-loop validation allows neighboring cut faces and bevel-band faces to carry their respective operation provenance while ordinary cut loops keep stricter same-operation incident-face checks.

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

This lane deliberately avoids materials, textures, UV coordinates, rig data, animation, LLM integration, generic mesh booleans, arbitrary imported-mesh editing, and SDF production geometry. The current cut operations are controlled topology generators for supported Plate faces and flat primary RoundedBox face patches, not arbitrary triangle-mesh subtraction. Surface regions, boundary loops, and seam candidates are metadata for future systems, not material or UV implementations.
