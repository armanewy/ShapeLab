# Polygon Topology Notes

## Explicit Mesh Contract

`shape-poly` stores polygon topology as positions, stable vertex `ElementId`s,
polygon faces, per-face provenance metadata, edge metadata, topology signature,
and bounds. Polygon faces remain the authoritative topology; triangulation is a
derived preview/export form.

Open boundary edges are valid only when declared with `BoundaryRole::OpenBoundary`.
Interior hard edges, feature edges, attachment edges, and seam candidates are
metadata on canonical edge keys and participate in split-normal generation.

## Validation

Validation reports all discovered issues instead of stopping at the first one.
The checks cover empty and non-finite positions, invalid indices, undersized
faces, repeated consecutive vertices, repeated vertices within a face, zero-area
faces, duplicate vertex and face IDs, duplicate directed edges, inconsistent
winding between manifold neighbors, nonmanifold edges, undeclared open
boundaries, metadata length mismatches, and invalid zero-valued semantic
provenance or region metadata.

## Deterministic Derived Data

Adjacency includes vertex-to-face incidence, canonical edge-to-face incidence,
face neighbors, boundary loops, and connected face components. Outputs are sorted
or map-backed so repeated builds produce identical results.

Triangulation preserves source polygon faces and maps every emitted triangle to
the source polygon face, region, part instance, and operation. Convex quads use a
stable diagonal rule, convex n-gons use a stable fan, and simple concave polygons
use deterministic ear clipping after rejecting self-intersections.

## Normals, Transforms, and Combination

Face normals use polygon winding. Smooth vertex normals average incident face
normals. Split normals respect hard edge classifications, hard boundary roles,
seam/feature/attachment roles, and smoothing-group boundaries; triangulation
duplicates output vertices only where a source vertex needs more than one split
normal.

Polygon transforms preserve semantic IDs and metadata while recomputing bounds
and topology signatures. Triangle mesh transforms use inverse-transpose normal
handling. Mesh combination remaps array indices, preserves `ElementId`s, and
rejects vertex or face ID collisions.
