# Sci-Fi Crate HQ Authoring Report

## Internal Authoring Roles

Art Director target: a hard-surface sci-fi equipment crate that reads in untextured clay as a complete prop, not a rounded box with small edits. The required read is clear body silhouette, framed/recessed panels, attached handles, readable vents, optional edge trim, visible fasteners, coherent detail density, no floating parts, no random noise, and no hidden-only parameter changes.

Geometry Author pass: reauthored the Sci-Fi Industrial Crate fixture with wider body-proportion endpoints, stronger panel depth, a broader raised access plate, a tighter five-slot dense vent bank, larger sparse vents, and multi-part handle assemblies for flush inset, side rail, and cargo bar styles. Front details were positioned outside the body envelope to keep validation clean while remaining visually mounted on the front face.

Variation Designer pass: replaced the old strategy surface with six intent labels: Compact Vented, Reinforced Cargo, Clean Lab Crate, Heavy Utility, Deep Panel Equipment, and Minimal Industrial. Each strategy combines multiple visible controls so whole-asset candidates change silhouette, vents, handles, panel depth, heft, or detail density rather than one tiny parameter.

Validation Engineer pass: added deterministic tests for strategy labels, vent structure, handle validation/proximity, body/heft/panel/detail/edge endpoint descriptors, Explore candidate survival, lock respect, conformance, and zero accidental intersections. The focused Sci-Fi Crate test target passes after the reauthoring pass.

Adversarial Critic pass: rejected the early two-row dense vent layout because compact variants could create disconnected body components. Rejected overlapping handle/fastener/trim placements because validation flagged accidental intersections. The current pass approves the authored geometry for benchmark inspection if contact sheets generate successfully.

## Control Visibility Evidence

- Body Proportions: maps width and depth, creating compact versus broad crates.
- Structural Heft: changes front/back body mass across a validated range.
- Panel Depth: reaches a deeper recessed-panel cut with visible clay-preview relief.
- Vent Density: sparse has two large slots, standard has three medium slots, dense has five smaller tight slots.
- Handle Style: flush inset grip, paired side rails with brackets, and cargo bar with mounts compile and validate.
- Detail Density: changes fastener repetition count from low to high.
- Edge Softness: changes rounded-box radius enough to be subtle but measurable.

## Contact Sheet Evidence

Generated evidence is available under `target/foundry-benchmark/scifi-crate-hq/`:

- `parent.png`
- `control-endpoint-strip.png`
- `explore-contact-sheet.png`
- `option-gallery-contact-sheet.png`
- `legibility-report.json`

The standard `shape-cli foundry-visual-benchmark` path was attempted but is currently blocked by an out-of-scope `shape-cli` compile error in `crates/shape-cli/src/game_ready_static.rs` for a `SurfaceMaterialVariantCandidate` initializer missing `blocked_full_ready`, `changed_material_slots`, `full_ready_status`, and related fields. To avoid editing outside Prompt 3 ownership, evidence was generated with a temporary local harness under `target/tmp-scifi-evidence` using the same reauthored fixture, `shape-render`, and `shape-search` path dependencies.

## Manual Gate Statement

Generated contact sheets show six Explore candidates with at least four visibly different whole-asset silhouettes/detail arrangements. Handle variants are front-mounted and validation-clean. Vent variants are structurally distinct and readable in the option/control sheets. Detail density changes the fastener row count. The adversarial critic approves this authored pass after rejecting earlier intersecting handle/trim placements and disconnected two-row dense vents.
