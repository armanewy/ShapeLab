# Model Export Notes

## Scope

This branch adds deterministic export helpers for compiled explicit asset
artifacts in `shape-compile::export`. The work is additive to the explicit
asset path and does not change implicit editor behavior, schema-2 decompiler
packages, schema-3 bend packages, or model validation policy.

## OBJ Export

- OBJ output is ordered by stable part instance ID.
- Each part instance is emitted as one `o` and one matching `g` block.
- Split normals are emitted as `vn` records and faces reference `v//vn`.
- Materials and MTL sidecars are intentionally not written.
- Part definition IDs, instance IDs, region names, and exact OBJ counts are
  emitted as deterministic comments and report data.
- Provenance is exposed as a JSON sidecar string next to the OBJ payload.

## Canonical Package

The internal package is a directory with fixed file names:

- `asset-manifest.json`
- `parts/<part-id>.meshbin`
- `provenance.json`
- `validation.json`
- `recipe.json`
- `blender_reconstruct.py`

Each part meshbin stores the source definition and instance IDs, optional
prototype and generating operation IDs, polygon positions, stable vertex and
face element IDs, polygon face counts, flattened polygon indices, one
split-normal entry per polygon loop, and face region IDs. Numeric payloads use
little-endian `f32`, `u32`, and `u64` values.

`asset-manifest.json` records deterministic part order, exact aggregate counts,
part-local checksums, region names, parent instance IDs, and recipe pivot
origins. `validation.json` records the same exact aggregate counts and carries
compile validation issues plus recipe-derived model validation issues.

## Verification

Package reads reject:

- unsupported schema or meshbin versions
- corrupted meshbin magic, truncated payloads, trailing bytes, non-finite
  numeric data, invalid face counts, and out-of-range polygon indices
- checksum mismatches between meshbins and `asset-manifest.json`
- unsafe package-relative paths, including absolute paths and `..` traversal
- sidecars whose hashes or exact counts disagree with the manifest

The package deliberately uses `asset-manifest.json` instead of the schema-2
decompiler `manifest.json`, and it does not write `source.meshbin`,
`target.meshbin`, `operators/`, or `residual/`.

## Blender Reconstruction

`blender_reconstruct.py` reads the canonical manifest and part meshbins, creates
one object per part, builds polygons from canonical arrays, applies split
normals when the runtime supports them, attaches custom semantic IDs, restores
manifest parent relationships, offsets mesh-local vertices so object origins
match recipe pivot origins, and can save plus reopen the reconstructed blend for
verification. The script includes a minimal `bpy` fallback so syntax and package
loading can be tested without a Blender installation.
