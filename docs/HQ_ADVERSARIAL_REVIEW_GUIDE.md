# HQ Adversarial Review Guide

Wave 41 adds a deterministic adversarial review pass over HQ benchmark
evidence. This pass is a quality gate, not an automatic art judge.

Run:

```bash
cargo run -p shape-cli -- hq-adversarial-review --benchmark-dir target/hq-benchmark/<profile> --out target/hq-benchmark/<profile>/adversarial-review.json
```

The review consumes `quality-report.json` and adjacent evidence files. If the
benchmark directory or report is missing, the review records missing evidence
and recommends Draft. It must not treat absent contact sheets or missing export
proof as a pass.

## Manual Questions

Reviewers must answer the subjective questions manually:

- Does this look like a toy?
- Does the silhouette read at 128px?
- Do variants preserve identity?
- Do armor, bridge, or gear pieces look attached rather than pasted on?
- Do all generated candidates look art-directed?
- Would this embarrass us next to a private clay-render reference board?
- Are primary controls visibly meaningful?
- Are there too many choices for a noob?
- Are candidates coherent or just random combinations?
- Does any output look like procedural filler?
- Would a curated Blender/Houdini kit beat this today?
- Is Visual Foundry still simpler than traditional modeling for the task?

The JSON report marks these fields as manual-required and lists them under
`cannot_automatically_judge_fields`.

## Tier Rules

- Showcase requires Usable evidence plus human/pro approval and adversarial
  visual review.
- Usable requires contact sheets, visible primary-control deltas, six surviving
  candidates or an approved exception, and export/reopen proof.
- Usable cannot require Advanced Recipe for the intended novice task.
- Draft and Prototype remain hidden from the default novice catalog.
- Hero output remains clay mesh only unless future work adds UVs, materials,
  rigging, animation, and game-ready export layers.

The review can recommend downgrades. It cannot promote a profile above the
evidence in `quality-report.json`.
