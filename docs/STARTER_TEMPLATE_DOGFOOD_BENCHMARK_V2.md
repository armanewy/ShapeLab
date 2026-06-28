# Starter Template Dogfood Benchmark v2

Status: Prompt 4 implementation for the product recovery stack.

Run:

```bash
cargo run -p shape-cli -- starter-template-dogfood-benchmark --out-dir target/starter-template-dogfood
```

The benchmark covers the three product-recovery starter templates:

- Sci-Fi Industrial Crate (`sci-fi-crate`)
- Roman Timber Bridge HQ (`roman-bridge-hq`)
- Stylized Furniture Lamp (`stylized-lamp`)

Each template directory must contain:

- `parent.png`
- `generated-ideas-contact-sheet.png`
- `selected-comparison-sheet.png`
- `control-endpoint-sheet.png`
- `option-gallery-sheet.png`
- `legibility-report.json`
- `adversarial-review.md`
- `dogfood-summary.json`

The command also writes a root `dogfood-summary.json`.

## What Changed From The Old Quality Benchmark

The v2 dogfood command uses the same Foundry candidate-card preview renderer and
512 px decision preview scale used by the app. The previous benchmark could
produce evidence from smaller preview cards, which made it easier for headless
evidence to look acceptable while the product UI was still unclear.

The report records both:

- `benchmark_preview_resolution_px`
- `app_decision_preview_resolution_px`

A benchmark cannot pass if its evidence scale is lower than the app decision
preview scale.

## Pass Criteria

A starter template passes this benchmark only when all automated criteria pass:

- at least four generated ideas are visibly distinct;
- all primary controls have endpoint visibility reports;
- no `TooSubtle` whole-asset candidate is returned as a normal idea;
- parent and candidate outputs have no broken or visibly floating parts;
- export and conformance checks are clean;
- no Advanced Recipe path is required;
- candidate summaries avoid raw technical terms;
- evidence uses the app camera/scale or stricter;
- manual review remains required before any Showcase claim.

Failing templates are reported as `PreviewOnly`. Passing benchmark evidence is
necessary but not always sufficient for a `Usable` catalog recommendation.
Roman Bridge HQ remains `PreviewOnly` until its separate six-direction HQ
Usable gate passes or an explicit exception is approved.

## Adversarial Review Questions

Every `adversarial-review.md` answers:

- Can a novice tell what changed?
- Do candidates look like authored alternatives?
- Does any candidate look like a broken procedural toy?
- Are controls meaningful?
- Would the user continue after two minutes?
- Does the benchmark evidence match the actual app camera/scale?

These answers are generated from benchmark signals. They are evidence, not a
replacement for the Prompt 5 human video/screenshot gate.

## Required Gates

```bash
cargo fmt --all --check
cargo test -p shape-cli starter_template_dogfood --jobs 1
cargo test -p shape-foundry-catalog --jobs 1
cargo clippy --workspace --all-targets -- -D warnings
cargo build --release --workspace
```

Prompt 5 must run the benchmark again at
`target/starter-template-dogfood` and compare the evidence against a release-app
dogfood recording before claiming product recovery pass.
