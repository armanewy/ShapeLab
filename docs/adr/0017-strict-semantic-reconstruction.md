# ADR 0017: Strict Semantic Reconstruction

## Status

Accepted.

## Context

Shape Lab can already replay ordered geometry programs and export verified assets. The next modeling lane needs a stricter contract so inverse reconstruction cannot be treated as successful by attaching residual vertex data to a weak semantic program.

The product goal is editable semantic reconstruction. A program that stores the target mesh, dense displacements, or one independent parameter per vertex is not meaningfully editable, even if it reproduces the input exactly.

## Decision

Introduce three crates:

- `shape-program` for the shared semantic modeling-program IR.
- `shape-program-verify` for strict-success verification.
- `shape-inverse` for inverse failure diagnostics.

Strict success requires exact canonical positions, exact semantic topology, exact serialization order, zero residual bytes, zero literal target mesh bytes, zero per-vertex independent position parameters, admissible operations, sufficient compression, and perturbation-validity evidence.

Target-index permutations are allowed only as audit/export adapters. They are reported separately and do not count as semantic explanation.

## Consequences

Strict search may fail. That is an expected and acceptable outcome.

Failure reports must preserve useful work: best semantic program, unexplained topology regions, unexplained geometry, missing operator capabilities, search limits, and residual diagnostics. Residual diagnostics are allowed for analysis, but they are explicitly excluded from strict success.

Forward modeling and inverse reconstruction now share one operation-program contract, so later operators must be designed for deterministic replay, compact selections, provenance, and admissibility from the start.
