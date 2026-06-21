# Basic Explicit Topology Generators

Wave 1 Prompt 1.3 adds deterministic polygon generators for the modeling crate:

- `RoundedBox`
- `Cylinder`
- `Frustum`
- `Plate`

The implementation lives in `crates/shape-modeling/src/generators/basic.rs`. The
crate still accepts the schema-1 `GeometrySource` variants, then maps them onto
richer generator parameter structs for direct testing and future schema growth.

## Topology

All generators emit indexed `PolygonMesh` values. Vertex and face IDs are
assigned in construction order and are deterministic for a fixed topology. The
mesh topology signature is derived from the polygon face/index layout, so scalar
changes such as radius or thickness preserve it, while segment/subdivision
changes alter it.

No SDF, voxel, marching, or post-remesh path is used.

## Regions

Each generator emits stable local semantic regions:

- Rounded box: `primary_faces`, `bevel_bands`, `corners`
- Cylinder/frustum: `side`, `top_cap`, `bottom_cap`, `top_bevel`,
  `bottom_bevel`
- Plate: `front`, `back`, `side`, `bevel`

Region IDs are local to each generated part and stay stable under scalar
parameter changes.

## Sockets

Rounded boxes emit six face-center sockets. Cylinders and frusta emit top,
bottom, and axis-midpoint sockets. Plates emit front and back center sockets.
Socket frame `z_axis` points along the socket normal.

## Boundaries And Metadata

Closed modes are manifold with two incident faces per edge. Open modes such as
rounded-box face masks and cylinder/frustum cap modes intentionally expose open
boundary edges. Edge metadata records open, hard, or smooth classification and
region transitions.

