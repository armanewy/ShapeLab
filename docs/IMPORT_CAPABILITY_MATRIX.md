# Import Capability Matrix

This matrix defines what Shape Lab may claim for imported, generated, and
recovered assets after Wave 20.

| Input class | Current outcome | User-facing label | Exact editable recovery | Evidence path | Boundary |
| --- | --- | --- | --- | --- | --- |
| Visual Foundry built-in profiles | Generate, customize, pack, export | Create asset family | Not an import path | `shape-foundry`, `shape-family-compile`, `shape-compile` | Product-ready for the ten authored profiles |
| Explicit asset recipes | Compile, validate, export | Modeling Workspace asset | Not an import path | `shape-modeling`, `shape-asset`, `shape-compile` | Requires authored recipe graph |
| Synthetic hard-surface benchmark meshes | Strict recovery gate | Exact semantic recovery | Yes, only when the strict gate accepts | `shape-inverse::recovery_gate` | Covered synthetic domains only |
| Synthetic known-base character corpus meshes | Strict known-base recovery gate | Exact known-base recovery | Yes, only when descriptor proof and runtime proof accept | `shape-character::corpus`, `shape-inverse::character_recovery` | Versioned known-base character grammar only |
| External clean-character descriptors | Canonicalize, rank, triage, and optionally run known-base strict proof | Analyze character import | Yes only when triage runs strict known-base recovery and it accepts | `shape-inverse::external_character`, `shape-inverse::import_triage` | Diagnostics alone are not editability |
| Same-topology source/target mesh pairs | Decompile ordered deformation package | Same-topology deformation replay | No strict semantic success when final correction is required | `shape-decompiler` | Requires identical ordered topology |
| Prepared mesh templates | Validate and customize authored known-base templates | Customize prepared template | Not an import recovery path | `shape-character::prepared` | Requires authored cages, weights, landmarks, base fingerprints, and whole-model controls |
| Arbitrary triangle mesh | Unsupported semantic editability | Diagnostic-only unsupported mesh | No | Failure report only | No general mesh-to-procedural compiler claim |
| Noisy scans or non-manifold meshes | Invalid or diagnostic-only | Analyze import failed | No | Canonicalization and clean-mesh diagnostics | Requires clean canonical input before ranking |
| UV/material/rig/animation imports | Out of scope | Unsupported data | No | None | Not part of current modeling/reconstruction scope |

## Exact Recovery Requirements

A result may be called exact editable recovery only when all of these are true:

- The recovered program passes strict semantic verification.
- Canonical positions are exact.
- Semantic topology is exact.
- Serialization order is exact.
- Residual bytes are zero.
- Literal target mesh bytes are zero.
- No per-vertex independent position payload is used.
- All operations are admissible under the strict policy.
- Base topology is versioned and independently fingerprinted, not target-derived.

## External Clean-Character Triage

External clean-character analysis can report:

- `ExactKnownBaseEligible`: public descriptor evidence is strong enough for a
  later strict known-base recovery attempt.
- `PartialKnownBaseDiagnostic`: a known-base candidate is plausible, but exact
  recovery is not proven.
- `DiagnosticOnlyUnsupported`: the input is clean enough to analyze, but no
  candidate reaches the reporting threshold.
- `InvalidInput`: schema, geometry, units, axes, config, or correspondences
  failed validation.

Eligibility is blocked by missing or stale fingerprints, dirty clean-mesh flags,
left-handed or axis-parity-inconsistent canonicalization, drifted
correspondences, surplus correspondence evidence, unexplained targets, invalid
config, duplicate correspondence IDs, and hidden JSON fields.
Dirty clean-mesh flags are warnings and exact-recovery rejections, not invalid
input by themselves.

Wave 24 adds `shape-cli import-triage-character`, which writes a product-facing
triage report. It may use the label "Recover exact editable program" only when
the composed known-base strict recovery report accepts. Partial and unsupported
results remain diagnostic reports.

Closest Foundry-family suggestions remain future work. They must not be emitted
without a concrete family ID, evidence, and validation path.

## Product Copy Rules

Use these labels:

- "Create with Visual Foundry" for authored family generation.
- "Customize prepared template" for authored prepared assets with cages,
  weights, landmarks, and base fingerprints.
- "Recover exact editable program" only after strict verification accepts.
- "Analyze import" for external clean-character triage.
- "Diagnostic-only unsupported mesh" for arbitrary or noisy meshes.

Avoid these labels:

- "Edit any mesh"
- "General mesh import"
- "Automatic proceduralization"
- "Editable import" when strict verification has not accepted
