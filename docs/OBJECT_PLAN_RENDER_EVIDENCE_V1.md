# ObjectPlan Render Evidence v1

ObjectPlan Render Evidence v1 makes supported materialized ObjectPlans visibly reviewable.

The supported CLI path is:

```bash
orchard-cli object-plan materialize \
  --plan fixtures/object-plan/valid_box_plan.json \
  --out-dir target/object-plan-render-evidence/box \
  --render-evidence
```

For supported drafts, the command writes:

- `plan-preview.png`
- `node-previews/*.png`
- `contact-sheet.png`
- `render-evidence-report.json`

The render evidence report records whether evidence was rendered, whether a draft was materialized, the plan ID, preview count, contact sheet path, unsupported primitives, unsupported attachments, warnings, `user_review_required: true`, and `approved: false`.

Contact sheets are evidence for human review. They are not approval, not publishing, and not a claim that the output is game-ready.

Unsupported or invalid plans write `render-evidence-report.json` with `rendered: false` and do not write `contact-sheet.png`. Render-blocked reports remain valid and required when a plan cannot be previewed honestly.

Render Evidence v1 does not add runtime LLM integration, public catalog publishing, surface/material workflows, UV/texturing, rigging, animation, or arbitrary mesh import.
