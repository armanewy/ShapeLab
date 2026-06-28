# HQ Asset Quality Bar

Wave 32 defines the evidence required before Visual Foundry content is treated
as high-quality product content. Compiler validation is necessary, but it is not
sufficient: authored taste, whole-model previews, candidate usefulness, mesh
review, export proof, and human review determine the final tier.

## Quality Tiers

Draft:

- internal only
- hidden from the novice Visual Foundry catalog by default
- may fail visual quality
- must not be described as export-ready product content

Prototype:

- internal or explicitly opt-in only
- hidden from the novice Visual Foundry catalog unless a developer enables it
- must compile and render at least one clay preview
- must not be shown as production-ready

Usable:

- may appear in Visual Foundry after validation and manual review
- must have whole-model clay previews and contact-sheet evidence
- must not require Advanced Recipe for the intended novice task
- must have six surviving direction candidates, or a documented family-specific exception
- must have visible primary-control difference evidence
- must reject candidate directions that are not Clear or Strong unless the user explicitly requested Detail
- must expose product-safe semantic part groups when Focus Part is available
- must reject hidden/internal-only candidate changes through rendered preview delta evidence
- must have export and package-reopen verification

Showcase:

- requires human/pro approval
- requires contact-sheet evidence
- requires no obvious procedural or toy-like artifacts
- requires clean mesh, component, and semantic-part review
- requires export/reopen proof
- requires adversarial visual review

## Global Rules

- No kit ships if primary controls do not visibly matter.
- No kit ships if style/provider combinations create incoherent style mixtures.
- No kit ships if quality evidence uses photoreal screenshots as product truth.
- No kit claims Showcase from automation alone.
- Unsupported outputs are recorded as unsupported instead of fabricated.

Run a benchmark report with:

```bash
cargo run -p shape-cli -- hq-quality-benchmark --profile roman-bridge --out-dir target/hq-benchmark/roman-bridge --verify-export
cargo run -p shape-cli -- hq-quality-benchmark --profile roman-bridge-hq --out-dir target/hq-benchmark/roman-bridge-hq --verify-export
cargo run -p shape-cli -- hq-quality-benchmark --profile fantasy-sword --out-dir target/hq-benchmark/fantasy-sword
cargo run -p shape-cli -- hq-quality-benchmark --profile prepared-hero-template-v1 --out-dir target/hq-benchmark/prepared-hero-template-v1
cargo run -p shape-cli -- hq-quality-benchmark --profile moba-hero-clay --out-dir target/hq-benchmark/moba-hero-clay
```

Use `--profile all --out-dir target/hq-benchmark` to baseline every built-in
profile under separate profile subdirectories. Storybook Tree writes to the
canonical fixture directory `target/hq-benchmark/stylized-tree`; the
`storybook-tree` name is accepted as an input alias.

The automated benchmark can establish that a kit reaches the Usable evidence
bar, but default novice-catalog exposure remains blocked until manual review is
approved. Primary-control evidence is measured with rendered whole-model pixel
deltas, not recipe fingerprints alone.

Focus Part evidence is measured against authored product-facing groups such as
Handles, Deck, Shade, and Fasteners. A focused candidate may ship only when its
changed controls/provider choices are bound to the selected group and the
preview evidence is visible to the user.

Candidate survival means more than receiving six candidate records: each
survivor must compile, pass model validation, and render a non-placeholder
whole-model preview.

Candidate survival also means passing Candidate Legibility Engine v2. Six weak
candidates are not acceptable; fewer honest candidates are acceptable when the
diagnostics separately report duplicate-looking, hidden/internal, and wrong-scope
rejections. Diversity selection happens only after legibility rejection.

Starter-template primary controls must also pass endpoint visibility reporting.
Sci-Fi Crate, Roman Bridge, and Stylized Lamp major controls must be at least
Clear. Edge and softness controls may be SubtleButExplainable only when the
report labels them that way.

Promoted Wave 38 gear benchmarks verify export/reopen by default because their
kit metadata targets the Usable tier. `--verify-export` remains accepted and is
still required for older profiles when making Usable claims.

`roman-bridge-hq` was the first built-in profile authored to satisfy the earlier
Usable automation bar, and refreshed starter-template benchmark evidence now
keeps it eligible for the default novice catalog. Wave 38 extends the HQ bar to
the promoted gear kits: Fantasy Sword, Round Shield, Hero Helmet, Pauldron Pair,
and Chest Armor. They remain hidden from the default novice catalog until manual
review is approved. Adversarial review remains required before Showcase claims.

`prepared-hero-template-v1` is a contract-only prepared hero benchmark. Its
expected automated report is Draft because it has no rendered clay mesh,
contact sheet, direction candidates, or export/reopen evidence yet. It is
included in `--profile all` to keep the unsupported evidence visible, not to
claim novice-catalog readiness.

`moba-hero-clay` is the Wave 40 authored Hero Foundry clay MVP. Its automated
benchmark verifies six surviving directions, seven visible primary-control
deltas, model validation, export/reopen, mode contact sheets, and a three-member
hero pack. It may claim a MOBA-quality clay hero family, not Dota/IP
reconstruction, textured output, materials, UVs, rigging, animation, or
marketplace readiness. Default novice-catalog exposure remains gated by manual
kit review.
