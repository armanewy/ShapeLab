# ObjectPlan Materialization v1 Integration Report

Date: 2026-07-01

## Verdict

`OBJECT_PLAN_MATERIALIZATION_V1_REVIEW_READY`

ObjectPlan has moved from validation-only infrastructure to Draft, review-required materialization and render evidence for supported primitive plans.

Subsequent Geometry Export v0 work moved supported ObjectPlan drafts to
geometry-only GLB export. See
`docs/GEOMETRY_EXPORT_V0_INTEGRATION_REPORT.md`. Current ObjectPlan outputs are
still not Godot-ready or game-ready engine packages.

## Integrated Work

- `75a986f` - ObjectPlan v0 truth/render blocker gate
- `ce8bced` - ObjectPlan materialization contracts
- `c9e5b7b` - Primitive preset library hardening
- `dc08222` - ObjectPlan materializer CLI
- `f9eda66` - ObjectPlan render evidence
- `9c3ab3d` - ObjectPlan batch review

## CLI Evidence

Evidence path: `target/object-plan-materialization-v1`

- `box/`: valid Box Primitive materialization and render evidence.
- `panel/`: valid Flat Panel Primitive materialization and render evidence.
- `panel-knob/`: valid Flat Panel plus knob-like Sphere attachment materialization and render evidence.
- `batch-basic/`: mixed batch with rendered supported plans and blocked/simplified unsupported paths.

Expected evidence files include:

- `materialization-report.json`
- `render-evidence-report.json`
- `plan-preview.png`
- `node-previews/*.png`
- `contact-sheet.png`
- `batch-validation-report.json`
- `batch-materialization-report.json`
- `batch-render-evidence-report.json`
- `batch-contact-sheet.png`

No evidence report may contain `approved: true` or `publish_allowed: true`.

## Proof Questions

| Question | Result |
| --- | --- |
| Can ObjectPlan validate supported plans? | Pass. Box, Flat Panel, Sphere, and supported Panel with Knob plans validate. |
| Can ObjectPlan materialize supported primitive plans? | Pass. Supported primitive nodes become Draft primitive instances. |
| Can ObjectPlan produce real render/contact-sheet evidence for supported plans? | Pass. Supported plans emit PNG previews and contact sheets. |
| Do unsupported plans produce honest blocked reports? | Pass. Unsupported primitives and unsupported attachments do not fake contact sheets. |
| Can batch review classify Keep / Regenerate / Simplify / Blocked? | Pass. Batch policy supports all four labels; the integration batch observes Keep, Simplify, and Blocked, while Regenerate remains the valid-but-incomplete-evidence path. |
| Are plans automatically approved? | Pass. Reports keep `approved: false`. |
| Are plans published to catalog? | Pass. Reports keep `publish_allowed: false`. |
| Is runtime LLM still absent? | Pass. No runtime LLM dependency or app-side LLM drafting was added. |
| Are material/surface, UV/texturing, rigging, and animation still absent? | Pass. No such workflow was added. |

## Automated Gates

| Gate | Result |
| --- | --- |
| `cargo fmt --all --check` | Pass |
| `python3 scripts/check_source_hygiene.py` | Pass |
| `cargo test -p shape-foundry object_plan --jobs 1` | Pass |
| `cargo test -p shape-foundry primitive_preset --jobs 1` | Pass |
| `cargo test -p shape-cli object_plan --jobs 1` | Pass |
| `cargo test -p shape-render foundry --jobs 1` | Pass |
| `cargo test -p shape-app foundry --jobs 1` | Pass |
| `cargo clippy --workspace --all-targets -- -D warnings` | Pass |
| `cargo build --release --workspace` | Pass |

## Current Allowed Status

- ObjectPlan can validate and materialize supported primitive plans.
- ObjectPlan can produce contact-sheet evidence for supported plans.
- Batch review can classify Draft plans for human review.
- ObjectPlan outputs remain Draft and review-required.
- Geometry-only GLB export exists for supported ObjectPlan drafts after the
  subsequent Geometry Export v0 gate.
- Godot import proof is required before claiming Godot-ready geometry; a
  blocked Godot proof report is not a Godot-ready claim.
- No runtime LLM integration exists.
- No public catalog publishing exists.
- No material/surface, UV/texturing, collision, rigging, or animation workflow
  exists.

## Next Allowed Work

- Primitive Surface V0 contracts
- Godot import proof on a machine with Godot installed
- Family Studio Lite Direct Kit contracts and local/private Personal Kit
  persistence
- Prototype Pack brief contracts

## Still Blocked

- Broad material editor
- UV editing UI
- Godot-ready or game-ready claims before import proof
- Collision/gameplay metadata
- Rigging/animation UI
- Automatic catalog publishing
- Game-ready claims
- Family Studio Lite public authoring, broad family generation, generated
  candidate trays, reviewed/showcase promotion, and runtime LLM generation
