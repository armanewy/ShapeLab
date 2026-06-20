# Experimental Schema 3 Decompiler Transport

Schema 3 is an experimental package transport for same-topology deformation decompiles. It is opt-in through:

```bash
cargo run -p shape-cli -- decompile source.obj target.obj \
  --package-schema 3 \
  --out-dir target/decompile-v3
```

Enable the experimental bend search with:

```bash
cargo run -p shape-cli -- decompile source.obj target.obj \
  --package-schema 3 \
  --enable-bend \
  --verbose \
  --out-dir target/bend-decompile
```

Schema 2 remains the default and keeps its existing manifest shape and numeric conventions. Schema 3 reuses the same canonical affine evaluation contract, but separates semantic explanation from exact replay.

## Current Scope

Supported in this milestone:

- lossless-only programs
- one selected affine-family operator followed by lossless correction
- one selected bend operator followed by lossless correction
- affine then bend, and bend then affine, followed by lossless correction
- translation, rigid transform, similarity transform, and general affine metadata
- uniform-curvature bend metadata: origin, longitudinal axis, bend direction, signed angle, and interval
- diagnostics schema 4
- schema-3 Blender reconstruction and saved-file verification script

Not enabled yet:

- topology-changing or correspondence-solving operations
- falloff masks, multiple bends, arbitrary handles, twist, FFDs, or native DCC deformers

A no-op inference result serializes no explanatory operators. The package still contains one terminal `lossless_correction` operator with a cumulative baked stage.

## Replay Contract

Every schema-3 package operator declares a `stage`:

- `stage_index`
- `operator_id`
- `label`
- `baked_positions_file`
- semantic verification policy and report

Semantic operators are validation metadata. Exact replay advances using the cumulative baked positions for each stage, then verifies that the terminal lossless correction reconstructs the target bit-exactly.

The required operator order is:

```text
zero or more explanatory operators
one terminal lossless_correction
```

The current CLI emits either:

```text
lossless_correction
```

or:

```text
affine
lossless_correction
```

With `--enable-bend`, the current CLI may emit:

```text
bend
lossless_correction
```

```text
affine
bend
lossless_correction
```

```text
bend
affine
lossless_correction
```

Stage payloads use:

```text
operators/{index:04}-{stable-slug}-positions.f32
```

The correction payloads are:

```text
residual/indices.u32
residual/positions.f32
```

Correction positions are absolute target positions.

## Package Files

A schema-3 package contains:

```text
manifest.json
package-verification.json
inference-diagnostics.json
source.meshbin
target.meshbin
operators/
residual/indices.u32
residual/positions.f32
blender_reconstruct.py
```

Unlike schema 2, schema 3 does not write `verification.json`; replay verification is represented by `package-verification.json` and the optional `package_verification` object embedded in `manifest.json`.

## Verification

`shape-cli verify-decompile` reads `manifest.json` and dispatches schema 2 or schema 3 automatically. Unknown schema versions are rejected.

Schema-3 verification checks:

- source and target meshbin topology match exactly
- topology counts match the manifest
- all declared package paths stay inside the package root
- operator IDs are non-empty and unique
- the lossless correction is terminal
- every declared baked stage exists
- affine semantic evaluation matches its baked stage bit-exactly
- bend semantic evaluation matches its baked stage within the declared bend tolerance policy
- lossless correction residual indices are strictly increasing
- final positions match the target by `f32` bits

Older schema-2 packages without a lossless baked-stage declaration still verify through the schema-2 path.

## Blender Round Trip

Create and verify a schema-3 `.blend` file:

```bash
blender --background \
  --python target/decompile-v3/blender_reconstruct.py \
  --
```

Reopen the saved file and verify persisted objects and shape keys:

```bash
blender --background target/decompile-v3/reconstructed.blend \
  --python target/decompile-v3/blender_reconstruct.py \
  -- --verify-existing --report blender-roundtrip-verification.json
```

Both reports must have `verification_passed: true`. The editable object must preserve the Basis shape key, one cumulative shape key for each package stage, exact vertex IDs, schema metadata, and the final shape-key value of one.

## Diagnostics

Schema 3 writes `inference-diagnostics.json` with diagnostics schema 4. The report records:

- package schema version
- scoring policy
- every considered ordered program hypothesis
- selected ordered program hypothesis
- actual affine and bend parameters
- per-stage weighted and raw errors
- semantic-to-baked verification metrics
- terminal correction counts and exact residual bytes
- recomputable score components
- rejection reasons for non-selected hypotheses
- advisory timing buckets by phase

No `no_op` explanatory operator is serialized in diagnostics or package manifests.

## Integration Interfaces

These interfaces are the current handoff points for ordered program work:

- `shape_decompiler::v3::inference::generate_affine_candidates`
- `shape_decompiler::v3::inference::generate_bend_candidates`
- `shape_decompiler::v3::inference::search_programs`
- `shape_decompiler::v3::inference::search_programs_for_mesh_pair`
- `shape_decompiler::v3::package::build_v3_package_from_program_with_diagnostics`

`search_programs_for_mesh_pair` uses source topology to derive surface weights and optionally admits bend candidates. `build_v3_package_from_program_with_diagnostics` appends the terminal lossless correction, writes cumulative baked stage payloads, preserves the full schema-4 search report, writes the schema-3 Blender adapter, and replay-verifies before publishing.

## Current Search Limits

The bend search is intentionally shallow. It can recover bend-only, negative bend, partial interval bend, uneven tessellation, large coordinate offsets, local edits after a bend, affine-only cases, and scale variants in the generated corpus. Some pre- and post-affine bend compositions are deterministic but ambiguous: the selected program can be an affine-plus-corrective-bend approximation rather than the exact generating sequence. The final lossless correction remains exact in all cases.
