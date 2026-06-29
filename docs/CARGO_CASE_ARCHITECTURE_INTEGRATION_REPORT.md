# Cargo Case Architecture Integration Report

Date: 2026-06-29

## Decision

`PASS`

Cargo Case is the first proven reusable family architecture in Shape Lab, scoped
to clay equipment cases only. The proof covers one executable base family, two
distinct product profiles, shared controls/roles/semantic groups, and current
Sci-Fi Crate compatibility boundaries.

This does not prove a broad archetype system. Future archetypes still require
their own vertical proof before novice catalog exposure. After the family
foundation pivot, Simple Crate is the next flagship family-authoring proof;
Cargo Case remains an equipment-case proof and Sci-Fi Crate remains a
regression/advanced profile, not the flagship.

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
| Does existing Sci-Fi Crate material-look preview remain valid or get honestly regenerated/disabled? | See material-look compatibility below. |
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
- The Sci-Fi Crate Make dogfood baseline remains approved as regression
  evidence, not as the flagship product proof.
- Simple Crate is the next flagship family-authoring proof.
- Cargo Case remains valid but scoped to equipment cases only.
- Material looks remain preview-only unless a later scoped branch adds
  persistence/export inclusion.
- Static surface package generation is preserved, but full game-ready status
  remains blocked.
- Roman Bridge remains `PreviewOnly`.
- No broad Surface, UV/Texturing, Rigging, skinning, animation, or game-ready UI
  claim is added.

## Material-Look Compatibility

The Cargo Case migration changes the geometry source for `sci-fi-crate`, so old
material-look evidence is stale. Current evidence must be regenerated against
the Cargo Case output before the preview UI is enabled.

The stabilization pass regenerated default Sci-Fi Crate surface evidence at
`target/surface-candidate-evidence-v0/sci-fi-crate`. During release-app dogfood,
after applying a new idea, the app honestly disabled material looks with
`Material looks do not match this crate build` instead of reusing stale evidence.

## Draft Materializer

The optional Archetype Draft Materializer v0 is included because the Cargo Case
proof is strong. It supports Cargo Case only and writes internal foundation
drafts for author review. It does not publish profiles, emit mesh payloads,
inject raw vertices, bypass validation, or require a runtime LLM SDK.

## Next Step

Allowed next work:

- Simple Crate Primitive v0;
- Simple Crate Make baseline;
- Utility Crate v1;
- Cargo Case ladder reconciliation;
- Sci-Fi Crate regression and material-look compatibility checks only when
  needed.

Do not use this proof to start broad archetype expansion, broad Surface mode,
UV/Texturing, rigging, skinning, or animation UI work.
