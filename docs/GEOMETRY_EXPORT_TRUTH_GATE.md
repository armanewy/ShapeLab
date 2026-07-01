# Geometry Export Truth Gate

Date: 2026-07-01

## Verdict

`GEOMETRY_EXPORT_NOT_YET_PROVEN`

ObjectPlan Materialization v1 can validate, materialize, render, and batch
review supported primitive plans. It does not yet export geometry-only GLB
engine assets, and it does not prove Godot-ready or game-ready output.

## What Materializes Today

Supported ObjectPlans can materialize when they contain only supported
primitive nodes and supported safe-anchor attachments:

- Box Primitive
- Flat Panel Primitive
- Sphere Primitive
- Flat Panel plus knob-like Sphere attachment through the supported handle-zone
  anchor

Invalid plans, unsupported primitive kinds, raw mesh payloads, public catalog
publish requests, or unsupported attachments remain blocked or unresolved.

## What Renders Today

Supported materialized ObjectPlans can produce PNG review evidence:

- `plan-preview.png`
- `node-previews/*.png`
- `contact-sheet.png`
- `render-evidence-report.json`

Those files are visual evidence for human review. They are not approval, public
catalog publishing, or an engine package.

## What Exports Today

ObjectPlan Materialization v1 does not yet export GLB. No current ObjectPlan
path can truthfully claim geometry-only GLB output, Godot-ready geometry, or
game-ready output.

The current product Export UI is not an ObjectPlan geometry-only GLB proof. It
is product flow copy for exporting the current clay primitive/package surface
that exists today; it is not a verified Godot import package and must not be
described as Godot-ready.

## Overclaim Audit

Current status docs should say:

- ObjectPlan Materialization v1 produces Draft, review-required internal asset
  graphs and contact-sheet evidence for supported primitive plans.
- Geometry-only GLB export is the next proof.
- Geometry-only export will not include UVs, textures, material looks,
  collision, rigging, animation, or game-ready status.
- Godot import proof is required before claiming Godot-ready geometry.
- Runtime LLM integration remains absent.
- Public catalog publishing remains blocked.

Docs must not claim that current ObjectPlan output is Godot-ready, game-ready,
textured, rigged, animated, UV-authored, collision-ready, or publicly
publishable.
