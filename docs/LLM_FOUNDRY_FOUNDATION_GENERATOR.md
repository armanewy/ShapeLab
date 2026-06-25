# LLM Foundry Foundation Generator

Wave 36 adds an SDK-free foundation draft system for LLM-assisted Foundry
authoring. It prepares structured kit foundations; it does not generate meshes
or publish content.

## Purpose

The foundation generator uses structured data for throughput:

- family blueprint drafts
- provider slot taxonomy drafts
- style pack drafts
- control profile drafts
- candidate strategy drafts
- compatibility matrix drafts
- quality gate drafts
- test plan drafts
- review checklist drafts
- deterministic repair suggestions
- deterministic adversarial reports

Humans or reviewed procedural/art tooling still supply taste-bearing geometry,
visual variants, contact sheets, and final approval.

## CLI

```bash
cargo run -p shape-cli -- foundry-foundation new --category weapons --family sword --out target/foundation/sword-draft.json
cargo run -p shape-cli -- foundry-foundation validate target/foundation/sword-draft.json
cargo run -p shape-cli -- foundry-foundation materialize target/foundation/sword-draft.json --out-dir target/foundation/sword-kit-draft
cargo run -p shape-cli -- foundry-foundation adversarial-report target/foundation/sword-draft.json --out target/foundation/adversarial-report.json
cargo run -p shape-cli -- foundry-foundation suggest-repair target/foundation/sword-draft.json --validation-report target/foundation/validation.json --out target/foundation/repair.json
```

All commands operate on local JSON files. They do not call an LLM service,
perform network requests, require a model SDK, generate raw vertices, or mutate
recipes directly.

## Defaults

Every new draft defaults to:

- `source_kind = human`
- `quality_target = draft`
- `catalog_visibility = internal_only`
- `human_review_required = true`
- `publish_allowed = false`

Internal fixtures use `source_kind = generated_fixture` and remain
Draft/Internal. Materialized kit packages also remain Draft and hidden from the
default novice catalog and developer preview catalog.

## Author Studio

Foundry Author Studio can show a gated Foundation Draft panel for internal/pro
authors. The default Visual Foundry asset-user workflow does not show these
drafts or their technical authoring terms.
