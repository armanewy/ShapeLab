# Wave 5 Search Quality Notes

## Implementation

- Added `generate_candidates_with_diagnostics`, returning `SearchOutput` with final candidates plus `SearchDiagnostics`.
- Kept `generate_candidates` source-compatible by routing it through the diagnostics path and returning only candidates.
- Added proposal rejection counters for empty edits, validation failures, descriptor failures, small parameter/visual/occupancy distances, and duplicate suppression.
- Added per-pass diagnostics with thresholds so callers can see when fallback passes relaxed quality gates.
- Added parameter-change summaries across returned candidates, keyed by canonical parameter path and group.
- Balanced mutation selection by parameter group before selecting within a group, so large groups and wide transform ranges do not dominate proposal generation.
- Bounded mutation deltas by group and mode. Refine remains local; Explore uses broader but capped transform and blend changes.
- Added a preservation penalty in ranking for edits near locked-node neighborhoods or descriptor bounds. The penalty affects ordering and diversity selection, not validation.
- Added duplicate suppression using exact parameter vectors, normalized Euclidean parameter-vector distance, and occupancy-bit distance.
- Added final fallback behavior for sparse descriptor cases: the last pass keeps parameter-distance and exact duplicate protection, but relaxes visual/occupancy minima so too few survivors can still yield parameter-diverse candidates.

## Tests And Benchmark

- Added diagnostics, fallback, duplicate-counter, preset-fixture diversity, mode distance, validation, and transform/blend safety tests in `shape-search`.
- Added a small `harness = false` benchmark binary at `crates/shape-search/benches/search_quality.rs`.
- The preset-spanning tests use local shape-core fixture documents instead of importing `shape-presets`, because adding a new dev-dependency would require `Cargo.lock` reconciliation outside this wave's ownership.

## Tradeoffs

- The final fallback can return candidates with zero measured occupancy distance when a sparse descriptor grid cannot distinguish enough local edits. Diagnostics expose that via final-pass thresholds and returned minimum distances.
- Duplicate suppression uses Euclidean parameter-vector distance rather than RMS distance, because RMS across many mutable parameters made distinct single-parameter edits look identical in larger documents.
