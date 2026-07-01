# Offline LLM ObjectPlan Policy

LLMs may draft ObjectPlans offline as structured JSON. They do not run inside
the app, do not call a runtime SDK from Shape Lab, and do not generate meshes.

Every ObjectPlan is validated by Object Orchard before it can be rendered,
saved, or considered for a local kit. The validator is authoritative over the
draft source. `LlmDraft` provenance does not bypass primitive property schemas,
anchor compatibility, bounded offsets, or review-tier rules.

ObjectPlans cannot contain:

- raw mesh data
- arbitrary matrices or raw transforms
- unsupported primitive kinds
- unknown property keys
- material, UV, rigging, animation, or game-ready claims
- absolute paths to local files
- direct public catalog publish requests

LLM-authored plans are Draft until reviewed. A later offline runner may validate
and render contact sheets for several drafts, but accepting a result remains a
local review decision. Invalid plans are rejected or sent back to an offline
repair loop; they are never published automatically.
