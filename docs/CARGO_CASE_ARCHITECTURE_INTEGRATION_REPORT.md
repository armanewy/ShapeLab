# Cargo Case Architecture Integration Report

Date: 2026-06-29

## Decision

`PASS`

Cargo Case is the first proven reusable family architecture in Shape Lab, scoped
to clay equipment cases. The proof covers one executable base family, two
distinct product profiles, shared controls/roles/semantic groups, and current
Sci-Fi Crate compatibility boundaries.

This does not prove a broad archetype system. Future archetypes still require
their own vertical proof before novice catalog exposure.

## Proof Answers

| Question | Answer |
| --- | --- |
| Is Sci-Fi Industrial Crate built from Cargo Case? | Yes. The stable `sci-fi-crate` profile routes through the Cargo Case family plus Sci-Fi Industrial profile defaults. |
| Is Clean Utility Case built from Cargo Case? | Yes. It uses the same Cargo Case family with Clean Utility defaults. |
| Do both profiles share role grammar? | Yes: body, lid, panel fields, edge trim, corner guards, skids/feet, handles, vents, fasteners, and related optional roles. |
| Do both profiles share control vocabulary? | Yes: Overall Proportions, Structural Heft, Panel Complexity, Handle Style, Vent Density, Trim Style, Detail Density. |
| Do both profiles share semantic part groups? | Yes. Both use the Cargo Case part-group/role map and semantic clay assignments. |
| Do both profiles have distinct default looks? | Yes. Clean Utility is sparse and practical; Sci-Fi Industrial is denser, vented, reinforced, and rail-heavy. |
| Do both profiles produce at least four visible candidates? | Yes, per the Cargo Case catalog tests and benchmark reports. |
| Does semantic clay help readability without UV/texturing? | Yes. Semantic clay is neutral gray viewport display metadata only. |
| Does pure clay still read? | Yes. Pure Clay pass/fail remains separate from Semantic Clay readability. |
| Is there any hidden bespoke sci-fi fork? | No. `scifi_crate.rs` is a compatibility shim over Cargo Case. |
| Are draft materialized families hidden from novice catalog? | Yes. Materialized drafts are `publish_allowed=false`, `novice_visible=false`, `human_review_required=true`. |
| Does existing Sci-Fi Crate material-look preview remain valid or get honestly regenerated/disabled? | The Cargo Case migration changes the geometry source, so old evidence is stale. Current evidence must be regenerated against the Cargo Case output before the preview UI is enabled. |
| Does static surface package generation remain blocked from full game-ready? | Yes. The static package still reports full game-ready blocked pending manual/runtime proof. |

## Contact Sheet Gate

Generated/expected integration artifacts:

- `target/cargo-case-architecture/cargo-case-base/`
- `target/cargo-case-architecture/clean-utility-case/`
- `target/cargo-case-architecture/scifi-industrial-cargo-case/`
- `target/cargo-case-architecture/comparison-clean-vs-scifi.png`
- `target/cargo-case-architecture/pure-vs-semantic-clay.png`
- `target/cargo-case-architecture/surface-compatibility-report.json`

Human gate result:

- Clean Utility and Sci-Fi Industrial look meaningfully different.
- Both preserve the same equipment-case family grammar.
- Sci-Fi reads industrial in untextured clay, without decals or texture maps.
- Clean Utility avoids sci-fi-specific dense vents, rails, and heavy bands.
- Base Cargo Case is richer than a plain rounded box with attachments.
- Semantic Clay improves part readability.
- Pure Clay still reads without semantic gray assistance.
- No profile is recorded as a broken procedural toy.

## Existing Product Baseline

- The stable `sci-fi-crate` profile ID is preserved.
- The Sci-Fi Crate Make dogfood baseline remains the narrow approved product
  baseline.
- Material looks remain preview-only unless a later scoped branch adds
  persistence/export inclusion.
- Static surface package generation is preserved, but full game-ready status
  remains blocked.
- Roman Bridge remains `PreviewOnly`.
- No broad Surface, UV/Texturing, Rigging, animation, or game-ready UI claim is
  added.

## Draft Materializer

The optional Archetype Draft Materializer v0 is included because the Cargo Case
proof is strong. It supports Cargo Case only and writes internal foundation
drafts for author review. It does not publish profiles, emit mesh payloads,
inject raw vertices, bypass validation, or require a runtime LLM SDK.

## Next Step

Allowed next work:

- one more Cargo Case profile, to further test family reuse;
- Stylized Lamp product dogfood pass;
- Sci-Fi Crate material persistence/export inclusion only if explicitly scoped.

Do not use this proof to start broad archetype, UV/Texturing, rigging, or
animation UI work.
