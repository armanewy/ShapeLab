# Explicit Bevel Quality

Wave 3 Prompt 3.2 adds `shape_modeling::bevel` as the explicit bevel capability
layer for the modeling crate. The layer validates `SetBevelProfile` against the
source family before generation and does not claim arbitrary mesh-wide bevel
support.

Supported explicit sources are:

- `RoundedBox` edge and corner bands
- `Plate` front/back perimeter bevels
- `Cylinder` cap rims
- `Frustum` cap rims
- `Sweep` closed profile corners before sweep generation

`SetBevelProfile` currently carries schema-2 `radius` and `segments` fields. The
capability model also names profile exponent, adjacent-normal hardening, and
affected semantic edge classes so later schema growth has a typed destination
without changing the mesh contract.

Positive bevels on unsupported topology return `UnsupportedOperation`. Excessive
widths are rejected before generator dispatch so bevel bands do not overlap and
remaining faces do not collapse. A zero radius is treated as disabled, including
when the source would otherwise be unsupported.

## Shading

`compute_weighted_split_normal_groups` and `triangulate_with_weighted_normals`
compute area-weighted split normals for preview shading while keeping existing
`shape-poly` geometric face normals and split-normal helpers unchanged for
verification. Hard edge metadata, feature boundaries, seams, and open boundaries
continue to split normals; smooth bevel bands can share weighted normals across
their intended smoothing groups.

## Quality Coverage

`crates/shape-modeling/tests/bevel_quality.rs` covers rounded-box bevel widths,
plate corner/perimeter bevels, cylinder cap bevels, sweep profile corner bevels,
excessive-width rejection, zero-bevel disablement, normal continuity, hard
boundary preservation, deterministic weighted triangulation, and degenerate
triangle checks.
