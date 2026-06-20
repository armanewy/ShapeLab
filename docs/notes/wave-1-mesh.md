# Wave 1 Mesh Notes

## Implemented

- Uniform-grid marching tetrahedra over `(resolution + 1)^3` samples.
- Fixed six-tetrahedron cube split along the `0 -> 6` diagonal.
- Linear edge interpolation against `MeshSettings.iso_value`.
- Gradient-oriented winding and per-vertex normals from central finite differences.
- Empty no-crossing fields return an empty `TriangleMesh` with empty bounds.
- OBJ export writes vertices, normals, and `v//vn` indexed faces to any `Write`.

## Limits

- Mesh output is triangle soup with duplicated vertices.
- There is no vertex welding, feature preservation, UV output, material output, or adaptive sampling.
- Bounds must be finite and have positive extent on every axis.
- Very small or zero gradients fall back to geometric face normals.

## Contract Issues

- `shape-mesh` needs a direct `glam.workspace = true` dependency because
  `shape_field::ScalarField::sample` takes `glam::Vec3` and the analytic tests
  must implement that trait. This is scoped to `crates/shape-mesh/Cargo.toml`.
- Cargo records that direct dependency in `Cargo.lock` package metadata. The
  branch avoids changing dependency versions; integration should reconcile the
  lockfile according to the parallel-wave dependency rule.
