# Wave 37 Foundation Batch Report

Wave 37 uses the SDK-free foundation draft system from Wave 36 to create a
reviewed internal backlog for weapon and armor kits. It does not add final
showcase gear, hero foundry behavior, mesh generation, raw vertex injection,
materials, UVs, rigging, animation, imports, LLM SDKs, or novice catalog
publication.

## Implementation

- Added `shape_foundry::foundation_batch`.
- Added `weapon_armor_foundation_draft_batch()` with 19 Draft/Internal
  foundations.
- Added `weapon_armor_foundation_batch_summary()` for deterministic review
  categorization.
- Added `shape-cli foundry-foundation batch --out-dir <dir>` to write draft,
  validation, adversarial, and summary JSON artifacts.
- Added tests for internal-only visibility, publish blocking, human review,
  structural validation, primary-control limits, style matrices, quality gates,
  candidate strategies, default catalog exclusion, and deterministic
  adversarial reports.

## Review Categories

Promising:

- sword
- dagger
- axe
- shield
- grenade_device_prop
- helmet
- pauldron
- belt

Needs simplification:

- spear_polearm
- bow_crossbow
- boot

Over-abstracted:

- chest_armor
- cape_back_accessory
- hero_accessory_set

Missing art ingredients:

- staff_wand
- gauntlet

High risk of style salad:

- mace_hammer
- scifi_rifle_blaster
- mask

Good candidates for a later curated gear pack:

- sword
- dagger
- axe
- shield
- grenade_device_prop
- helmet
- pauldron
- belt

## Validation Evidence

Representative commands:

```bash
cargo run -p shape-cli -- foundry-foundation batch --out-dir target/foundation/wave37
cargo run -p shape-cli -- foundry-foundation validate target/foundation/wave37/drafts/sword.foundation-draft.json
cargo run -p shape-cli -- foundry-foundation adversarial-report target/foundation/wave37/drafts/sword.foundation-draft.json --out target/foundation/adversarial.json
```

Full verification for the committed milestone should include:

```bash
cargo fmt --all --check
cargo test -p shape-cli foundry_foundation
cargo test -p shape-foundry foundation_batch
cargo test -p shape-app wave37_foundation
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace --no-fail-fast
cargo build --release --workspace
```

## Boundary Confirmation

- All drafts use `quality_target = draft`.
- All drafts use `catalog_visibility = internal_only`.
- All drafts require human review.
- All drafts have `publish_allowed = false`.
- The default Visual Foundry kit cards are still sourced only from curated
  built-in packages.
- Style incompatibilities are recorded with reasons and remain hidden from
  novice surfaces until reviewed curation exists.
