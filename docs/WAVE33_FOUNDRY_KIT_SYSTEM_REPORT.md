# Wave 33 Foundry Kit System Report

Wave 33 added a curated Foundry kit and provider pack system.

## Implemented

- Versioned kit package contracts in `shape-foundry`.
- Family blueprint, provider pack, style pack, control profile, candidate
  strategy pack, quality gate profile, compatibility matrix, review manifest,
  and kit catalog manifest types.
- Validation for schema versions, cross-references, compatibility, required
  roles, provider slots, duplicate visible control ownership, seven-control
  default limit, visibility policy, contact-sheet evidence, Showcase approval,
  and catalog manifest consistency.
- Built-in kit metadata for all ten Visual Foundry profiles in
  `shape-foundry-catalog`.
- CLI commands:
  - `foundry-kit validate`
  - `foundry-kit inspect`
  - `foundry-kit preview`
  - `foundry-kit contact-sheet`
  - `foundry-kit package`
  - `foundry-kit review`
- Product-safe app-side kit card view data with display name, quality badge,
  style name, category chips, review badge, clay-preview badge, and hidden
  policy.
- Expanded product copy gate terms to keep kit authoring internals out of the
  default Visual Foundry UI.

## Boundaries

Wave 33 does not add a visual author studio, marketplace, arbitrary mesh import,
raw vertex injection, LLM SDK/network use, materials, UVs, rigging, animation,
or texture workflows. Kits package curated authored content and review evidence;
the existing exact Foundry compiler remains the geometry source of truth.

## Open Product Questions

- Which built-in Usable kits should receive first human approval for default
  novice exposure?
- Should preview-catalog mode be surfaced to users, kept developer-only, or
  reserved for QA builds?
- What final visual wording should replace "Prototype" in consumer-facing
  builds if preview content is ever exposed?
