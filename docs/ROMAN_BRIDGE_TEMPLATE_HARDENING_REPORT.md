# Roman Bridge Template Hardening Report

## Current Tier Truth

Current catalog recommendation: `PreviewOnly`.

Prompt 0 chose the conservative downgrade path. Roman Bridge HQ has useful
starter-template evidence, but the broader HQ Usable-tier gate still requires
six surviving direction candidates or an approved exception. No exception is
approved, so `roman-bridge-hq` must stay out of the default novice catalog.

## Scope

Prompt 3B v2 re-verified the existing `roman-bridge-hq` hardening. The earlier
Roman hardening pass reauthored `roman-bridge-hq` so untextured clay previews have stronger
shape separation across deck width, span length, structural heft, supports,
bracing, railings, and detail density.

Changed authoring points:

- six-course parameterized deck planks replace the single-slab HQ deck read;
- paired main span beams remain visible under span-length changes;
- support providers separate round piles, squared posts, stacked stone piers,
  and trestle frames without disconnected supports;
- bracing providers separate minimal ties, angled X/K lanes, and heavy stacked
  reinforcement;
- rail providers separate low curb, guard, and lookout courses by height and
  side offset;
- connector providers separate clean cross ties, bolted joinery, and dense
  weathered fasteners.

## Contact Sheet Evidence

Generated command:

```bash
cargo run -p shape-cli -- hq-quality-benchmark --profile roman-bridge-hq --out-dir target/hq-benchmark/roman-bridge-hq --verify-export
```

Generated output:

- `target/hq-benchmark/roman-bridge-hq/contact-sheet.png`
- `target/hq-benchmark/roman-bridge-hq/quality-report.json`
- `target/hq-benchmark/roman-bridge-hq/candidate-report.json`
- `target/hq-benchmark/roman-bridge-hq/export-reopen-report.json`

The benchmark reported clay/contact-sheet output, clean model validation,
verified export/reopen, and visible geometry deltas for all seven primary
controls.

## Candidate Readability

Prompt 4 requires at least four visibly distinct bridge ideas. The catalog test
`roman_bridge_hq_explore_returns_clear_distinct_whole_asset_directions`
generates six Explore candidates, rejects `TooSubtle` whole-asset candidates,
validates each candidate model, and requires at least four distinct fixed-camera
signatures.

Named direction strategies now cover:

- Light Crossing
- Reinforced
- Wide Crossing
- Compact Span
- Stone-Pier Outpost
- Detailed Timberwork
- Minimal Span

The generated HQ benchmark candidate report recorded `returned_count = 6` and
`candidate_survival_count = 4`. This satisfies Prompt 4's four-distinct-idea
gate, but not the broader HQ Usable-tier gate that requires six surviving
directions or an approved exception.

## Connectivity

Automated checks cover:

- no disconnected required attachments;
- support options are structurally distinct and model-valid;
- bracing options are structurally distinct and model-valid;
- deck width locks change walkable width and still export/reopen cleanly;
- all HQ primary controls have visible endpoint differences.

The HQ benchmark `mesh_validity_summary` reports zero errors, zero warnings, and
zero accidental intersections.

## Adversarial Critic

Generated command:

```bash
cargo run -p shape-cli -- hq-adversarial-review --benchmark-dir target/hq-benchmark/roman-bridge-hq --out target/hq-benchmark/roman-bridge-hq/adversarial-review.json
```

Result: `tier_recommendation = prototype`.

Blockers:

- `quality_report_blocker: Usable requires six surviving direction candidates or an approved exception`
- `recomputed_quality_blocker: Usable requires six surviving direction candidates or an approved exception`

Non-blocking finding:

- manual art review is still pending; automatic evidence is not human approval.

Prompt 4 hardening is complete, but broader Usable-tier promotion remains
blocked until the six-direction benchmark gate is met or an exception is
approved. Until then, Roman Bridge HQ remains `PreviewOnly`.
