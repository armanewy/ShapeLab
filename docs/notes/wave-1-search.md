# Wave 1 Search Notes

## Implementation

- Implemented deterministic candidate generation inside `crates/shape-search`.
- Candidate proposals are seeded with SplitMix64-derived ChaCha8 seeds from `SearchRequest.seed` and the proposal index.
- Proposal generation uses Rayon, but every proposal retains its index and final ordering is sorted deterministically before selection.
- Search mutates only scalar parameters returned by `shape_core::enumerate_parameters`.
- Parameter targeting supports selected node, selected subtree, and whole-model scopes.
- Group filters and document locks are applied before mutation.
- Edits are recorded as `EditProgram` values and applied through `shape_core::apply_edit`.
- Candidate IDs are deterministic from the proposal seed and proposal index.
- Candidate selection uses greedy max-min diversity after exact parameter-vector duplicate removal.

## Descriptor

The public `ShapeDescriptor` contract currently exposes only `values: Vec<f32>`. To keep that contract stable, the descriptor stores:

1. Packed occupancy words encoded as exact finite `u16` values in `f32`.
2. Occupied volume fraction.
3. Normalized occupied centroid.
4. Normalized occupied AABB extent.
5. Normalized occupied AABB center.

The fixed comparison domain is the parent bounds expanded by `35%` of the parent maximum extent. A proposal is rejected if its bounds escape that domain by more than an additional `15%` of the comparison-domain maximum extent.

Occupancy is sampled on `descriptor_resolution^3` points in deterministic Z/Y/X nested-loop order matching the field grid convention. Empty, nearly full, non-finite, and unchanged proposals are rejected.

## Scoring Formula

Parent distance combines geometric and parameter differences:

```text
geometric = 0.45 * occupancy_hamming
          + 0.15 * occupied_fraction_delta
          + 0.15 * centroid_distance
          + 0.15 * extent_distance
          + 0.10 * center_distance

distance_from_parent = 0.90 * geometric
                     + 0.10 * normalized_parameter_distance
```

`Refine` starts from lower parent distance and applies a small parent-distance penalty during diversity selection. `Explore` starts from higher parent distance and applies a parent-distance bonus during diversity selection.

## Contract Issues

- `shape-field` is still the Wave 0 stub in this worktree, and adding a direct crate dependency would update `Cargo.lock`, which this prompt forbids. The search crate uses a private category-independent evaluator for descriptor sampling until the real field implementation is merged and dependency reconciliation is owned by an integration wave.
- `shape-core::validate_document` in this worktree does not yet enforce every geometric invariant needed by search. The search crate performs private validation for finite transforms, positive primitive dimensions, combiner child presence, dangling references, cycles, and key primitive cross-constraints so invalid proposals are rejected locally.
- `ShapeDescriptor` has no typed fields for occupancy bits and metrics. The implementation keeps the public struct unchanged and encodes packed occupancy plus metrics in `values`.
