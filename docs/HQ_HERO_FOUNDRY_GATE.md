# HQ Hero Foundry Gate

This gate defines the evidence required before a Hero Foundry profile can be
called a usable clay hero generator.

## Required Command

```bash
cargo run -p shape-cli -- hq-quality-benchmark --profile moba-hero-clay --out-dir target/hq-benchmark/moba-hero-clay
```

The command must emit:

- `quality-report.json`
- `contact-sheet.png`
- `front.png`, `three-quarter.png`, `side.png`, `back.png`
- `wireframe.png`, `silhouette.png`
- `mesh-stats.json`
- `semantic-parts.json`
- `candidate-report.json`
- `controls-visibility-report.json`
- `export-reopen-report.json`
- `explore-contact-sheet.png`
- `silhouette-contact-sheet.png`
- `gear-contact-sheet.png`
- `hero-pack-report.json`
- `hero-pack-model-package/`
- `model-package/`

## Automated Pass Criteria

The automated gate may report `Usable` only when all of these are true:

- the profile compiles through the normal Foundry path;
- model validation has zero errors;
- six Explore candidates compile, validate, and render non-placeholder previews;
- Explore, Silhouette, and Armor/Gear mode contact sheets are generated;
- exactly seven primary controls are exposed;
- every primary control has a visible whole-model rendered delta;
- the generated pack has three members and each member compiles;
- export and reopen verification are `verified`;
- unsupported outputs are recorded instead of implied.

## Product Boundary

Allowed claim:

- Shape Lab can create a MOBA-quality clay hero family.

Disallowed claims:

- Dota or third-party IP reconstruction.
- Textured, material-authored, UV-authored, rigged, animated, or game-ready
  characters.
- Marketplace-ready packages.
- LLM mesh generation.
- Arbitrary imported mesh editing.
- Photoreal parity with external references.

## Visibility

`moba-hero-clay` remains hidden from the default novice catalog until manual kit
review is approved. Developer/preview catalog visibility is allowed so the
profile can be audited and used as internal release evidence.
