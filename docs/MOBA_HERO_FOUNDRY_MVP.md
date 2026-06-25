# MOBA Hero Foundry MVP

Wave 40 adds `moba-hero-clay`, an authored Visual Foundry profile for clay-only
hero family generation. It is sourced from the Wave 39 prepared hero template
contract and implemented as normal Foundry catalog content.

## Scope

The MVP proves that Shape Lab can generate a coherent MOBA-quality clay hero
family with whole-model controls, directions, pack evidence, and export/reopen
verification.

It does not add materials, UVs, rigging, animation, marketplace packaging,
LLM-generated geometry, Dota reconstruction, or arbitrary mesh editing.

## Primary Controls

The profile exposes exactly seven novice-facing controls:

- Hero Archetype
- Body Proportions
- Silhouette
- Armor Mass
- Head & Face
- Hair / Headgear
- Weapon / Accessory

Choice groups have at least five whole-model options for body/archetype,
head/face, hair/headgear, armor, and weapon/accessory.

## Directions

The authored whole-character direction set is:

- Armored Duelist
- Arcane Ranger
- Brutal Champion
- Agile Assassin
- Ceremonial Guardian
- Monster Hunter

The profile also exposes Explore, Silhouette, Armor/Gear, and Detail candidate
strategies. The Wave 40 benchmark requires six surviving Explore candidates and
separate mode contact sheets for Explore, Silhouette, and Armor/Gear.

## Provider Coverage

The profile includes clay provider fragments for:

- body and proportions;
- head and face;
- hair and headgear;
- shoulders;
- torso armor;
- belt/skirt;
- gauntlets;
- boots;
- weapon;
- back accessory;
- small details.

The authored parts are simple clay geometry with explicit spacing so generated
variants pass model validation without accidental intersections.

## Evidence Artifacts

The canonical evidence directory is:

```text
target/hq-benchmark/moba-hero-clay
```

`hero-pack-report.json` records the prepared source template ID, prepared base
library fingerprint, shared controls, three pack members, semantic part
inventory, conformance status, pack report fingerprint, verified member export
count, and export/reopen state.

`hero-pack-model-package/` records the compiled pack document, the pack compile
report, and one verified canonical model package for each hero pack member.
