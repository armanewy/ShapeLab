# LLM Authoring Boundary

Shape Lab may use LLM-assisted structure drafting, but Wave 36 does not add LLM
runtime integration.

## Allowed

- Draft family roles and provider slots.
- Draft style compatibility policy.
- Draft novice control labels and descriptions.
- Draft candidate strategy names and control-space intent.
- Draft quality gates, test plans, review checklists, and repair suggestions.
- Produce deterministic JSON that humans and local validators can review.

## Forbidden

- External LLM SDK dependency in core crates.
- Network calls.
- Raw mesh generation.
- Raw vertex position generation.
- Hidden mesh payloads.
- Direct recipe mutation.
- Validation bypass.
- Silent topology fixes.
- Unbounded random variants.
- Automatic publishing to the novice catalog.
- Photoreal/material/UV/rigging/animation work.
- Arbitrary imported-mesh editability.

The command contract intentionally omits forbidden operations such as
`SetRawVertexPositions`, `InjectMeshPayload`, `BypassValidation`,
`PublishToNoviceCatalog`, `MutateRecipeDirectly`, and
`HideValidationFailure`. Such commands cannot deserialize into the accepted
authoring command enum.

## Review Rule

LLMs can draft foundations, not final taste. A draft cannot become default
novice content until validation, contact sheets where required, authored
geometry, adversarial review where required, and human approval are complete.
