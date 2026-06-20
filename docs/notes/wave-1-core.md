# Wave 1 Core Notes

## Assumptions

- Disabled nodes remain fully stored and validated, but later evaluators can ignore them when sampling fields.
- `Difference` with an empty subtractor list is valid because it still represents its base field. Empty `Union`, `SmoothUnion`, and `Intersection` nodes are invalid.
- `descendants_of` returns reachable descendants sorted by `NodeId`, with duplicates removed. It reports an unknown node error if traversal encounters a dangling reference.
- Transform scale components with absolute value at or below `1.0e-5` are treated as near-zero and invalid.
- Parameter descriptor bounds are MVP editing bounds, not a complete validation policy. Primitive validity is still checked directly by validation.

## Contract Issues

- Prompt 1.1 requires a serde JSON round-trip test in `shape-core`, but `shape-core` did not depend on `serde_json`. I added a crate-local dev dependency on the existing workspace `serde_json` version and did not change the root workspace manifest.
- Running `cargo test -p shape-core` updates the `shape-core` package dependency list in `Cargo.lock` because of that dev dependency. The lockfile change is intentionally not part of this branch because Prompt 1.1 forbids editing `Cargo.lock`.
