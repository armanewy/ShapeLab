# Weapon And Armor Foundation Batch

Wave 37 adds a deterministic internal batch of weapon and armor Foundry
foundation drafts. These drafts are structured authoring backlogs, not final
content.

## Boundary

- Quality target: Draft
- Catalog visibility: internal_only
- Human review: required
- Publishing: disabled
- Geometry: not included
- Raw vertex payloads: rejected by the Wave 36 schema
- Runtime LLM calls: none

The default Visual Foundry catalog is not populated from this batch. Drafts
must be pruned, authored, contact-sheeted, validated, and manually reviewed
before any curated kit can be considered for novice exposure.

## Families

Weapons:

- sword
- dagger
- axe
- mace_hammer
- spear_polearm
- bow_crossbow
- staff_wand
- shield
- scifi_rifle_blaster
- grenade_device_prop

Armor and gear:

- helmet
- pauldron
- chest_armor
- gauntlet
- boot
- belt
- cape_back_accessory
- mask
- hero_accessory_set

## Draft Contents

Every draft includes:

- required and optional part roles
- attachment expectations between parts
- provider slots and at least five provider option names
- seven or fewer primary controls with human labels
- six candidate direction strategies
- compatibility notes for Rustic Medieval, Clean Sci-Fi, Ancient Ruin,
  Stylized Cozy, Dark Fantasy, Toylike Low-Poly, Industrial Heavy, Elegant
  Elven, Brutalist Stone, and MOBA Heroic
- quality-gate checklist rows for required roles, attachment failures,
  self-intersections, triangle budgets, 128px silhouettes, visible controls,
  six candidate survivors, whole-model previews, export/reopen, contact sheets,
  and human review

## CLI

Export the deterministic batch:

```bash
cargo run -p shape-cli -- foundry-foundation batch --out-dir target/foundation/wave37
```

The command writes:

- `drafts/*.foundation-draft.json`
- `validation/*.validation.json`
- `adversarial/*.adversarial-report.json`
- `foundation-batch-summary.json`

Representative validation:

```bash
cargo run -p shape-cli -- foundry-foundation validate target/foundation/wave37/drafts/sword.foundation-draft.json
cargo run -p shape-cli -- foundry-foundation adversarial-report target/foundation/wave37/drafts/sword.foundation-draft.json --out target/foundation/adversarial.json
```
