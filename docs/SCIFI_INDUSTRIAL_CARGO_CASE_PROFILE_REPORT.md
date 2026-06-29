# Sci-Fi Industrial Cargo Case Profile Report

Date: 2026-06-29

Status: PASS as Sci-Fi Crate regression and Cargo Case compatibility proof.
This is not a flagship product proof.

Sci-Fi Industrial Crate is implemented as Cargo Case family plus Sci-Fi
Industrial style/profile defaults. The public `sci-fi-crate` profile ID is
preserved as a compatibility slug.

Sci-Fi Crate is an advanced regression profile. Simple Crate is the new novice
baseline proof once implemented and baseline-passed. Cargo Case is the advanced
equipment-case proof.

## Reuse Proof

- Clean Utility Case and Sci-Fi Industrial Crate share the Cargo Case family.
- Both profiles share the same role grammar.
- Both profiles share the same control vocabulary.
- Both profiles share the same semantic part groups and clay display metadata.
- Defaults differ: Clean Utility is sparse/clean/light; Sci-Fi Industrial is
  dense/reinforced/heavier.
- Provider preferences differ within declared Cargo Case slots.
- Sci-Fi Industrial is not a hidden bespoke fork.

## Sci-Fi Defaults

- Panel Complexity: deep framed panel
- Detail Density: high detail
- Vent Density: dense grille
- Handle Style: side rail
- Trim Style: reinforced edge trim
- Corner Guards: chamfered armor block
- Structural Heft: medium, with heavier identity carried by reinforced providers

## Candidate Strategies

- Light Industrial
- Reinforced Cargo
- Compact Vented
- Wide Equipment Case
- Minimal Industrial
- Detailed Utility Case

Automated candidate generation returns at least four visibly distinct
untextured-clay candidates.

## Contact Sheet Gate

Generated/documented output location:
`target/foundry-benchmark/scifi-industrial-cargo-case/`

Expected files:

- `parent.png`
- `pure-clay-preview.png`
- `semantic-clay-preview.png`
- `candidate-contact-sheet.png`
- `control-endpoint-sheet.png`
- `comparison-to-clean-utility.png`
- `quality-report.json`
- `surface-compatibility-report.json`

Gate answers:

- Clean Utility and Sci-Fi Industrial share family grammar: PASS.
- Defaults are meaningfully different: PASS.
- Sci-Fi reads industrial without textures or decals: PASS.
- Pure clay reads: PASS.
- Semantic clay clarifies structure without hiding weak geometry: PASS.
- No profile looks like a broken procedural toy: PASS.

## Surface Compatibility

Material-look preview evidence is preview-only. The Cargo Case migration changes
the geometry source for `sci-fi-crate`; any existing material-look preview
evidence tied to the older frozen mesh fingerprint is stale until regenerated.
Stale evidence must not enable material-look UI or be silently reused.

The stabilization pass regenerated default surface evidence at
`target/surface-candidate-evidence-v0/sci-fi-crate`. Release-app dogfood also
confirmed that material looks are disabled after an idea changes the crate build,
with copy stating that the material looks do not match the current crate build.

The static surface package command remains supported for the narrow static prop
path:

```bash
cargo run -p shape-cli -- game-ready-static-prop --profile sci-fi-crate --out-dir target/game-ready/sci-fi-crate-static-prop-v1
```

Static surface packaging remains blocked from any full game-ready claim.

## Catalog Visibility

Sci-Fi Crate may remain visible only while current dogfood status says the
advanced regression baseline is non-regressed. If the Make baseline, material
preview gating, static package command, or full-ready blocker truth regresses,
the profile should move to preview/developer catalog visibility until fixed.

When Simple Crate exists and passes its baseline, Simple Crate should become the
first/featured novice crate. This branch does not add Simple Crate catalog
metadata because Simple Crate is not implemented here.

## Boundaries

No UV/texturing support, material editor, decals, labels, rigging, animation,
runtime LLM integration, or broad Surface mode is introduced by this migration.
Full game-ready remains blocked.
