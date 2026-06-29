# Simple Crate Primitive v0 Report

## Result

Simple Crate adds a small primitive crate family to the foundry catalog. It uses the existing family/style/profile/compiler pipeline and keeps the asset to plain clay geometry: rectangular body, raised lid, lid seam, soft edges, one trim band, and feet or skids.

## Dogfood Answers

- Can a user tell what changed? Yes. Proportions, lid height, trim thickness, and feet style produce clear whole-model changes; edge softness changes the rounded body/lid silhouette.
- Does the asset read in pure clay? Yes. The crate is readable from body mass, raised lid, seam rails, trim band, and underside support geometry without color or texture semantics.
- Is it faster and simpler than Sci-Fi Crate? Yes. It has five controls, no vent/handle/fastener surface noise, and fewer roles.
- Does any variant look broken? The automated gate checks candidates for validation, export conformance, and visibly disconnected parts. Broken variants are not accepted.

## Controls

- Proportions
- Lid Height
- Edge Softness
- Trim Thickness
- Feet Style

The implementation uses discrete choices for topology-changing controls and continuous scalar bindings only for dimensions that remain within the primitive crate shape.

## Candidate Strategies

- Compact Box
- Wide Storage Crate
- Tall Supply Crate
- Low Flat Crate
- Reinforced Simple Crate
- Clean Minimal Crate

The candidate test requires at least four distinct whole-asset ideas and rejects TooSubtle results.

## Evidence

Expected branch evidence directory:

`target/simple-crate-primitive-v0/`

Files:

- `parent.png`
- `candidate-contact-sheet.png`
- `control-endpoint-sheet.png`
- `dogfood-summary.json`

Speed targets recorded in `dogfood-summary.json`:

- first visible model under 2s
- preview ready under 4s
- first candidate skeleton tray under 500ms
- first selectable candidate under 8s

Measured in this worktree:

- first visible model: 5102ms, missed the 2s target
- preview ready: 112ms, passed
- first candidate skeleton tray: 0ms, passed
- first selectable candidate: 8111ms, missed the 8s target by 111ms
- full six-card candidate evidence batch: 25238ms

The visible-model and first-selectable misses are compile-bound in the debug-mode headless Foundry build. The full evidence batch is slower because it compiles a 72-proposal candidate set plus endpoint variants for contact-sheet output; it is not the first selectable candidate path.

## Boundaries

This primitive family does not add browser code, servers, Blender or other DCC integration, runtime LLM behavior, humanoid concepts, imported mesh editing, rigging, animation, UV unwrapping, texturing, GPU compute, adaptive octrees, dual contouring, cloud collaboration, plugin systems, or structural candidate mutations.
