# ObjectPlan Batch Runner v0

Date: 2026-07-01

ObjectPlan Batch Runner v0 runs many ObjectPlans offline and produces review
reports. It is review infrastructure, not Prototype Pack Mode and not catalog
publishing.

## CLI

```bash
shape-cli object-plan batch-run \
  --input target/object-plan-inputs/basic-batch \
  --out-dir target/object-plan-batches/basic-batch
```

`--input` may be:

- a directory of ObjectPlan JSON files
- an ObjectPlanBatch JSON file with relative plan paths

## Output

The command writes:

- `batch-validation-report.json`
- `keep-regenerate-simplify.md`
- `batch-user-summary.md`
- per-plan reports under `plans/`

If future ObjectPlan render bindings produce rendered plan evidence, a later
runner may write `batch-contact-sheet.png`. The current runner does not fake
that file when renderability is blocked.

## Review Rules

The batch report records:

- total plans
- passed validation
- failed validation
- rendered count
- unsupported count
- `human_review_required: true`
- `approved: false`

Keep, Regenerate, Simplify, and Blocked labels are review recommendations only.
They do not publish, approve, or mutate the catalog.

## Boundaries

The batch runner does not call an LLM, does not import raw meshes, does not add
app UI, does not publish to a public catalog, and does not add material,
surface, UV, rigging, animation, or game-ready support.
