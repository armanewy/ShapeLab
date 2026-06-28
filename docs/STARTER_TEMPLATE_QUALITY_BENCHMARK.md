# Starter Template Quality Benchmark

Shape Lab gates the default novice starter catalog with a headless benchmark for
the three starter templates:

- Sci-Fi Industrial Crate (`sci-fi-crate`)
- Roman Timber Bridge HQ (`roman-bridge-hq`)
- Stylized Furniture Lamp (`stylized-lamp`)

Run:

```bash
cargo run -p shape-cli -- starter-template-quality-benchmark --out-dir target/starter-template-quality
```

The command writes one directory per starter template:

```text
target/starter-template-quality/
  sci-fi-crate/
  roman-bridge-hq/
  stylized-lamp/
```

Each template directory contains:

- `parent.png`
- `generated-ideas-contact-sheet.png`
- `selected-comparison-sheet.png`
- `control-endpoint-sheet.png`
- `option-gallery-sheet.png`
- `legibility-report.json`
- `adversarial-review.md`

The command also writes `target/starter-template-quality/summary.json`.
The process succeeds when evidence is generated; inspect `summary.json` and each
`legibility-report.json` for per-template pass/fail and catalog recommendation.

## Pass Criteria

A starter template passes only when all automated criteria are true:

- at least four returned whole-asset ideas are visible and distinct;
- every primary novice control has a readable endpoint report;
- no returned whole-asset idea is classified `TooSubtle`;
- parent and candidate models pass conformance and model validation;
- package export verification is clean;
- no Advanced Recipe or authoring lane is needed;
- candidate labels and summaries do not expose raw technical terms such as
  provider, scalar, recipe, fingerprint, semantic, mesh, triangle, or
  conformance.

Rejected `TooSubtle` proposals are allowed. Returned normal candidate cards are
not.

## Adversarial Review

`adversarial-review.md` records the required review questions:

- Can a novice tell what changed?
- Do candidates look like real authored alternatives?
- Does any candidate look like a broken procedural toy?
- Are the controls meaningful?
- Would the user continue after two minutes?

The markdown is generated from the same pass/fail signals as
`legibility-report.json`; it is not a substitute for human review.

## Catalog Behavior

Starter templates that pass may be curated as `Usable`. A failing starter must
remain in source but be downgraded to `PreviewOnly`, which keeps it out of the
default novice catalog while still allowing preview/developer review.

The catalog helper
`starter_template_curation_state_from_quality` enforces this rule for tests and
future release integration. `PreviewOnly` starters must not be deleted just
because they fail the benchmark.

Current benchmark evidence passes all three starter templates:
`sci-fi-crate`, `roman-bridge-hq`, and `stylized-lamp`. A future regression in
any starter must downgrade only that starter to `PreviewOnly`.
