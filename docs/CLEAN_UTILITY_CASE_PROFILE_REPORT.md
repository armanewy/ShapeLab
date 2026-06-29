# Clean Utility Case Profile Report

Date: 2026-06-29

Status: PASS for Prompt 4 automated proof.

Clean Utility Case is implemented as Cargo Case family plus Clean Utility
profile/style defaults. It is not a bespoke family and does not add a hidden
fork of the Cargo Case implementation.

## Architecture

- Family: Cargo Case
- Style/profile: Clean Utility Case
- Shared grammar: body, lid, panel fields, edge trim, corner guards,
  feet/skids, handles, vents, fasteners, and optional detail roles
- Shared controls: Overall Proportions, Structural Heft, Panel Complexity,
  Handle Style, Vent Density, Trim Style, Detail Density
- Shared clay metadata: Pure Clay and Semantic Clay role display mapping

## Clean Utility Defaults

- Panel Complexity: clean panel
- Detail Density: low detail
- Trim Style: clean
- Vent Density: none/sparse
- Handle Style: flush grip
- Corner Guards: minimal cap
- Structural Heft: light/medium

The profile avoids dense sci-fi vents, heavy industrial rails, cargo-bar handles
as its primary default, and heavy industrial bands as its primary default.

## Candidate Strategies

- Light Utility
- Compact Carry Case
- Clean Storage Case
- Reinforced Utility
- Minimal Field Case

Automated candidate generation returns at least four visibly distinct
untextured-clay candidates for the Clean Utility profile.

## Contact Sheet Gate

Generated/documented output location:
`target/foundry-benchmark/clean-utility-case/`

Expected files:

- `parent.png`
- `candidate-contact-sheet.png`
- `control-endpoint-sheet.png`
- `semantic-clay-preview.png`
- `pure-clay-preview.png`
- `quality-report.json`

Gate answers:

- Distinct from Sci-Fi Industrial: PASS. Clean Utility uses sparse vents,
  clean trim, low detail, flush grip handles, and minimal corner guards.
- Same family grammar: PASS. It uses the Cargo Case family, roles, controls,
  and provider slots.
- Avoids sci-fi-specific defaults: PASS.
- Pure clay reads: PASS.
- Semantic clay clarifies structure without hiding weak geometry: PASS.

## Boundaries

No decals, logos, text labels, grime, UVs, texture maps, material maps, material
editor, rigging, animation, runtime LLM integration, or broad Surface mode is
introduced by this profile.
