# ObjectPlan Offline Runner CLI

The ObjectPlan offline runner validates structured ObjectPlan JSON files from
offline tools. It does not add app UI, runtime LLM integration, network calls,
or raw mesh generation.

Validate one plan:

```bash
orchard-cli object-plan validate object-plan.json
```

The command prints an `ObjectPlanValidationReport` as JSON and exits non-zero
when the plan is invalid.

Prepare deterministic runner artifacts:

```bash
orchard-cli object-plan run \
  --plan object-plan.json \
  --out-dir target/object-plan-runs/example-plan
```

The run command writes:

- `validation-report.json`
- `primitive-summary.json`
- `normalized-object-plan.json`
- `renderability-report.json`
- `rendering-report.json`
- `plan-user-summary.md`

Contact-sheet evidence can be requested with `--contact-sheet`. Until
ObjectPlan materialization to renderable geometry exists, the command writes
an honest blocked report and does not create `contact-sheet.png`.

The runner output is validation and review preparation. It must not be
described as generated asset geometry until materialization and real render
evidence are wired for the plan.

The legacy `orchard-cli object-plan render --plan ... --out-dir ...` command is
kept as a compatibility alias for the same validation and preparation flow.

The runner uses the ObjectPlan validator, so it rejects unsupported primitives,
unknown properties, out-of-domain property values, invalid attachments, raw mesh
payloads, public catalog publishing requests, and validation bypass attempts.
LLM-authored plans are handled only as offline Draft inputs; the CLI has no LLM
runtime dependency.
