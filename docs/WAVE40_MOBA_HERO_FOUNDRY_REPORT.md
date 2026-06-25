# Wave 40 MOBA Hero Foundry Report

Wave 40 adds the `moba-hero-clay` Visual Foundry profile as a clay-only Hero
Foundry MVP.

## Implementation Summary

- Added `shape_foundry_catalog::moba_hero` with the `moba-hero-clay` fixture.
- Registered `Hero Foundry, Clay MVP` as the seventeenth built-in Foundry kit.
- Kept the kit hidden from the default novice catalog and visible only in
  developer/preview mode until manual review.
- Sourced the profile from `prepared-hero-template-v1`.
- Exposed exactly seven primary controls:
  - Hero Archetype
  - Body Proportions
  - Silhouette
  - Armor Mass
  - Head & Face
  - Hair / Headgear
  - Weapon / Accessory
- Added provider fragments for body, head/face, hair/headgear, shoulders, torso
  armor, belt/skirt, gauntlets, boots, weapon, back accessory, and small detail.
- Added six named whole-character directions:
  - Armored Duelist
  - Arcane Ranger
  - Brutal Champion
  - Agile Assassin
  - Ceremonial Guardian
  - Monster Hunter
- Added Explore, Silhouette, Armor/Gear, and Detail candidate strategies.
- Added Wave 40 benchmark evidence for mode contact sheets and a three-member
  hero pack.

## Benchmark Result

Command:

```bash
cargo run -p shape-cli -- hq-quality-benchmark --profile moba-hero-clay --out-dir target/hq-benchmark/moba-hero-clay
```

Result:

- requested tier: `Usable`
- achieved tier: `Usable`
- blockers: `0`
- primary controls: `7`
- visible control deltas: `7`
- candidate survival: `6`
- six-direction availability: `true`
- export status: `verified`
- reopen status: `verified`
- triangle count: `9996`
- semantic part count: `17`
- novice catalog exposure by default: `false`

The model validation summary reports zero errors, zero warnings, full
provenance coverage, closed-manifold part fraction `1.0`, and zero accidental
intersections.

## Pack Evidence

`hero-pack-report.json` records:

- schema version: `2`
- pack ID: `moba-hero-clay-demo-pack`
- source template: `prepared-hero-template-v1`
- pack report fingerprint: recorded
- shared style: `MOBA Heroic Clay`
- members: `3`
- total triangle count: `29988`
- semantic part inventory count: `51`
- exported member package count: `3`
- export status: `verified`
- reopen status: `verified`

Pack members:

- Duelist Vanguard, main, `9996` triangles, `17` semantic parts
- Arcane Ranger, variant, `9996` triangles, `17` semantic parts
- Monster Hunter, variant, `9996` triangles, `17` semantic parts

## Generated Artifacts

The benchmark writes:

- `back.png`
- `candidate-report.json`
- `contact-sheet.png`
- `controls-visibility-report.json`
- `explore-contact-sheet.png`
- `export-reopen-report.json`
- `front.png`
- `gear-contact-sheet.png`
- `hero-pack-report.json`
- `hero-pack-model-package/`
- `hero-pack-model-package/pack-document.json`
- `hero-pack-model-package/pack-report.json`
- `mesh-stats.json`
- `model-package/`
- `quality-report.json`
- `semantic-parts.json`
- `side.png`
- `silhouette-contact-sheet.png`
- `silhouette.png`
- `three-quarter.png`
- `wireframe.png`

## Boundary

This wave supports the claim:

- Shape Lab can create a MOBA-quality clay hero family.

It does not support claims about Dota/IP reconstruction, textured output,
materials, UVs, rigging, animation, marketplace packaging, LLM mesh generation,
or arbitrary mesh editing.
