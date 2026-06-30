# Hinge Edge Feature Module v0

Date: 2026-06-30

## Status

`PASS - INTERNAL FEATURE EVIDENCE`

Hinge Edge is the first visible feature after Flat Panel Primitive. It produces
an internal Hinged Panel profile: one upright clay panel plus a visible
hinge-side edge.

This is not Door, Door Primitive, open/close behavior, rigging, animation,
material looks, or a textured asset.

## Module Contract

The Hinge Edge module declares:

- required zone: hinge-candidate edge
- provided role: hinge_edge
- owned control: Hinge Edge
- candidate hook: hinged-panel ideas
- quality gates: visible hinge edge, no motion claim, attached edge, visible
  endpoint

The Flat Panel Primitive profile remains unchanged. It still has only:

- Proportions
- Edge Softness

## Geometry

The feature uses export-safe clay geometry: a tangent vertical side strip along
the panel's hinge-candidate edge. The Hinge Edge control changes the strip's
front/back depth, which makes the endpoint visible without pushing geometry
through the panel body.

The first implementation attempt overlapped the edge strip into the panel. The
model validator rejected it with triangle-intersection evidence, so the final
geometry uses a tangent strip instead.

## Candidate Ideas

The internal Hinged Panel fixture exposes:

- Clean Hinged Panel
- Wide Hinged Panel
- Tall Hinged Panel
- Heavy Edge Panel
- Minimal Hinged Panel

All five candidates compile to distinct artifact fingerprints.

## Visual Evidence

Evidence path:

`target/hinge-edge-feature-module-v0/`

Artifacts:

- `flat-panel-parent.png`
- `hinged-panel-parent.png`
- `candidate-contact-sheet.png`
- `control-endpoint-sheet.png`
- `quality-report.json`

Quality report result:

- hinge edge visible in pure clay: pass
- hinge edge does not imply motion: pass
- hinge edge attached: pass
- hinge edge endpoint visible: pass
- model remains a panel: pass
- candidates differ: pass
- no Door claim: pass
- no material/rig/animation claim: pass
- export clean: pass

## Tests

Focused tests cover:

- Flat Panel without Hinge Edge remains unchanged.
- Hinged Panel includes exactly one hinge edge role.
- Hinge Edge is visible through endpoint evidence.
- Candidate ideas visibly differ by compiled artifact fingerprint.
- Hinge Edge does not add handle, knob, inset, motion, material, rigging, or
  animation claims.
- Export verification passes.

## Next Gate

The next branch may expose Hinged Panel in the Make loop if the product gate
accepts this visual evidence.

Still blocked:

- Door naming
- handle/knob
- inset panel
- open/close motion
- material looks
- UV/texturing
- rigging/animation
