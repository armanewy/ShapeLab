# Sci-Fi Crate Foundry Profile

The sci-fi crate profile is a headless foundry catalog entry for a hard-surface equipment crate. The HQ authoring pass makes the crate read as an untextured clay sci-fi prop with broad silhouette changes, strong panel relief, physically plausible front-mounted handles, visible vents, edge trim, and fasteners that read at card size.

## Primary Controls

- Body Proportions: maps to body width and depth so compact and broad crates are visibly different.
- Structural Heft: maps to front/back body mass while keeping mounted details validation-clean.
- Panel Depth: maps to deeper recessed panel cuts for visible clay-preview relief.
- Vent Density: sparse, standard, and dense body providers use different authored slot counts, sizes, and spacing.
- Handle Style: flush inset grip, side rail handles with brackets, or cargo bar with mounts.
- Edge Softness: maps to rounded-box radius for subtle but measurable edge treatment.
- Detail Density: maps to fastener repetition count for low, medium, and high detail reads.

Hidden metadata controls keep trim presence, runtime wear, and advisory weathering available without exceeding the seven-control primary surface.

## Authored Geometry

- Rounded-box body with cut-compatible front face and stronger body endpoints.
- Semantic cut groups for recessed panels, vent slots, and mounting holes.
- Boundary-loop bevel operations on panel and mounting-hole cut edges.
- Raised front access plate, edge trim strips, visible fastener row, and authored handle assemblies.
- Vent providers: two large sparse slots, three standard slots, and five smaller dense slots.
- Handle providers: flush inset grip with cheeks, paired side rails with bracket bars, and cargo bar with mounts.
- Optional trim is selected by hidden `has_trim` so candidates can compile with or without trim.

## Candidate Strategies

- Compact Vented: compact body, dense vents, visible handle, medium detail.
- Reinforced Cargo: heavier frame, deeper panel relief, cargo-bar handle, high fasteners.
- Clean Lab Crate: smoother body, sparse vents, flush handle, low detail, trim optional.
- Heavy Utility: broad/heavy massing, cargo handling, high detail.
- Deep Panel Equipment: strong panel depth, dense vents, softened edges, higher detail.
- Minimal Industrial: compact/simple body, shallow panels, flush handle, low detail, trim optional.

The catalog crate does not depend on `shape-search` at runtime. Tests cover strategy-style foundry states, endpoint descriptor differences, candidate survival, locks, conformance, and zero accidental intersections.

## HQ Evidence

Run the visual benchmark from the repository root:

```bash
$env:CARGO_TARGET_DIR='C:\Users\aoztu\Documents\Shape Lab\target'
cargo run -p shape-cli -- foundry-visual-benchmark --profile sci-fi-crate --proposal-count 72 --out-dir target/foundry-benchmark/scifi-crate-hq --skip-blender
```

Do not commit generated PNGs or JSON unless a later integration prompt asks for binary evidence in source control.
