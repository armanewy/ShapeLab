# ObjectPlan v0 Integration Report

Date: 2026-07-01

## Merged Branches

- `2c0f45d` Post-Primitive Composition Truth Gate
- `8224064` ObjectPlan DSL Contracts v0
- `76181bd` ObjectPlan Offline Runner CLI
- `a348f77` ObjectPlan Contact Sheet Evidence v0
- `2279f48` Primitive Preset Library v0
- `392a7e4` ObjectPlan Batch Runner v0
- `09b7749` Offline LLM Draft Policy v0
- `e5b99ad` ObjectPlan Review UI Internal Gate

## Proof Questions

| Question | Result | Evidence |
| --- | --- | --- |
| Can a structured ObjectPlan describe a supported primitive? | Pass | `fixtures/object-plan/valid_box_plan.json`, `fixtures/object-plan/valid_sphere_plan.json` |
| Can a structured ObjectPlan describe a supported composition? | Pass | `fixtures/object-plan/valid_panel_knob_plan.json` |
| Are invalid primitives/properties/attachments rejected? | Pass | ObjectPlan validators and `fixtures/object-plan/invalid_unknown_primitive_plan.json` in the batch evidence |
| Are raw mesh payloads rejected? | Pass | `cargo test -p orchard-foundry object_plan --jobs 1` and `cargo test -p orchard-cli object_plan --jobs 1` |
| Can plans be validated offline? | Pass | `orchard-cli object-plan run --plan fixtures/object-plan/valid_panel_knob_plan.json --out-dir target/object-plan-v0/valid-panel-knob --contact-sheet` |
| Can plans produce honest render/contact-sheet evidence or honest render-blocked reports? | Pass | `target/object-plan-v0/valid-panel-knob/renderability-report.json` reports `renderable: false`; no fake contact sheet is written |
| Can batches be run without catalog publication? | Pass | `target/object-plan-v0/batch-basic/batch-validation-report.json` reports `approved: false` |
| Can an offline LLM be instructed to output only draft JSON? | Pass | `docs/OFFLINE_LLM_DRAFT_POLICY_V0.md` and `docs/llm_prompt_packs/object_plan_draft_v0.md` |
| Does the UI remain internal-only? | Pass | `docs/OBJECT_PLAN_REVIEW_UI_INTERNAL_GATE.md` and screenshots under `target/object-plan-review-ui/` |
| Are no material/surface/UV/rigging/animation claims added? | Pass | Status docs keep these capabilities blocked |

## CLI Evidence

Single plan run:

```bash
orchard-cli object-plan run \
  --plan fixtures/object-plan/valid_panel_knob_plan.json \
  --out-dir target/object-plan-v0/valid-panel-knob \
  --contact-sheet
```

Outputs:

- `target/object-plan-v0/valid-panel-knob/validation-report.json`
- `target/object-plan-v0/valid-panel-knob/primitive-summary.json`
- `target/object-plan-v0/valid-panel-knob/normalized-object-plan.json`
- `target/object-plan-v0/valid-panel-knob/renderability-report.json`
- `target/object-plan-v0/valid-panel-knob/rendering-report.json`
- `target/object-plan-v0/valid-panel-knob/visual-evidence-report.json`
- `target/object-plan-v0/valid-panel-knob/plan-user-summary.md`

Observed result: validation passed with zero issues. Rendering is honestly
blocked because ObjectPlan preview materialization is not wired yet. No
`contact-sheet.png` was written.

This means ObjectPlan v0 is validation and review infrastructure. It does not
yet prove reusable prototype geometry for every supported plan.

Batch run:

```bash
orchard-cli object-plan batch-run \
  --input fixtures/object-plan/batch-basic \
  --out-dir target/object-plan-v0/batch-basic
```

Outputs:

- `target/object-plan-v0/batch-basic/batch-validation-report.json`
- `target/object-plan-v0/batch-basic/keep-regenerate-simplify.md`
- `target/object-plan-v0/batch-basic/batch-user-summary.md`
- per-plan reports under `target/object-plan-v0/batch-basic/plans/`

Observed result: 4 total plans, 3 passed validation, 1 failed validation, 0
rendered, 4 unsupported for rendering, `human_review_required: true`, and
`approved: false`.

## UI Evidence

ObjectPlan review UI is internal-only and gated by
`SHAPE_LAB_OBJECT_PLAN_REVIEW`.

- `target/object-plan-review-ui/default-hidden.png`: review entry hidden by default.
- `target/object-plan-review-ui/dev-entry-visible.png`: review entry visible under the dev flag.
- `target/object-plan-review-ui/review-drawer.png`: review drawer shows Draft only, Not catalog published, Human review required, batch target, contact-sheet area, and Keep / Regenerate / Simplify / Blocked labels.

Computer Use note: the app state accessibility read timed out during the gate.
`list_apps` confirmed the rebuilt Object Orchard app was frontmost and running, and
the drawer screenshot was captured after the app's internal screenshot scenario
reported `ObjectPlanReviewDrawer: PASS`.

## Automated Gates

Passed in this integration branch:

- `cargo fmt --all --check`
- `python3 scripts/check_source_hygiene.py`
- `cargo test -p orchard-foundry object_plan --jobs 1`
- `cargo test -p orchard-foundry primitive_preset --jobs 1`
- `cargo test -p orchard-foundry primitive_composition --jobs 1`
- `cargo test -p orchard-cli object_plan --jobs 1`
- `cargo test -p orchard-app object_plan --jobs 1`
- `cargo test -p orchard-app foundry --jobs 1`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo build --release --workspace`

## Next Allowed Work

- Wire ObjectPlan materialization into the preview/render evidence path.
- Treat render-blocked reports as truthful but incomplete until
  materialization exists.
- Expand the approved primitive and safe-anchor vocabulary only through schema
  gates.
- Add more deterministic primitive presets after validator coverage exists.
- Start Prototype Pack Mode only after batch evidence can render or report
  failures honestly at pack scale.

Still blocked:

- runtime LLM integration
- public catalog publishing
- material/surface workflows
- UV/texturing
- rigging and animation
- imported mesh editing
- broad family generation claims
