# ADR 0006: Part-Aware Asset Recipes

## Decision

Object Orchard will represent the forward-modeling lane with serializable `AssetRecipe` documents composed of reusable part definitions and concrete part instances.

## Rationale

The implicit MVP graph is useful for field-based exploration, but explicit production assets need stable semantic parts. Part definitions let generators describe reusable local geometry, sockets, regions, and parameter paths. Part instances let the asset express hierarchy, attachments, transforms, enabled state, and operation provenance without requiring a host DCC scene graph.

## Consequences

- Asset recipes are additive and do not replace `shape-core::ShapeDocument`.
- IDs for parts, operations, regions, sockets, parameters, and revisions are strongly typed and ordered.
- Recipe validation can catch dangling definitions, parent cycles, invalid attachments, stale locks, and unresolved parameter paths before compilation.
- Wave 0 stores generic surface roles instead of material names, so later material assignment can remain separate.
