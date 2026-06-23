# Strict Recovery Failure Atlas

Strict semantic reconstruction is allowed to fail. This atlas defines common
failure classes and the honest outcome for each one.

| Failure class | Exact success allowed | Expected report | User-facing wording |
| --- | --- | --- | --- |
| Invalid schema or unsupported version | No | Validation issue | Input format is unsupported |
| Non-finite or unordered bounds | No | Invalid input issue | Mesh bounds are invalid |
| Dirty clean-mesh flags | No | Clean-mesh warning or rejection | Mesh can be analyzed, but exact recovery requires clean input |
| Unknown units or non-orthogonal axes | No | Canonicalization error | Mesh cannot be canonicalized |
| Left-handed label or signed-axis parity mismatch | No | Canonicalization warning and rejection | Source frame is diagnostic-only for exact recovery |
| Missing public fingerprints | No | Fingerprint rejection | Exact recovery is not proven because required fingerprints are missing |
| Stale topology or position fingerprint | No | Fingerprint mismatch | Input does not match the known-base descriptor |
| Semantic descriptor mismatch | No | Descriptor proof failure | Semantic evidence contradicts the candidate |
| Topology mismatch | No | Signature or unexplained topology failure | Topology is outside the candidate grammar |
| Position mismatch | No | Bounds or runtime proof failure | Geometry is not exact under the candidate program |
| Missing operator capability | No | Missing capability record | The forward language cannot express the detected feature |
| Search limit reached | No | Search-limit record | Search did not prove exact recovery |
| Drifted correspondence | No | Correspondence diagnostic | A semantic landmark is too far from the candidate |
| Surplus correspondence | No | Correspondence diagnostic | More semantic support was observed than expected |
| Unexplained correspondence target | No | Correspondence diagnostic | The input contains semantic evidence outside this candidate |
| Duplicate correspondence ID | No | Invalid input issue | Correspondence evidence is not traceable |
| Literal target mesh payload | No | Admissibility failure | The result stores the target instead of explaining it |
| Dense per-vertex displacement | No | Admissibility failure | The result is a correction buffer, not semantic recovery |
| Opaque residual bytes | No | Residual diagnostic excluded from strict success | Residual data is analysis-only |
| Target-derived base topology | No | Integrity failure | Base topology must be versioned independently |
| Export-only target-index permutation | No | Audit/export adapter only | Index remapping cannot repair semantic topology |

## Report Surfaces

Use the most specific report available:

- `shape_program_verify::StrictSemanticVerification` for strict program
  acceptance or rejection.
- `shape_inverse::StrictReconstructionFailureReport` for best-program,
  unexplained-topology, unexplained-geometry, missing-capability, search-limit,
  and residual diagnostics.
- `shape_inverse::character_recovery::KnownBaseCharacterRecoveryReport` for
  known-base character recovery metrics.
- `shape_inverse::external_character::ExternalCharacterAnalysisReport` for
  external clean-character canonicalization and ranking diagnostics.

## Residual Policy

Residual diagnostics may be useful for engineering and visualization, but they
are explicitly excluded from strict success. A residual-bearing result can be
reported as partial, diagnostic, or same-topology replay, not as exact semantic
recovery.

## Recovery Boundary

The correct unsupported outcome is a useful failure report. Shape Lab should
prefer a precise failure over a fake editable import.

