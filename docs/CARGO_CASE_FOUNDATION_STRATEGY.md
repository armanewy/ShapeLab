# Cargo Case Foundation Strategy

Date: 2026-06-29

This strategy is the anti-abstraction gate for Foundry architecture work. Cargo
Case proved one rich reusable equipment-case family with two distinct,
high-quality clay profiles: Clean Utility Case and Sci-Fi Industrial Case. That
proof remains scoped to equipment cases only. Do not build a broad archetype
system until another family proof exists beyond Cargo Case and passes its own
clay, catalog, contact-sheet, and human review gates.

This document is a contract and planning gate only. It does not add geometry,
runtime LLM integration, UV/texturing, rigging, animation, or game-ready export
support.

## Layering

### Archetype

An archetype is an internal/pro authoring grammar. It is not a user product and
is not visible in novice Visual Foundry. Archetypes describe reusable role,
slot, control, candidate strategy, and gate contracts for authors and internal
tools.

### Executable Base Family

An executable base family is the structural grammar for a class of assets. It
owns semantic roles and compatible slots, and it is responsible for generating
valid clay geometry through those roles and slots.

### Provider Pack

A provider pack contains authored geometry choices for slots and roles. Provider
packs must pass validation and visual gates before a profile that uses them can
ship to novice catalog tiers.

### Style Pack

A style pack is an art-direction bias over compatible family slots. It can bias
providers, defaults, and detail language, but it must not become a hidden fork of
the family.

### Product Profile

A product profile is what novice users see in the catalog. It combines a base
family, style/profile defaults, visible controls, and candidate strategies into
one curated product entry.

### Candidate Strategy Pack

A candidate strategy pack defines user-facing variation intent. It must operate
through visible controls and provider choices, not through hidden scalar paths,
raw provider IDs, semantic IDs, operation IDs, sockets, ports, fragments,
remaps, or conformance terms in novice UI.

### Quality Gate

A quality gate records the evidence required before a profile reaches the
novice catalog. Contact sheets and human/adversarial review are required before
any profile can be marked Usable or Showcase.

### Catalog Visibility Tier

Catalog visibility tiers decide whether a profile is internal-only, preview,
usable, or showcase. Archetypes and draft authoring outputs are never direct
novice catalog entries.

## Cargo Case Roles

Cargo Case required roles:

- body
- lid
- panel_fields
- edge_trim
- corner_guards
- base_feet_or_skids

Cargo Case optional roles:

- handles
- latches
- vents
- fasteners
- reinforcement_bands
- utility_rails
- side_grilles
- label_plate_geometry
- hinge_or_closure_detail

Cargo Case roles may not depend on decals, text labels, UVs, textures,
materials, image-based details, or logo graphics. Geometry must carry the form.

## Cargo Case Primary Controls

Cargo Case has a maximum of seven primary controls:

- Overall Proportions
- Structural Heft
- Panel Complexity
- Handle Style
- Vent Density
- Trim Style
- Detail Density

Every control must visibly matter in clay preview, own explicit family slots,
and avoid fighting another visible control over the same slot. Topology-changing
controls must be discrete galleries. Continuous controls must preserve topology.
Novice UI must not expose scalar paths, provider IDs, semantic IDs, operation
IDs, sockets, ports, fragment/remap/conformance terms, or other internal
authoring language.

## Proof Requirement

The Cargo Case equipment-family proof is recorded because it produces both:

- Clean Utility Case profile.
- Sci-Fi Industrial Case profile.

Both profiles must be generated from the same Cargo Case family. They must
share role grammar, control vocabulary, candidate strategy templates, semantic
part groups, pure clay preview mode, and semantic clay preview mode.

They may differ by style/provider preferences, default control values, candidate
strategy names, detail-density bias, trim/vent/handle provider choices, and
catalog tags or descriptions.

Sci-Fi Industrial Crate is Cargo Case family + Sci-Fi Industrial style/profile,
not a bespoke one-off family.

## Existing Sci-Fi Crate Compatibility

The existing Sci-Fi Industrial Crate product profile and its material-look
preview evidence must be preserved during migration. The Cargo Case proof must
not break:

- profile ID `sci-fi-crate`, unless an explicit migration is implemented.
- the existing Make dogfood baseline.
- the existing material-look preview baseline.
- the existing static surface package command.
- product copy that material looks are preview-only unless persistence lands
  later.
- current game-ready blockers.

If Cargo Case migration changes the frozen mesh fingerprint, existing
material-look evidence must be marked stale and regenerated. It must not be
silently reused against changed geometry.

Prompt 5 implementation note: `sci-fi-crate` now routes through Cargo Case
family plus Sci-Fi Industrial style/profile defaults. Existing material-look
preview evidence for the older bespoke geometry is stale unless regenerated
against the Cargo Case output.

## LLM And Internal Author Policy

Internal agents may draft archetype, family, style, and provider specs. They may
propose repairs, generate QA checklists, and write contact-sheet prompts.

Internal agents may not publish to the novice catalog. They may not inject raw
mesh or vertex payloads, bypass validation, or mark anything Usable or Showcase
without human/adversarial review.

Policy: Direct novice catalog publication from drafts is forbidden.

No runtime LLM SDK is required for this work.
