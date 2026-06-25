# Wave 38 Showcase Gear Pack Report

Wave 38 adds five promoted gear kits to the built-in Visual Foundry catalog:

- Fantasy Sword
- Round Shield
- Hero Helmet
- Pauldron Pair
- Chest Armor

Implementation summary:

- added `crates/shape-foundry-catalog/src/showcase_gear.rs`;
- registered the gear fixtures in the built-in catalog;
- promoted the selected gear kits to Usable kit metadata, not Showcase;
- added deterministic product pack metadata through
  `showcase_gear_pack_report`;
- added catalog tests for compile validity, review gating, whole-model option
  preview refs, product-safe metadata, and pack coherence;
- generated HQ benchmark coverage proving all five promoted gear kits reach
  Usable with six surviving candidates and verified export/reopen evidence;
- updated release/product inventory gates from eleven to sixteen built-in
  profiles.

Required benchmark artifacts are emitted under:

```text
target/hq-benchmark/<profile>/
```

Each benchmark directory contains:

- `contact-sheet.png`
- `front.png`
- `three-quarter.png`
- `side.png`
- `back.png`
- `wireframe.png`
- `silhouette.png`
- `mesh-stats.json`
- `semantic-parts.json`
- `candidate-report.json`
- `controls-visibility-report.json`
- `export-reopen-report.json`
- `quality-report.json`

Release note:

The five gear kits satisfy the automated Usable evidence target only when their
HQ benchmark reports pass with export/reopen verification. Default novice
catalog exposure remains blocked by manual review policy, and Showcase remains
blocked without human approval plus adversarial visual review.
