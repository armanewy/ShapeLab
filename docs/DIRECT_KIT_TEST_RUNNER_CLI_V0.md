# Direct Kit Test Runner CLI v0

Status: deterministic CLI only.

The Direct Kit test runner checks a Direct Kit without app UI. "Test the kit"
means bounded evidence:

- property endpoint summaries
- preset contact sheet status when presets exist
- ObjectPlan evidence status when the kit starts from an ObjectPlan Draft
- validation warnings
- user-friendly results

It does not mean generated variations, random generation, runtime LLM
integration, automatic approval, public catalog publishing, material editor UI,
UV editing, rigging, or animation.

## CLI

```bash
shape-cli direct-kit test \
  --kit direct-kit.json \
  --out-dir target/direct-kit-test/<kit-id>
```

Output:

- `direct-kit-test-report.json`
- `capability-results.json`
- `property-endpoint-report.json`
- `preset-evidence-report.json`, when presets exist
- `object-plan-evidence-report.json`, when the source is ObjectPlan
- `user-summary.md`

Contact sheets and render evidence are required for stronger review, but the
test runner does not fake evidence when it is missing.
