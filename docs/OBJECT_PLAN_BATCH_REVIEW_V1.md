# ObjectPlan Batch Review v1

ObjectPlan Batch Review v1 runs a directory or batch file of ObjectPlans through validation, materialization, and render-evidence generation for offline review.

The CLI command is:

```bash
shape-cli object-plan batch-run \
  --input fixtures/object-plan/batch-basic \
  --out-dir target/object-plan-batch-review
```

Batch output includes:

- `batch-validation-report.json`
- `batch-materialization-report.json`
- `batch-render-evidence-report.json`
- `batch-contact-sheet.png`, when at least one plan renders
- per-plan reports under `plans/`
- `batch-user-summary.md`
- `keep-regenerate-simplify.md`

Recommendations are review labels only:

- `Keep`: suitable for human review; not approved.
- `Regenerate`: valid, but render evidence is missing or incomplete.
- `Simplify`: the plan is too complex for the available primitives or anchors.
- `Blocked`: invalid or unsupported.

All batch outputs keep `human_review_required: true`, `approved: false`, and `publish_allowed: false`.

Batch Review v1 is offline review infrastructure. It is not Prototype Pack Mode, not runtime LLM integration, not public catalog publishing, and not a surface/material, UV/texturing, rigging, or animation workflow.
