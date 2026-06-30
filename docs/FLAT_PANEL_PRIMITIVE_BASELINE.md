# Flat Panel Primitive Baseline

Date: 2026-06-30

## Verdict

Flat Panel Primitive is the second app-visible primitive baseline after the box
profiles. It proves a non-box kernel in the same `Choose -> Make -> Pack ->
Export` loop without claiming Door behavior.

## Product Truth

- Flat Panel Primitive is one upright clay panel.
- The model must show readable width, height, and thickness.
- It has meaningful front/back orientation and stands on the support plane.
- It has two controls: Proportions and Edge Softness.
- It is not Door, Hinged Panel, or a rigged/opening object.
- It has no material looks, UV/texturing, rigging, animation, or game-ready UI.

## Visible Catalog

The default Visual Foundry catalog now exposes:

1. Box Primitive
2. Lidded Box
3. Flat Panel Primitive

Trimmed Box remains internal evidence only.

## Make Flow

The accepted Flat Panel flow is:

```text
Choose Flat Panel Primitive
-> Make ready
-> Try panel ideas
-> Use one
-> Adjust Proportions or Edge Softness
-> Add to Pack
-> Export
```

The Make screen must not expose part/focus chips, Material Looks, surface-only
panels, Family Studio, Door, hinge, handle, open/close, rig, or animation copy.

## Candidate Ideas

Flat Panel candidate ideas are:

- Narrow Panel
- Wide Panel
- Tall Panel
- Short Panel
- Soft-Edged Panel
- Sharp Panel

At least four must visibly differ at Make preview size.

## Evidence

Automated coverage verifies:

- Flat Panel validates and exports cleanly.
- Flat Panel appears in the novice catalog.
- Flat Panel has no Door claim.
- Flat Panel has no Material Looks panel.
- Flat Panel has no part/focus chips.
- Flat Panel candidate ideas compile to distinct shapes.
- Flat Panel export copy says the asset is not textured, rigged, animated, or
  game-ready.
- User-facing copy avoids raw kernel/module/provider/slot terms.

Manual dogfood evidence passed locally using the release app and computer-use UI
validation. Evidence is recorded under:

```text
target/flat-panel-primitive-baseline/
```

The manual gate caught and fixed one issue: whole-asset Flat Panel ideas were
initially named with box labels. The generator now emits panel labels for Flat
Panel profiles, and `shape-search` has a regression test for this.

## Next Allowed Work

The next single visible feature is Hinge Edge. It should produce Hinged Panel,
not Door Primitive, unless a later human visual gate approves Door naming.
