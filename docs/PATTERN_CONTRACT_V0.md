# Pattern Contract v0

Date: 2026-07-01

Status: schema/contracts only.

## Goal

Pattern contracts define repetition semantics before exposing pattern UI.
Patterns are deterministic asset semantics, not random generation and not an
export-instancing claim.

## Contract Shape

`PatternContract` has:

- stable `PatternId`;
- `PatternType`;
- optional source instance;
- optional legacy/simple count;
- `PatternCountPolicy`;
- optional density policy;
- export instancing policy shell.

Pattern kinds include linear, radial, grid, mirror, along-curve, on-surface, and
scatter.

## Validation

Validation checks:

- populated source instance must exist;
- exact counts must be in the supported bounded range;
- count ranges must be ordered and bounded;
- density values must be finite and non-negative;
- density ranges must be ordered.

## Export Boundary

Pattern export instancing remains a shell. V0 does not claim engine instancing,
mesh instancing, materials, collision, motion, terrain, or game-ready output.
