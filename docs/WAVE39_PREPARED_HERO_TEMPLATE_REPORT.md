# Wave 39 Prepared Hero Template Report

Wave 39 adds a validated prepared-template foundation for a stylized hero
family.

Implementation summary:

- added `PreparedHeroTemplate` and related hero contract types in
  `shape-character::prepared`;
- added `prepared_hero_template_v1`;
- extended the prepared humanoid known-base contract with leg, wrist, and knee
  bindings needed by hero slots and landmarks;
- validated base fingerprints, topology versions, landmarks, semantic regions,
  cages, weight sets, provider slots, product-safe control labels, unsupported
  operations, and review gates;
- declared future hero provider slots for headgear, shoulders, torso armor,
  belt/skirt, gauntlets, boots, weapon, back accessory, and hair/head mass;
- added the `prepared-hero-template-v1` HQ benchmark profile;
- wrote an honest Draft HQ report path for the prepared template because no
  clay mesh renderer/export path exists yet;
- added focused `shape-character`, `shape-foundry`, and `shape-cli` tests.

The prepared hero profile is not a novice-catalog product asset yet. It is a
validated Prototype contract with Draft HQ benchmark evidence until authored
whole-character clay rendering, candidate generation, contact sheets, and
export/reopen evidence exist.

Required benchmark command:

```bash
cargo run -p shape-cli -- hq-quality-benchmark --profile prepared-hero-template-v1 --out-dir target/hq-benchmark/prepared-hero-template-v1
```

Release note:

This wave does not add arbitrary character import, Dota/IP reconstruction,
materials, UVs, rigging, animation, or marketplace packaging. It creates the
prepared-template contract that later hero foundry work can safely build on.
