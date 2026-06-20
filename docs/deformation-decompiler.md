# Lossless Deformation Decompiler

The decompiler is an offline, same-topology round-trip demo. It accepts a source OBJ and a target OBJ, requires identical ordered topology, and emits a portable reconstruction package containing:

1. an editable affine stage when it explains enough of the motion
2. a final exact correction containing absolute target positions for every remaining mismatched vertex
3. canonical little-endian source and target mesh payloads
4. a versioned manifest and independent replay-verification reports
5. a self-contained Blender reconstruction and verification script

This is deliberately narrower than general inverse modeling. It proves a measurable contract first: the serialized deformation program must reconstruct the target with the same ordered triangle topology and the same canonical `f32` position bits.

The current package format is **schema 2**. Schema 2 fixes affine evaluation to left-to-right IEEE-754 binary32 arithmetic, rounding after every multiplication and addition and forbidding fused multiply-add contraction. JSON matrix numbers are normalized back to binary32 before evaluation. Packages produced by the earlier experimental schema 1 should be regenerated.

## Commands

Create a package:

```bash
cargo run -p shape-cli -- decompile source.obj target.obj \
  --out-dir target/decompile-package
```

Verify the serialized package independently of the in-memory decompile result:

```bash
cargo run -p shape-cli -- verify-decompile target/decompile-package
```

Useful creation options:

```bash
--affine-min-explained 0.01
--residual-epsilon 0.0
```

`--affine-min-explained` controls whether an affine fit is worth emitting as an editable stage. The decompiler now scores ordered program hypotheses instead of returning the first simple family that is close enough. In the current schema-2 replay manifest those programs contain at most one affine-family explanatory operator followed by the mandatory lossless correction; diagnostics schema 3 records them as ordered programs so multi-step operators can be added without changing the audit model again. Eligibility and semantic scoring use triangle-area-weighted explained displacement, while the manifest still reports the raw unweighted vertex explained fraction as a backward-compatible diagnostic. The internal score combines normalized weighted geometric error, operator parameter complexity, semantic metadata size, tolerance-based approximate residual cost, exact audit-correction bytes as a light tie-breaker, and small operator prior penalties. Approximate residual tolerance uses intrinsic object scale plus a shape-local `f32` ULP floor, so translating both meshes far from the origin does not change semantic residual coverage. This lets rigid or similarity win over translation when the simpler model would force a large meaningful correction, while still letting lossless-only reconstruction win for isolated local edits. `--residual-epsilon` affects verification reporting only; the final correction remains bit-exact.

Affine fitting uses deterministic triangle-area-derived vertex weights so densely tessellated regions do not automatically dominate the inferred geometric explanation. Exact package verification still compares every original vertex and ordered triangle index directly.

The package writer builds and verifies a sibling staging directory before replacing the requested output directory. A failed write therefore does not partially overwrite an existing valid package, and stale files from an older package are removed on successful replacement.

## Input Contract

Both OBJs must have:

- the same vertex count
- the same ordered triangle-index array, including face order and winding
- triangular faces
- finite `f32` positions
- three distinct vertex indices per triangle

The current OBJ reader accepts `v` records and triangular `f` records using exactly `v`, `v/vt`, `v//vn`, or `v/vt/vn` face elements. Positive and negative position indices are supported. Normals are recomputed on import but are not part of the decompiler contract.

There is no correspondence solver yet. Geometrically identical meshes with reordered vertices or faces are rejected.

## Package Layout

```text
manifest.json
verification.json
package-verification.json
inference-diagnostics.json
source.meshbin
target.meshbin
operators/
    0000-global-affine-positions.f32
    0001-lossless-correction-positions.f32
residual/indices.u32
residual/positions.f32
blender_reconstruct.py
```

When no affine stage is emitted, the package instead contains the lossless stage as the first operator payload:

```text
manifest.json
verification.json
package-verification.json
inference-diagnostics.json
source.meshbin
target.meshbin
operators/
    0000-lossless-correction-positions.f32
residual/indices.u32
residual/positions.f32
blender_reconstruct.py
```

The affine positions file exists only when the affine stage clears the configured threshold. The cumulative lossless stage file and correction files always exist; the correction files may be empty.

The manifest's numeric contract is:

```json
{
  "scalar": "float32",
  "endian": "little",
  "affine_evaluation": "float32_stepwise_no_fma"
}
```

Affine operators are still serialized as `kind: "global_affine"` in schema 2, so exact replay remains a matrix-plus-baked-stage contract. Newer packages may also include `semantic_family` metadata. `semantic_family: "translation"` includes a `translation` vector and is accepted only when its matrix is bit-identical to that translation matrix. `semantic_family: "rigid_transform"` includes `translation` and `rotation_row_major_3x3`; `semantic_family: "similarity_transform"` additionally includes `uniform_scale`. Rigid and similarity rotations must be proper orthonormal bases and their parameters must reconstruct the matrix bit-for-bit. `semantic_family: "general_affine"` must not declare semantic parameters.

`source.meshbin` and `target.meshbin` contain:

- magic bytes `SLMBIN01`
- little-endian `u64` vertex count
- little-endian `u64` index count
- tightly packed little-endian `f32` positions
- tightly packed little-endian `u32` triangle indices

Correction positions are absolute target positions, not accumulated deltas. This prevents drift during the final exact reconstruction.

`verification.json` must match the verification embedded in `manifest.json`. `package-verification.json` is a generated replay report. `inference-diagnostics.json` is advisory and versioned independently from the replay manifest: diagnostics schema 3 records every model-selection program hypothesis, its ordered explanatory operators, terminal lossless correction summary, weighted and raw explained fractions, normalized geometric-error cost, approximate residual coverage and score contribution, exact residual bytes and score contribution, prior penalty, total score, selection state, and rejection reason. It also serializes the scoring policy, including coefficient values, family priors, and the approximate-residual tolerance policy, so score components can be recomputed from the JSON. The `verify-decompile` command does not trust generated reports or diagnostics; it rereads the package, validates all declared formats and paths, recomputes operator metadata, replays every serialized stage, compares the ordered topology arrays directly, and checks every final position component by its `f32` bit pattern.

The manifest also contains an FNV-1a topology fingerprint for quick diagnostics. That fingerprint is not treated as proof of equality: exact verification compares vertex counts and the complete ordered index arrays.

## Exact Correction Count

A mathematically affine target can still require a few exact-correction entries. The inferred matrix is serialized as `f32`, and a least-squares coefficient that differs by one unit in the last place can produce a different final `f32` bit pattern even when the geometric error is tiny.

For this reason the CLI reports both:

- the affine stage's raw vertex explained-displacement percentage and maximum geometric error
- the number of **exact correction vertices** needed to reach bit equality

The exact-correction count should not be interpreted as the number of semantically non-affine edits by itself.

## Blender Reconstruction and Round-Trip Verification

Create the Blender file and the first report:

```bash
blender --background \
  --python target/decompile-package/blender_reconstruct.py
```

The generated script:

- validates package paths, payload sizes, fixed numeric conventions, operator ordering, and metadata
- constructs the source topology directly from canonical arrays
- adds one cumulative shape key for every inferred stage
- enables the final exact shape key
- creates a separate baked target object
- writes a persistent `shapelab_vertex_id` point attribute
- verifies the baked object and every cumulative editable shape-key stage independently
- checks that the Basis key matches the canonical source, intermediate keys match serialized stages, earlier key values remain zero, and the final key value remains one
- saves `reconstructed.blend`
- writes `blender-verification.json`

Then reopen the saved file and verify that serialization preserved both objects and the final shape key:

```bash
blender --background target/decompile-package/reconstructed.blend \
  --python target/decompile-package/blender_reconstruct.py \
  -- --verify-existing --report blender-roundtrip-verification.json
```

A Blender verification passes only when:

- face order, winding, and indices match exactly
- every target position is bit-exact at canonical `f32` precision
- persistent vertex IDs equal their array indices
- the topology fingerprint and schema properties match
- the baked object, Basis key, every intermediate cumulative shape key, and final shape key all pass

## Current Limitations

- No vertex or face correspondence solving.
- No topology-changing operation inference.
- Only affine-family explanatory operators are inferred before the exact correction. The current semantic split is translation, rigid transform, similarity transform, and general affine; bend, twist, regional operators, FFDs, and structural operators are not implemented yet.
- No Maya reconstruction adapter yet; the package itself is DCC-independent, but only Blender execution is emitted.
- UVs, normals, materials, skinning, animation, custom attributes, and object hierarchy are not reconstructed.
- The topology fingerprint is diagnostic rather than cryptographic.
- The generated Blender adapter must be tested with the Blender versions the project intends to support.
