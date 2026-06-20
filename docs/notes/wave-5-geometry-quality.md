# Wave 5 Geometry Quality Notes

## Scope

- Added deterministic randomized integration coverage for `shape-core`, `shape-field`, and `shape-mesh`.
- Random document generators use bounded SplitMix-style seeds and acyclic node construction by `NodeId` order.
- Coverage includes valid primitive parameter ranges, bounded DAGs, serde round trips, scalar get/set round trips, field sampling NaN checks, conservative bounds checks for sampled negative regions, mesh topology validation, representative outward winding, deterministic OBJ output, malformed document rejection, unsafe sampling/meshing settings, and several hundred seeded no-panic sweeps.

## Benchmarks

- `shape-field` now has Criterion benches for representative graph-size field sampling and 16^3 grid descriptor sampling through `sample_grid`.
- `shape-mesh` now has Criterion benches for deterministic 36^3 candidate meshing and 56^3 current meshing.
- Benchmarks are deterministic fixtures only and make no hardware-independent performance claims.

## Dependency Notes

- Criterion was already declared in workspace dependencies. To compile owned Criterion bench targets, `crates/shape-field/Cargo.toml` and `crates/shape-mesh/Cargo.toml` need crate-local dev-dependencies and bench target registration.
- Running the bench compile updates `Cargo.lock` with Criterion transitive packages locally. That lockfile change is intentionally not part of this branch because `Cargo.lock` is outside Wave 5.1 ownership; integration should reconcile it.
- No production source changes were made.
