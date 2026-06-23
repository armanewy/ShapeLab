# Sci-Fi Crate Foundry Profile

The sci-fi crate profile is a headless foundry catalog entry for a hard-surface equipment crate. It uses a rounded-box body provider with real front-face semantic cuts, exterior detail fragments, and seven primary customizer controls.

## Primary Controls

- Body Proportions: normalized width control mapped to rounded-box half width.
- Structural Heft: normalized body thickness control mapped to rounded-box front depth.
- Panel Depth: normalized control mapped to the primary recessed panel cut depth.
- Vent Density: sparse, standard, or dense body providers with different authored vent cut counts.
- Handle Style: flush, side-rail, or cargo-bar handle providers.
- Edge Softness: normalized rounded-box radius control.
- Detail Density: integer fastener array count.

Hidden metadata controls keep trim presence, runtime wear, and advisory weathering available without exceeding the seven-control primary surface.

## Authored Geometry

- Rounded-box body with cut-compatible front face.
- Semantic cut groups for recessed panels, vent slots, and mounting holes.
- Boundary-loop bevel operations on panel and mounting-hole cut edges.
- Separate front access plate, trim strips, fasteners, and handle providers.
- Optional trim is selected by a hidden `has_trim` control so candidates can compile with or without it.

## Candidate Strategies

- Compact Storage
- Reinforced Cargo
- Vented Equipment
- Minimal Industrial
- Hero Prop

The catalog crate does not depend on `shape-search`, so the integration test covers six strategy-style foundry states through the current foundry compile APIs. Each state compiles, passes conformance, and validates with zero accidental intersections.

## Implementation Note

Real cut operations in remapped recipe fragments require reused source region IDs for the target face and surviving outer face. The remap collector now records each source region once so these authored cut fragments can survive foundry remapping and still reach the modeling generator with the fixed primary face region it expects.
