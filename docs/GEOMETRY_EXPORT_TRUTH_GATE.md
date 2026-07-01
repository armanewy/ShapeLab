# Geometry Export Truth Gate

Date: 2026-07-01

## Verdict

`HISTORICAL_BASELINE_SUPERSEDED_BY_GEOMETRY_EXPORT_V0`

This gate captured the pre-export baseline before Geometry Export v0. The
current status is tracked in `docs/GEOMETRY_EXPORT_V0_INTEGRATION_REPORT.md`.

At this gate's baseline, ObjectPlan Materialization v1 could validate,
materialize, render, and batch review supported primitive plans. It did not yet
export geometry-only GLB engine assets, and it did not prove Godot-ready or
game-ready output.

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

## What Exported At This Baseline

ObjectPlan Materialization v1 did not yet export GLB at this baseline.
Geometry Export v0 has since added geometry-only GLB export for supported
ObjectPlan drafts. No ObjectPlan path can truthfully claim Godot-ready geometry
or game-ready output until a Godot import proof passes.

The current product Export UI is not an ObjectPlan geometry-only GLB proof. It
is product flow copy for exporting the current clay primitive/package surface
that exists today; it is not a verified Godot import package and must not be
described as Godot-ready.

## Overclaim Audit

At this gate's baseline, status docs needed to say:

- ObjectPlan Materialization v1 produces Draft, review-required internal asset
  graphs and contact-sheet evidence for supported primitive plans.
- Geometry-only GLB export was the next proof.
- Geometry-only export will not include UVs, textures, material looks,
  collision, rigging, animation, or game-ready status.
- Godot import proof is required before claiming Godot-ready geometry.
- Runtime LLM integration remains absent.
- Public catalog publishing remains blocked.

Docs must not claim that current ObjectPlan output is Godot-ready, game-ready,
textured, rigged, animated, UV-authored, collision-ready, or publicly
publishable.
