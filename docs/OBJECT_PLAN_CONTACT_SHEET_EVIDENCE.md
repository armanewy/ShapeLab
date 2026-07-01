# ObjectPlan Contact Sheet Evidence v0

Date: 2026-07-01

ObjectPlan contact-sheet evidence makes validation inspectable, but it does
not approve a plan. Every ObjectPlan remains Draft until human review or a
trusted local review workflow keeps it.

## CLI

```bash
shape-cli object-plan run \
  --plan object-plan.json \
  --out-dir target/object-plan-runs/example-plan \
  --contact-sheet
```

The command always validates first. It writes:

- `validation-report.json`
- `primitive-summary.json`
- `normalized-object-plan.json`
- `renderability-report.json`
- `plan-user-summary.md`

When contact-sheet evidence is requested, it also writes
`visual-evidence-report.json`.

## Honest Renderability

If ObjectPlan materialization is not wired for a plan, the command records
`renderable: false`, lists missing preview bindings, and does not write
`contact-sheet.png`.

That render-blocked state is valid ObjectPlan v0 output, but it is incomplete.
It identifies the next materialization/render-evidence work rather than
pretending the plan produced visible reusable geometry.

Unsupported plans must fail honestly. The CLI must not claim rendered evidence,
must not write placeholder contact sheets, and must not mark the plan approved.

## Review Rule

Contact sheets do not equal approval. `visual-evidence-report.json` keeps:

- `user_review_required: true`
- `approved: false`

Future renderer wiring may add `node-previews/`, `plan-preview.png`, and
`contact-sheet.png` for renderable plans. Human review remains required.
