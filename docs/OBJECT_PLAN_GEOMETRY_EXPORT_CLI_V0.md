# ObjectPlan Geometry Export CLI v0

Status: implemented for supported ObjectPlan drafts.

`shape-cli object-plan export-geometry` converts supported ObjectPlans into a
geometry-only GLB draft package. The command materializes the ObjectPlan first,
writes the usual review artifacts, and exports GLB only when the draft has no
unresolved nodes or attachments.

## Command

```bash
shape-cli object-plan export-geometry \
  --plan fixtures/object-plan/valid_box_plan.json \
  --out-dir target/object-plan-geometry-export/box \
  --format glb
```

## Outputs

For supported plans:

- `asset.glb`
- `geometry-export-report.json`
- `geometry-export-user-summary.md`
- `normalized-object-plan.json`
- `materialization-report.json`
- `materialized-object-draft.json`
- `materialized-user-summary.md`
- `render-evidence-report.json`
- render evidence PNGs and `contact-sheet.png` when preview evidence is
  available

For blocked plans:

- `geometry-export-report.json`
- `geometry-export-user-summary.md`
- materialization artifacts when the plan parsed far enough to materialize
- no `asset.glb`

## Supported V0 Scope

Supported:

- Box Primitive
- Flat Panel Primitive
- Sphere Primitive
- Panel with Knob composition when materialization has no unresolved pieces

Unsupported:

- textures
- material looks
- UV support claims
- collision or gameplay metadata
- rigging
- animation
- arbitrary mesh payloads
- public catalog publishing
- game-ready status

Forbidden capability fields in input JSON are blocked before materialization.
They are not treated as ObjectPlan schema extensions.

## GLB Contents

The GLB contains:

- glTF 2.0 binary container
- one mesh primitive
- `POSITION`
- `NORMAL`
- triangle indices
- one neutral placeholder material so the mesh is visible

The GLB does not contain:

- `TEXCOORD_0`
- images
- textures
- authored material looks
- collision
- skins or skeletons
- animations
- custom game-ready extensions

The neutral placeholder material is only a visibility aid. It is not a Surface
workflow, texture workflow, or material-look claim.

## Report Guarantees

`geometry-export-report.json` must keep:

- `includes_uvs: false`
- `includes_textures: false`
- `includes_material_looks: false`
- `includes_collision: false`
- `includes_rig: false`
- `includes_animation: false`
- `game_ready: false`
- `human_review_required: true`

The command exits nonzero when export is blocked or failed.

## Godot Boundary

This CLI does not prove Godot import. Godot import proof is a separate gate.
Until that harness passes, Shape Lab must not claim Godot-ready geometry.
