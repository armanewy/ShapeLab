# ADR 0013: Semantic Fragment Remapping

## Status

Accepted.

## Context

Executable family compilation merges independently authored `AssetRecipe` fragments. Those fragments contain semantic IDs for definitions, instances, parameters, operations, regions, boundary loops, and sockets. Reusing fragment-local IDs directly would collide in the target recipe, while string replacement over serialized JSON would make provenance and validation fragile.

## Decision

All fragment remapping must pass through typed remap structures in `shape-family-compile::remap`.

`FragmentRemap` owns one explicit map per semantic ID kind:

- `PartDefinitionId`
- `PartInstanceId`
- `ParameterId`
- `OperationId`
- `RegionId`
- `BoundaryLoopId`
- `SocketId`

The module is split by remap concern:

- `ids`
- `operations`
- `assembly`
- `relationships`
- `variation`
- `ports`

The current compiler still supports only the safe first slice of primitive fragments, but unsupported stages now have named module boundaries instead of ad hoc logic. The merge path allocates supported IDs through `remap::ids` and records typed fragment remap reports in the instantiation report.

Recipe fragments export their public contract through `RecipeFragmentExports`: role occurrence roots, internal roots, socket ports, and surface ports. Cross-fragment attachment binding is expressed through exported port IDs and family attachment-rule IDs, not by reaching into another fragment's private IDs.

## Consequences

- Future operations, relationships, variation metadata, and ports have a forced typed remapping path.
- Fragment internals can remain private unless deliberately exported.
- The compiler can audit allocated semantic IDs and remap decisions deterministically.
- Arbitrary textual ID replacement is outside the contract.
