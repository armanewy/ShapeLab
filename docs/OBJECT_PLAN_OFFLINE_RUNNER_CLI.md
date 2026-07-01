# ObjectPlan Offline Runner CLI

The ObjectPlan offline runner validates structured ObjectPlan JSON files from
offline tools. It does not add app UI, runtime LLM integration, network calls,
or raw mesh generation.

Validate one plan:

```bash
shape-cli object-plan validate object-plan.json
```

The command prints an `ObjectPlanValidationReport` as JSON and exits non-zero
when the plan is invalid.

Prepare deterministic runner artifacts:

```bash
shape-cli object-plan render \
  --plan object-plan.json \
  --out-dir target/object-plan-runs/example-plan
```

The render command writes:

- `validation-report.json`
- `primitive-summary.json`
- `rendering-report.json`
- `plan-user-summary.md`

`contact-sheet.png` is written only after ObjectPlan materialization to renderable
geometry exists. Until then, `rendering-report.json` records a blocked rendering
status with a clear reason.

The runner uses the ObjectPlan validator, so it rejects unsupported primitives,
unknown properties, out-of-domain property values, invalid attachments, raw mesh
payloads, public catalog publishing requests, and validation bypass attempts.
LLM-authored plans are handled only as offline Draft inputs; the CLI has no LLM
runtime dependency.
