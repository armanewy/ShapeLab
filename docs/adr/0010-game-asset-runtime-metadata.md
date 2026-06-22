# ADR 0010: Game Asset Runtime Metadata

## Status

Accepted

## Context

Project Caesar needs more than triangle meshes. Runtime modules need footprints, anchors, support surfaces, walkable surfaces, construction states, collision proxies, readability requirements, and triangle budgets. Those contracts are also broadly useful to other games, so they should not be embedded directly in a Project Caesar-only format.

Shape Lab must avoid encoding gameplay balance values such as cost, labor, damage, movement bonuses, road bonuses, or AI behavior. Those values are owned by the game runtime and scenario code.

## Decision

Add `shape-gamekit` as a runtime-neutral metadata crate. It defines serializable contracts for `GameAssetPack`, `GameAssetDefinition`, `ModuleSemantics`, `LogicalFootprint`, `SnapAnchor`, `SupportSurface`, `WalkableSurface`, `TraversalLink`, `CollisionProxy`, `ConstructionProfile`, `ReadabilityProfile`, and `TriangleBudget`.

Add `shape-caesar-assets` as the Project Caesar authored pack crate. It can contain Roman/Gallic names and River Bend runtime keys, while depending on the generic `shape-gamekit` contracts.

## Consequences

- The game-facing metadata can be validated before export.
- Later export, readability, construction-state, runtime-semantics, Godot, and LOD work can build on a stable contract layer.
- Project Caesar content can evolve without contaminating generic Shape Lab crates with game-specific names.
- Shape Lab remains responsible for geometry and semantic structure, not runtime game balance.
