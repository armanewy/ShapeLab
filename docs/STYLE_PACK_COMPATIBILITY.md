# Style Pack Compatibility

Style packs describe product-facing visual language and compatibility policy for
curated kits. They summarize the style-kit language already present in the exact
Foundry catalog.

## Style Pack Fields

`StylePack` records:

- style ID and display name
- compatible family IDs
- bevel language
- proportion language
- detail-density policy
- silhouette exaggeration policy
- symmetry/asymmetry policy
- allowed and forbidden provider tags
- compatible and incompatible provider pack IDs
- optional metadata-only future material vocabulary

The material vocabulary is intentionally metadata-only. Wave 33 does not add
materials, UVs, texture authoring, rigging, animation, or marketplace workflow.

## Compatibility Matrix

`KitCompatibilityMatrix` records explicit compatible and incompatible
style/provider pairs. Validation rejects incompatible pairs. Default novice
catalogs hide incompatible combinations instead of asking users to understand
the underlying authored provider structure.

## Product Rule

The Visual Foundry product path should present style as whole-model visual
direction and meaningful controls. It should not expose provider pack names,
sockets, ports, family facets, scalar paths, conformance bindings, recipe
fragments, or operation IDs.
