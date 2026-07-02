# Public Contracts

This file summarizes the Wave 0 public API surface. Rust source is authoritative.

## Coordinate and Numeric Contract

- `f32` is the MVP geometry scalar.
- Coordinates are right handed.
- Positive Y is up.
- Primitive local forward direction is negative Z.
- Capsule and cylinder primary axes are local Y.
- SDF values are negative inside and positive outside.
- Surface is the zero isocontour.
- Mesh triangle winding is counterclockwise when viewed from outside.
- Normals point outward.
- Transform rotation is stored as XYZ Euler degrees in the document and converted internally when evaluated.
- Nonuniform transformed SDF distance is multiplied by the smallest absolute scale component. This preserves the zero set but is not an exact signed distance.

## Canonical Parameter Keys

```text
transform.translation.x
transform.translation.y
transform.translation.z
transform.rotation_degrees.x
transform.rotation_degrees.y
transform.rotation_degrees.z
transform.scale.x
transform.scale.y
transform.scale.z
primitive.radius
primitive.half_extents.x
primitive.half_extents.y
primitive.half_extents.z
primitive.roundness
primitive.half_length
primitive.half_height
primitive.major_radius
primitive.minor_radius
csg.smoothness
```

## Crate Contracts

`orchard-core-legacy` defines `Scalar`, IDs, `Transform3`, `Aabb`, primitive and CSG node kinds, `ShapeDocument`, parameter descriptors, edit programs, validation reports, and document helper functions.

`orchard-field` defines `ScalarField`, `CompiledField`, `compile_document`, `GridSpec`, and `sample_grid`.

`orchard-mesh` defines `TriangleMesh`, `MeshSettings`, `mesh_field`, `write_obj`, and `write_obj_to_path`.

`orchard-search-internal` defines exploration modes, target scopes, `SearchRequest`, `ShapeDescriptor`, `Candidate`, and `generate_candidates`.

`orchard-project` defines `Revision`, `Project`, and history/persistence methods.

`orchard-presets` defines `PresetId`, `PresetMetadata`, `list_presets`, and `build_preset`.

`orchard-render` defines `OrbitCamera`, `RenderSettings`, `RenderedImage`, `fit_camera_to_bounds`, and `render_mesh`.
