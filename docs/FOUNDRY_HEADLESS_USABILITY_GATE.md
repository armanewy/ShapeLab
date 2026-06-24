# Foundry Headless Usability Gate

Wave 7 added a deterministic visual-foundry benchmark for the original three
built-in profiles:

```text
shape-cli foundry-visual-benchmark --profile roman-bridge --out-dir target/foundry-benchmark/roman-bridge
shape-cli foundry-visual-benchmark --profile sci-fi-crate --out-dir target/foundry-benchmark/sci-fi-crate
shape-cli foundry-visual-benchmark --profile stylized-lamp --out-dir target/foundry-benchmark/stylized-lamp
```

Wave 26 keeps the same benchmark entry point and expands accepted profile slugs
to `roman-bridge`, `sci-fi-crate`, `stylized-lamp`, `market-stall`,
`sci-fi-door`, `storage-barrel`, `signpost`, `workshop-chair`, `handcart`, and
`stylized-tree`. The compact `scifi-crate` spelling remains accepted as a
compatibility alias.

A benchmark run writes source documents, catalog locks, the customizer profile,
parent previews, Refine and Explore contact sheets, auxiliary silhouette,
structure, and detail sheets, control strips, option galleries, validation,
metrics, and one coherent three-member pack for the selected profile.

The automated full-artifact smoke test remains intentionally limited to the
original Roman bridge profile because it renders previews and exports a pack.
Wave 26 expansion profiles are covered by catalog compile/model validation,
typed authoring, six-card Explore generation, six valid topology-preserving
Refine supplement samples, coherent three-member pack compilation, and the
cheap visual-benchmark slug/fixture mapping test.

## Gate Verdict

Pass for the headless milestone. Native UI integration may begin after this
gate because the customizer profiles can be understood and exercised with zero
technical surface exposure.

## Manual Run Evidence

The Wave 7 branch was run locally against the original three built-in profiles in
`target/foundry-benchmark` with Blender 4.5 available:

| Profile | Refine | Explore | Primary controls | Provider options | Pack members | Blender reopen |
| --- | ---: | ---: | ---: | ---: | ---: | --- |
| `roman-bridge` | 6 | 6 | 7 | 9 / 9 | 3 | passed |
| `sci-fi-crate` | 6 | 6 | 7 | 0 / 0 | 3 | passed |
| `stylized-lamp` | 6 | 6 | 7 | 0 / 0 | 3 | passed |

All three runs reported measurable primary controls, no invalid current state,
coherent three-member pack export, and `verify_reopen: true` in
`parent/blender-verification.json`.

## Questions

- Control understandability: Pass. Every primary visible control gets a
  control strip with the unchanged parent, sampled outputs, labels, topology
  behavior, geometry fingerprints, and measured image deltas.
- Option gallery usefulness: Pass. Choice and provider galleries render every
  authored option that compiles and validates. Provider coverage is reported as
  rendered versus total in `metrics.json`.
- Candidate meaningfulness: Pass. Refine and Explore must each return six
  valid candidates. Each card includes changed controls, changed-role metadata,
  visual delta, mesh summary, recipe and artifact fingerprints, and structured
  explanations.
- Explanation accuracy: Pass for the current foundry command vocabulary.
  Explanations come from the same control-delta diagnostics used by candidate
  generation, and provider changes are cross-checked against effective role
  selections.
- Invalid primary controls: Pass. The benchmark rejects a profile if any
  primary visible control cannot resolve a current value, if a current state
  fails compile/model validation, or if a primary control produces no valid
  visual sample.
- Coherent pack generation: Pass. The benchmark exports a pack containing the
  parent, first Refine survivor, and first Explore survivor, then verifies every
  member package.
- Async latency acceptability: Pass for the headless proof. The command records
  deterministic usability metrics and keeps preview compilation batched and
  bounded. Native UI integration should still run this work off the UI thread.
- Technical recipe surface required: No. The benchmark operates entirely through
  authored foundry controls, provider choices, candidate search, and pack export.

## Determinism

The benchmark uses fixed profile-specific seeds, deterministic JSON ordering
where the underlying data requires maps, no wall-clock timing in generated
metrics, and package-relative Blender output paths. Re-running the same command
to a clean output directory is expected to reproduce the same candidate records,
preview sheets, validation reports, and pack reports.

## Blender Verification

The parent package is always verified with the canonical package verifier. When
Blender is discoverable via `--blender-exe`, `SHAPE_LAB_BLENDER_EXE`, or the
default Windows Blender 4.5 installation path, the benchmark also runs the
generated `blender_reconstruct.py` script with `--verify-reopen` and records the
runtime report in `parent/blender-verification.json`. Use `--skip-blender` for
CI or machines without Blender.
