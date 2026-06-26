# Focus Part Surface Readiness Integration

This integration keeps Focus Part modeling, headless Surface evidence, and
rig/motion readiness as separate layers.

## Merged Branches

- `codex/surface-preview-rig-motion-readiness`
- `codex/focus-part-perceptual-variation-v0`

The surface branch adds Sci-Fi Crate textured preview evidence, material-only
variants, surface delta reports, surface-aware package data, and rig/motion
readiness contracts. The Focus Part branch adds semantic part-group targeting,
focused modeling candidates, render-based visible-delta gates, and model-first
UI affordances.

## Readiness Boundaries

Shape Lab now distinguishes four states:

1. Static surface package availability:
   The Sci-Fi Crate export can emit UVs, material slots, procedural texture
   files, evidence images, and validation reports.

2. Headless surface visual evidence:
   Textured preview images, contact sheets, and material-variant delta reports
   exist outside the novice app workflow.

3. Visual Foundry Surface variation:
   Still unavailable until the app can show and compare textured
   material/surface candidates.

4. Focus Part Surface editing:
   Still unavailable until part-specific surface candidate generation and
   preview evidence exist.

Headless material-only variants do not automatically become Visual Foundry
candidates. Shape candidates cannot pass by material or surface deltas alone,
and Surface variants cannot claim shape changes.

## Product Boundary

The app may say:

- Static prop surface package available.
- Exports a frozen crate mesh with UVs, material slots, simple procedural
  texture files, evidence images, and a validation report.
- Still blocked from full game-ready status until manual review, engine import
  proof, and engine-native package handoff are complete.

The app must not claim:

- textured Visual Foundry previews;
- Surface mode readiness;
- game-ready output;
- rigging;
- skinning;
- animation;
- retargeting;
- engine-native Unity, Unreal, or Godot packages.

## Rig And Motion

Rig and motion artifacts are contracts and validation surfaces only. They remain
available to docs, schemas, package metadata, and headless validation. They are
not novice UI features and do not imply rigging, skinning, animation curve
generation, or humanoid retargeting support.

## Focus Part

Focus Part remains semantic part-group targeting, not raw mesh-part editing. v0
uses part chips and product-safe labels. True mesh picking still requires
semantic masks or hit testing in a later branch.

## Integration Rule

Headless evidence can make exports more inspectable, but it does not unlock a
novice-facing mode by itself. A Visual Foundry mode becomes available only when
the app can show the relevant candidates at product scale and validate the
visible differences without backend terminology.
