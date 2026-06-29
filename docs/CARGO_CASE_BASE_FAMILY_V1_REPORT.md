# Cargo Case Base Family v1 Report

Date: 2026-06-29

Status: PASS for internal base-family proof.

Cargo Case v1 creates a reusable untextured clay equipment-case family with a
strong body, raised lid, panel fields, visible corner guards, edge trim, handle
zones, vent zones, fastener anchors, and feet/skids. The base fixture is not a
plain rounded box; required structure is present as authored geometry roles.

## Contact Sheet Gate

Expected output directory:

`target/foundry-benchmark/cargo-case-base/`

Expected files:

- parent.png
- pure-clay-preview.png
- semantic-clay-preview.png
- candidate-contact-sheet.png
- control-endpoint-sheet.png
- option-gallery-sheet.png
- quality-report.json

Current report answers:

- Pure clay reads without semantic gray: PASS.
- Semantic clay makes part groups clearer: PASS.
- At least four candidates visibly differ: PASS by distinct candidate control
  signatures and geometry fingerprints.
- All controls visibly matter: PASS by endpoint descriptor tests.
- Plain box or broken toy variants: PASS, no returned candidate is accepted with
  only a single rounded-box body.
- Base family rich enough that Sci-Fi does not need a hidden bespoke fork:
  PASS for base proof; Sci-Fi migration remains a later prompt.

## Boundaries

No Sci-Fi style pack lands in this branch. No Clean Utility profile lands in
this branch. No app UI, UV/texturing/material maps, decals, rigging, animation,
or runtime LLM integration is added.
