# Import Triage Workflow

Wave 24 exposes import triage as a diagnostic workflow, not as arbitrary mesh
editing. The first supported lane is an external clean-character descriptor.

## Command

```powershell
cargo run -p shape-cli -- import-triage-character external-character.json --out-dir triage-out
```

The command writes:

- `import-triage-report.json`
- `external-character-analysis.json`
- `strict-known-base-recovery.json`

`strict-known-base-recovery.json` is `null` unless the descriptor has enough
evidence to run the known-base strict recovery gate.

## Outcomes

- `exact_editable_recovery`: strict known-base verification accepted. This is
  the only triage outcome allowed to use "Recover exact editable program".
- `known_base_partial_diagnostic`: a known-base candidate was found, but exact
  editability is not proven.
- `diagnostic_only_unsupported`: the input can be analyzed, but current
  grammars do not support recovery.
- `invalid_input`: schema, units, axes, bounds, config, or correspondence
  evidence failed validation.

## Boundary

External clean-character analysis ranks candidates and explains rejections.
Import triage may compose that analysis with strict known-base recovery when
the descriptor is exactly eligible. Missing fingerprints, stale fingerprints,
left-handed frames, dirty clean-mesh flags, drifted correspondences, surplus
correspondences, unexplained targets, invalid units, invalid bounds, and hidden
JSON fields keep the result out of exact editable recovery. Dirty clean-mesh
flags are warnings and exact-recovery rejections, not invalid-input errors by
themselves.

Closest Foundry-family suggestions are not emitted by the current
clean-character triage path. A future suggestion report must include an explicit
family ID, evidence, and validation path before any "open template" action can
be marked available.

The workflow does not import arbitrary triangle meshes, create prepared cages,
infer materials, rig characters, or edit DCC scene files. Unsupported inputs get
a report that says what failed and what evidence would be needed next.
