# Program Package Builder

Prompt 3.3 adds `shape_decompiler::v3::decompile`, an in-memory schema-3
builder for callers that already selected an explanatory operator program. It
does not fit, infer, reorder, or simplify operators.

The builder starts from canonical source positions, evaluates each supplied
operator against the previous baked stage, writes a cumulative in-memory baked
stage, records semantic-to-baked verification metrics, and advances exact
continuation from the baked payload. Affine stages use bit-exact verification.
Bend stages use the bend tolerance policy and record maximum and RMS error.

After explanatory operators, the builder compares final explanatory positions
to target `f32` bits, stores strictly increasing residual indices with absolute
target positions, applies the lossless correction, and requires final target
bit equality. The returned package retains the manifest, stage payloads,
residual payloads, package verification report, final positions, and schema-4
diagnostics.

Stage IDs and paths are deterministic:

- `op-0000-translation` / `operators/0000-translation-positions.f32`
- `op-0000-rigid-transform` / `operators/0000-rigid-transform-positions.f32`
- `op-0000-similarity-transform` / `operators/0000-similarity-transform-positions.f32`
- `op-0000-general-affine` / `operators/0000-general-affine-positions.f32`
- `op-0000-bend` / `operators/0000-bend-positions.f32`
- `op-0001-bend` / `operators/0001-bend-positions.f32`
- `op-0002-lossless-correction` / `operators/0002-lossless-correction-positions.f32`

Only `crates/shape-decompiler/src/v3/mod.rs` was changed outside the owned
builder files, to declare the new `decompile` module.
