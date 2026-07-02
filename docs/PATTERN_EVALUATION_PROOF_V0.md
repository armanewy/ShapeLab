# PatternContract Evaluation Proof v0

Status: Wave 2 implementation slice.

This branch adds the first deterministic PatternContract evaluator. It remains
internal infrastructure and does not expose pattern UI, pattern handles, export
instancing, materials, collision, motion, terrain, or game-ready claims.

## Supported V0 Scope

Supported:

- `PatternType::Linear`
- finite non-negative spacing
- count from `count` or `PatternCountPolicy`
- axes `X`, `Y`, and `Z`
- deterministic occurrence IDs from pattern ID plus occurrence index

Unsupported:

- radial, grid, mirror, along-curve, on-surface, and scatter evaluation
- product-visible pattern controls
- export instancing claims
- material, collision, motion, terrain, rigging, or animation behavior

## Reports

`PatternEvaluationReport` records:

- source pattern ID
- generated occurrence count
- deterministic occurrence IDs
- copied export instancing policy
- `export_instancing_enabled: false`

Compile-layer pattern evaluation records blockers instead of crashing when a
pattern is invalid or unsupported.
