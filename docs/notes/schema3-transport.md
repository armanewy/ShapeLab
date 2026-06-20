# Schema 3 Package Transport

Wave 1 schema-3 package transport is additive to the schema-2 package path.
The root `verify_decompile_package` dispatcher still reads schema 2 only; callers
that need schema 3 should use `shape_decompiler::v3::package`.

Implemented entry points:

- `write_decompile_package_v3`
- `read_decompile_package_v3`
- `verify_decompile_package_v3`

The writer publishes through the existing staged package directory flow. It
uses the existing meshbin, ordered position, residual index, and residual
position sidecar encodings.

Replay contract:

- source and target meshbin payloads must have identical ordered topology
- topology fingerprints are reported diagnostically and are not replay authority
- each explanatory operator consumes the previous baked cumulative positions
- affine semantic output is compared bit-exactly to its baked stage
- bend records are structurally readable, but non-identity bend replay can fail
  until bend evaluation is implemented
- every stage then advances using its baked cumulative sidecar
- one terminal lossless correction applies strictly increasing residual indices
  carrying absolute target positions
- the terminal correction must match its baked stage bit-exactly and reconstruct
  target positions bit-exactly

Stage sidecars follow `operators/{index:04}-{stable-slug}-positions.f32`.
The initial writer emits `affine`, `bend`, and `lossless-correction` slugs.
