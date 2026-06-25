# HQ Adversarial Review Results

Wave 41 review artifacts were generated with:

```bash
cargo run -p shape-cli -- hq-adversarial-review --benchmark-dir target/hq-benchmark/roman-bridge-hq --out target/hq-benchmark/roman-bridge-hq/adversarial-review.json
target/debug/shape-cli hq-adversarial-review --benchmark-dir target/hq-benchmark/fantasy-sword --out target/hq-benchmark/fantasy-sword/adversarial-review.json
target/debug/shape-cli hq-adversarial-review --benchmark-dir target/hq-benchmark/round-shield --out target/hq-benchmark/round-shield/adversarial-review.json
target/debug/shape-cli hq-adversarial-review --benchmark-dir target/hq-benchmark/hero-helmet --out target/hq-benchmark/hero-helmet/adversarial-review.json
target/debug/shape-cli hq-adversarial-review --benchmark-dir target/hq-benchmark/pauldron-pair --out target/hq-benchmark/pauldron-pair/adversarial-review.json
target/debug/shape-cli hq-adversarial-review --benchmark-dir target/hq-benchmark/chest-armor --out target/hq-benchmark/chest-armor/adversarial-review.json
target/debug/shape-cli hq-adversarial-review --benchmark-dir target/hq-benchmark/prepared-hero-template-v1 --out target/hq-benchmark/prepared-hero-template-v1/adversarial-review.json
target/debug/shape-cli hq-adversarial-review --benchmark-dir target/hq-benchmark/moba-hero-clay --out target/hq-benchmark/moba-hero-clay/adversarial-review.json
```

## Automatic Evidence

| Profile | Local quality report | Recommendation | Blockers |
| --- | --- | --- | --- |
| roman-bridge-hq | Missing | Draft | 1 |
| fantasy-sword | Missing | Draft | 1 |
| round-shield | Missing | Draft | 1 |
| hero-helmet | Missing | Draft | 1 |
| pauldron-pair | Missing | Draft | 1 |
| chest-armor | Missing | Draft | 1 |
| prepared-hero-template-v1 | Missing | Draft | 1 |
| moba-hero-clay | Present | Usable | 0 |

The missing profiles are not failed art reviews. They are missing local
benchmark evidence in this workspace and must be regenerated before a stronger
claim can be made.

## Missing Evidence

These profile directories did not have a local `quality-report.json` at review
time:

- `roman-bridge-hq`
- `fantasy-sword`
- `round-shield`
- `hero-helmet`
- `pauldron-pair`
- `chest-armor`
- `prepared-hero-template-v1`

The review records missing evidence instead of passing those profiles.

## Manual Review Questions

Every review includes the manual-required visual, mesh, and UX questions from
[`docs/HQ_ADVERSARIAL_REVIEW_GUIDE.md`](HQ_ADVERSARIAL_REVIEW_GUIDE.md).
Automation cannot answer toy-likeness, art direction, reference-board
comparison, or whether a curated DCC kit would beat the generated output.

## Tier Recommendations

- `moba-hero-clay`: automated recommendation `Usable`, with human art review
  still required before default novice exposure.
- All missing-evidence profiles: `Draft` until benchmark evidence is
  regenerated and reviewed.

No profile is recommended as Showcase. Showcase remains blocked without
human/pro approval and adversarial visual review.

## Downgraded Tiers

The generated Wave 41 local results do not downgrade an evidence-backed profile.
They do downgrade absent local evidence to Draft rather than preserving stale
claims from documentation.

## Blockers

- Missing local benchmark reports for the non-MOBA reviewed profiles.
- Manual art review remains pending for every profile.

## Follow-Up Wave Recommendations

- Regenerate `roman-bridge-hq` and Wave 38 gear benchmark directories before
  any public demo quality claim.
- Run adversarial review immediately after every HQ benchmark generation.
- Keep `prepared-hero-template-v1` contract-only until it has actual clay mesh,
  contact-sheet, candidate, and export/reopen evidence.
- Keep `moba-hero-clay` hidden from the default novice catalog until manual
  review confirms it does not look toy-like or procedural.
