# Archetype Draft Materializer v0

## Status

Archetype Draft Materializer v0 is an internal/pro authoring scaffold for Cargo Case foundation drafts. It is not final product content, not a novice catalog feature, and not a broad archetype-generation system.

The v0 command supports the Cargo Case archetype only:

```bash
shape-cli foundry materialize-archetype \
  --archetype cargo-case \
  --family-id clean-medical-case \
  --style-id clean-medical \
  --out-dir target/foundry-archetype-drafts/clean-medical-case
```

The command writes:

- `family-blueprint-draft.json`
- `provider-taxonomy-draft.json`
- `style-pack-draft.json`
- `control-profile-draft.json`
- `candidate-strategy-draft.json`
- `quality-gate-draft.json`
- `test-plan-draft.json`
- `review-checklist.md`
- `materialization-report.json`

## Draft Policy

Generated drafts are foundation scaffolding only:

- `publish_allowed` is false.
- `novice_visible` is false.
- `human_review_required` is true.
- Showcase promotion is not allowed from the generated draft.
- No geometry payloads are emitted.
- No raw vertices are emitted.
- No direct recipe mutation is emitted.
- Validation cannot be bypassed.

The materializer may produce starting drafts for:

- Clean Medical Case
- Rugged Field Case
- Industrial Storage Case

These are structured drafts only. They do not implement final geometry, provider art, contact sheets, or catalog-ready product profiles.

## LLM And Agent Boundary

No runtime LLM SDK is required for this work. Future LLM-assisted authoring agents may call this CLI through typed commands, but the output remains an internal draft. Agents may draft structure and checklists; they may not publish to the novice catalog, inject raw mesh data, bypass validation, or mark a profile Usable/Showcase.

Promotion requires authored or reviewed providers, validation, contact sheets, Pure Clay and Semantic Clay review, and human/adversarial approval before any product profile becomes visible to novice users.
