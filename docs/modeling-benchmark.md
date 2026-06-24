# Modeling Benchmark

The explicit-topology benchmark assets live in `crates/shape-modeling-assets`.

Checked-in recipe JSON fixtures:

- `crates/shape-modeling-assets/assets/industrial_crate.asset.json`
- `crates/shape-modeling-assets/assets/explicit_desk_lamp.asset.json`

The Rust constructors in `shape-modeling-assets` are the source of truth used by
tests and `shape-cli model-demo`; the JSON files are serialized benchmark
recipes for inspection and external tooling. The former native Asset Modeling
Lab templates were removed from the product app in Wave 31.

## Industrial Crate

The Industrial Crate exercises deterministic part hierarchy, repeated details, mirrored components, hard and smooth boundaries, and semantic provenance.

Implemented parts and features:

- rounded-box body
- four separate feet
- semantic recessed front and back panels
- swept side handle with mirrored generated counterpart
- repeated cylinder bolt rows on front and back panels
- body-level recessed panel, ventilation slots, mounting holes, and targeted boundary bevels on the rounded crate body
- optional top ventilation slat array with rectangular through-cuts
- constrained semantic Plate and RoundedBox cuts, but no generic mesh booleans
- no SDF or remeshing path

Current `model-demo` output:

- parts: 31
- exported triangles: 5012
- budget: below 30,000 triangles

## Explicit Desk Lamp

The Explicit Desk Lamp exercises lathe and sweep generators, articulated part structure, named sockets, pivots, and clean separated meshes.

Implemented parts and features:

- lathed weighted base
- swept angled stem
- cylinder lower and upper joints
- lathed shade
- named socket and pivot definitions on base, stem, joints, and shade
- all parts separate and intentional
- no SDF or remeshing path

Current `model-demo` output:

- parts: 12
- exported triangles: 2776
- budget: below 25,000 triangles

## Stylized Stool

The Stylized Stool exercises non-crate, non-lamp part hierarchy with tapered supports and clean repeated construction.

Implemented parts and features:

- rounded seat
- four tapered legs
- foot pads
- support rails
- bevel controls
- optional seat trim
- no SDF or remeshing path

Current `model-demo` output:

- parts: 13
- exported triangles: 2140
- budget: below 25,000 triangles

## Quality Checks

The benchmark assets are covered by `shape-modeling-assets` tests and `shape-cli model-demo` output validation. The compiler validates:

- no invalid polygon indices
- no degenerate faces
- closed parts have no boundary loops
- declared open parts only use expected boundary metadata
- consistent manifold winding
- finite split normals
- face provenance for every polygon
- deterministic statistics and source recipe hash
- no SDF or remeshing usage

The generated Blender reconstruction scripts are debug verification tools only. Their colors are simple per-object debug colors and are not an asset material system.
