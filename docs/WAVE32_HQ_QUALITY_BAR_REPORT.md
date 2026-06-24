# Wave 32 HQ Quality Bar Report

Wave 32 adds a headless quality benchmark and tier gate for Visual Foundry
content. It does not add a hero foundry, new catalog families, LLM integration,
materials, UVs, rigging, animation, marketplace work, or a GPU renderer.

## Tier Meanings

Draft is internal-only. Prototype compiles and renders but remains hidden unless
explicitly enabled. Usable requires contact sheets, visible primary-control
differences, six surviving directions or an approved exception, no Advanced
Recipe dependency, and export/reopen evidence. Showcase requires Usable
evidence plus human approval and adversarial visual review.

## Automatic Evidence

`shape-cli hq-quality-benchmark` records:

- clay views and contact sheet paths
- wireframe and silhouette images
- mesh validity summary
- triangle count and budget
- semantic part inventory
- required role coverage
- provider/attachment validity
- candidate survival count
- visible primary-control difference evidence measured by rendered whole-model
  pixel deltas
- export/reopen status when `--verify-export` is used
- unsupported outputs such as UVs, materials, textures, rigging, animation, and photoreal output

## Human Evidence

Silhouette readability, aesthetic quality, procedural artifacts, style
coherence, and Showcase approval remain manual review tasks. Automation cannot
mark Showcase unless a reviewer supplies approval and adversarial review
markers.

## Benchmark Outputs

For one profile:

```bash
cargo run -p shape-cli -- hq-quality-benchmark --profile roman-bridge --out-dir target/hq-benchmark/roman-bridge --verify-export
```

For all built-ins:

```bash
cargo run -p shape-cli -- hq-quality-benchmark --profile all --out-dir target/hq-benchmark --verify-export
```

Each profile directory contains `quality-report.json`, contact-sheet and view
PNGs, `mesh-stats.json`, `semantic-parts.json`, `candidate-report.json`,
`controls-visibility-report.json`, and `export-reopen-report.json`.

## Baseline Profiles

The benchmark enumerates the ten built-in Visual Foundry profiles:

| Profile | Automated Tier With `--verify-export` | Candidate Survivors | Blocker |
| --- | --- | ---: | --- |
| roman-bridge | Prototype | 4 | Needs six validated/rendered direction survivors or an approved exception |
| sci-fi-crate | Usable | 6 | None |
| stylized-lamp | Usable | 6 | None |
| market-stall | Prototype | 5 | Needs six validated/rendered direction survivors or an approved exception |
| sci-fi-door | Usable | 6 | None |
| storage-barrel | Prototype | 3 | Needs six validated/rendered direction survivors or an approved exception |
| signpost | Usable | 6 | None |
| workshop-chair | Prototype | 5 | Needs six validated/rendered direction survivors or an approved exception |
| handcart | Prototype | 5 | Needs six validated/rendered direction survivors or an approved exception |
| stylized-tree | Prototype | 5 | Needs six validated/rendered direction survivors or an approved exception |

The `storybook-tree` profile name is accepted as an alias for `stylized-tree`.
`--profile all` writes the canonical fixture output directory
`target/hq-benchmark/stylized-tree`.

All ten compile, render views, record contact sheets, record visible
primary-control pixel deltas for seven controls, and verify export/reopen when
`--verify-export` is used. Four profiles currently reach automated Usable.
Six remain Prototype until candidate generation produces six validated/rendered
survivors or a reviewer approves a family-specific exception. None receive
default novice catalog exposure without human review approval.

## Unsupported Outputs

The report explicitly records photoreal renders, UV layout, materials,
textures, rigging, animation, and marketplace packages as unsupported. These
entries prevent quality reports from implying output capabilities that do not
exist.
