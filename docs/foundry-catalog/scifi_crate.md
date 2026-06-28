# Sci-Fi Crate Foundry Profile

The sci-fi crate profile is a headless foundry catalog entry for a hard-surface equipment crate. The HQ authoring pass makes the crate read as an untextured clay sci-fi prop with broad silhouette changes, strong panel relief, physically plausible front-mounted handles, visible vents, edge trim, and fasteners that read at card size.

## Primary Controls

- Body Proportions: maps to body width and height/depth so compact and broad/tall crates are visibly different.
- Structural Heft: maps to front/back body mass while keeping mounted details validation-clean.
- Panel Depth: maps to both recessed panel cuts for visible clay-preview relief.
- Panel Spacing: secondary, non-primary panel control that moves the paired recessed panels apart or together; it exists to strengthen panel-visible variation but is not exposed as a primary app control.
- Vent Density: sparse, standard, and dense body providers use different authored slot counts, sizes, spacing, placement, and rim widths.
- Handle Style: flush inset grip, side rail handles with brackets, or cargo bar with mounts.
- Edge Softness: maps to rounded-box radius for subtle but measurable edge treatment.
- Detail Density: maps to a 4-16 fastener repetition range for low, medium, and high detail reads.

Hidden metadata controls keep trim presence, runtime wear, and advisory weathering available without exceeding the seven-control primary surface.

## Authored Geometry

- Rounded-box body with cut-compatible front face and stronger compact versus broad/tall endpoints.
- Semantic cut groups for recessed panels, vent slots, and mounting holes.
- Boundary-loop bevel operations on panel and mounting-hole cut edges.
- Raised front access plate, edge trim strips, visible fastener row, and authored handle assemblies laid out with validation-clean gaps.
- Vent providers: two large sparse framed slots, three standard slots, and six smaller dense slots in a two-row bank.
- Handle providers: flush inset grip with cheeks, paired side rails with bracket bars, and cargo bar with mounts that sit near the front body shell without intersecting other details.
- Optional trim is selected by hidden `has_trim` so candidates can compile with or without trim.

## Candidate Strategies

- Compact Vented: compact body, stronger panel/read changes, dense vents, visible handle, medium detail, and crisper edges.
- Reinforced Cargo: heavier and broader frame, deeper/spaced panel relief, standard vents, cargo-bar handle, high fasteners, and softened edges.
- Clean Lab Crate: smoother body, shallower/spaced panels, sparse vents, flush handle, low detail, and trim optional.
- Heavy Utility: broad/heavy massing, panel relief, cargo handling, visible vents, high detail, and heavier edge treatment.
- Deep Panel Equipment: panel depth and spacing changes combined with body, vent, handle, edge, and detail changes.
- Minimal Industrial: compact/simple body, shallow/tighter panels, sparse vents, flush handle, low detail, and trim optional.

The catalog crate does not depend on `shape-search` at runtime. Tests cover strategy-style foundry states, handle attachment, structural vent differences, panel-depth and panel-spacing descriptors, detail-density mesh growth, candidate survival, TooSubtle rejection from returned whole-asset ideas, locks, export/package verification, conformance, and zero accidental intersections.

## Focused Part Capability

- Body: candidate-ready for focused shape ideas through Body Proportions and Structural Heft.
- Panels: inspection-only in this build. Panel Depth and Panel Spacing visibly change the crate, but focused panel search currently collapses to one surviving idea, so Panels are not reported as candidate-ready.
- Vents: focusable for inspection but not candidate-ready. Vents can be adjusted through Vent Density, but focused vent ideas are limited in this build.
- Handles: inspection-only in this build. Handle Style swaps authored attached assemblies, but focused handle search currently collapses to one surviving idea, so Handles are not reported as candidate-ready.
- Edge Trim: not candidate-ready. Edge softness is shared with the body and trim presence is hidden, so focused trim ideas are not isolated.
- Fasteners: not candidate-ready. Detail Density changes fastener count, but focused fastener ideas are detail-only and are not exposed as shape-focused candidates.

## HQ Evidence

Run the visual benchmark from the repository root:

```bash
cargo run -p shape-cli -- foundry-visual-benchmark --profile sci-fi-crate --proposal-count 72 --out-dir target/foundry-benchmark/scifi-crate-hq --skip-blender
```

Expected generated evidence includes:

- `target/foundry-benchmark/scifi-crate-hq/parent/preview.png`
- `target/foundry-benchmark/scifi-crate-hq/explore/contact-sheet.png`
- `target/foundry-benchmark/scifi-crate-hq/control-strips/summary.json`
- `target/foundry-benchmark/scifi-crate-hq/option-galleries/summary.json`
- `target/foundry-benchmark/scifi-crate-hq/metrics.json`

Do not commit generated PNGs or JSON unless a later integration prompt asks for binary evidence in source control.
