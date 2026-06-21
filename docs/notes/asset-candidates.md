# Asset Candidates

`shape-search::asset` generates deterministic semantic candidate programs over
`shape_asset::AssetRecipe`. The generator builds ordered `AssetEditProgram`
values with seeded ChaCha randomness and applies them through `shape-asset`, so
the source recipe is never mutated and literal mesh vertices are not edited.

Refine mode uses one or two local, topology-preserving edits. Explore mode uses
larger multi-edit programs and may include authored optional-part toggles,
compatible replacement groups, array count ranges, and detail-density changes
when topology locks permit them.

Candidate diagnostics include one entry per edit operation with the stable edit
kind, subject path, before/after summary, and whether the change can alter
topology. Generation diagnostics report available semantic edit targets,
lock-skipped targets, accepted candidates, and proposal rejection counts.
