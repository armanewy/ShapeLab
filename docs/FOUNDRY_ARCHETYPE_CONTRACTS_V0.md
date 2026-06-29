# Foundry Archetype Contracts v0

Date: 2026-06-29

Foundry archetypes are internal/pro authoring contracts. They describe reusable
role, provider slot, control axis, candidate strategy, and quality gate
templates for later family/profile work.

Archetypes are not product profiles. No novice user sees archetype internals in
Visual Foundry, and an archetype cannot publish directly to the novice catalog.
The v0 contract has no geometry payloads, no raw vertex payloads, and no runtime
LLM SDK integration.

## v0 Scope

CargoCase is the only v0 archetype. This branch deliberately does not add
SpanStructure, UprightFixture, LinearWeapon, ShieldDisc, ArmorShell, HeroBody,
or any broad archetype library.

Future archetypes require one vertical proof each before they can be considered
for reusable family work. An archetype contract does not imply broad content
generation.

## CargoCase Roles

Required roles:

- body
- lid
- panel_fields
- edge_trim
- corner_guards
- base_feet_or_skids

Optional roles:

- handles
- latches
- vents
- fasteners
- reinforcement_bands
- utility_rails
- side_grilles
- label_plate_geometry
- hinge_or_closure_detail

## CargoCase Controls

CargoCase exposes seven primary control templates for authored profiles:

- Overall Proportions
- Structural Heft
- Panel Complexity
- Handle Style
- Vent Density
- Trim Style
- Detail Density

These are templates for future profiles, not a novice UI commitment by
themselves. Product profiles may expose product-safe versions only after family,
provider, and quality gates pass.

## Candidate Strategies

CargoCase v0 defines these candidate strategy templates:

- Light
- Reinforced
- Compact
- Wide
- Minimal
- Detailed

They are intended to operate through visible controls and declared CargoCase
roles/slots.

## Quality Gates

CargoCase v0 defines these quality gate templates:

- pure_clay_readability
- semantic_clay_readability
- visible_control_endpoints
- visible_candidate_survivors
- no_floating_parts
- export_clean

These gates do not imply UV/texturing, rigging, skinning, animation, a material
editor, or full game-ready status. Archetype contracts do not imply broad
UV/texturing/rigging/animation support.
