# Offline LLM Draft Policy v0

Date: 2026-07-01

Offline LLMs may help draft structured ObjectPlan JSON outside the app. Shape
Lab does not call an LLM service, add an LLM SDK, or accept plain-text object
descriptions as modeling input.

## Allowed Uses

Offline LLMs may:

- draft ObjectPlan JSON
- suggest primitive presets
- suggest repair actions for failed validation findings
- propose batch asset lists for offline review

## Hard Blocks

Offline LLMs may not:

- generate raw mesh data
- bypass ObjectPlan validation
- invent unsupported primitives, properties, anchors, or capabilities
- publish plans, kits, or presets
- mark plans approved
- claim material, surface, UV, rigging, animation, or game-ready support

Every LLM-authored ObjectPlan remains Draft until a human review or trusted
local workflow explicitly keeps it. Validation is authoritative over the draft
source.

## Accepted Format

ObjectPlan is the only accepted offline LLM object-description format for v0.
Repair responses use ObjectPlanRepairSuggestion JSON. Both formats are bounded
structured data; neither format may contain raw mesh, arbitrary transforms, file
paths, or catalog publishing instructions.
