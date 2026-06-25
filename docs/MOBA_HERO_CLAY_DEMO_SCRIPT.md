# MOBA Hero Clay Demo Script

Use this script for the clay-only Hero Foundry MVP. Keep the demo inside the
Visual Foundry product loop: choose, directions, customize, pack, export.

## Setup

Run the benchmark evidence first:

```bash
cargo run -p shape-cli -- hq-quality-benchmark --profile moba-hero-clay --out-dir target/hq-benchmark/moba-hero-clay
```

Confirm `quality-report.json` reports:

- `quality_tier_achieved: usable`
- `candidate_survival_count: 6`
- `primary_control_count: 7`
- `export_status: verified`
- `reopen_status: verified`

## Demo Path

1. Open the developer/preview catalog and choose `Hero Foundry, Clay MVP`.
2. Show the dominant clay preview as the source hero.
3. Generate Directions in Explore mode and show six whole-model candidates.
4. Switch to Silhouette and Armor/Gear mode evidence using the generated contact
   sheets.
5. Customize the hero with the seven primary controls:
   - Hero Archetype
   - Body Proportions
   - Silhouette
   - Armor Mass
   - Head & Face
   - Hair / Headgear
   - Weapon / Accessory
6. Lock one visible trait, regenerate Armor/Gear directions, and confirm the
   locked trait does not change.
7. Add three variants to a pack:
   - Duelist Vanguard
   - Arcane Ranger
   - Monster Hunter
8. Export the pack and show the verified export/reopen evidence.

## Say

- "This is an authored clay hero family."
- "The seven controls change the whole model, not isolated debug parts."
- "The benchmark validates mesh quality, candidate survival, pack compile, and
  export/reopen."

## Do Not Say

- "This reconstructs Dota heroes."
- "This is textured, rigged, animated, or game-ready."
- "This imports and edits arbitrary meshes."
- "This generates geometry from an LLM."
- "This is marketplace-ready."
