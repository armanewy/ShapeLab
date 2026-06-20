# Bend Inference Report

This branch delivers an experimental schema-3 vertical slice for same-topology bend decompilation.

## Delivered

- CLI flag: `--enable-bend`, accepted only with `--package-schema 3`.
- Search families: `[]`, `[Affine]`, `[Bend]`, `[Affine, Bend]`, `[Bend, Affine]`.
- Terminal correction: every selected program ends with an exact lossless correction using absolute target positions.
- Diagnostics schema 4: every considered program, selected program, actual affine and bend parameters, per-stage errors, final score components, correction size, rejection reasons, scoring policy, and advisory timing buckets.
- Schema-3 package replay: ordered topology, every baked cumulative stage, semantic affine stages, semantic bend stages, terminal correction, and final `f32` position bits.
- Blender schema-3 script: generic affine/bend/lossless stage replay, cumulative shape keys, save, reopen, and `--verify-existing` support.

## Generated Corpus

The deterministic generated corpus is implemented in `crates/shape-decompiler/tests/bend_inference_vertical_slice.rs`. It covers:

1. bend only
2. negative bend
3. partial interval bend
4. translation then bend
5. rigid transform then bend
6. similarity then bend
7. bend then translation
8. bend then rigid transform
9. bend plus localized edit
10. affine-only case
11. local-edit-only case
12. uneven tessellation
13. large coordinate offset
14. scales `1e-3`, `1`, and `1e3`

Strict cases assert the expected explanatory family sequence, bend axis and angle within 2 degrees, interval endpoints within 5% of longitudinal extent, weighted explained fraction at least 0.95, and exact final reconstruction. Ambiguous affine/bend compositions assert deterministic selection and retain competing program scores without test-only scoring constants.

## Robustness

The test suite rejects malformed bend frames, zero intervals, angle magnitudes above the supported range, corrupted bend baked stages, missing stage payloads, unknown schema-3 operators, non-terminal lossless correction, reordered or malformed stage metadata, and corrupted correction payloads.

## Scoring Policy

Schema-4 scoring now weights normalized geometric error strongly enough that a materially better bend explanation beats a cheaper affine approximation. Approximate residual coverage remains a penalty, but it no longer dominates parameter recovery for partial interval bends. The bend family prior is intentionally small so obvious bend geometry is not suppressed.

## Limitations

The search is shallow and deterministic. It does not infer multiple bends, masks, arbitrary handles, topology changes, or correspondence. Some pre- and post-affine bend compositions are ambiguous because affine candidates are fit against the final target before the bend is known; these cases still replay exactly through the lossless correction, but the selected explanatory parameters may be an approximation rather than the generating sequence.
