# Asset-Family Foundry

Shape Lab's product direction is a general asset-family compiler. Project Caesar is the first serious dogfood customer, not the identity of the modeling engine.

The core architecture is split into three reusable layers.

## Asset Family

An asset family is theme-neutral functional structure. It defines what kind of object is being generated before any visual style is chosen.

Examples:

- `bridge`: supports, spans, deck modules, braces, ramps, connectors.
- `crate`: body, panels, handles, corner protection, fasteners, vents.
- `lamp`: base, stem, joints, shade, bracket.
- `wall_gate_tower`: wall runs, gates, towers, platforms, continuation sockets.

Family documents contain:

- part roles
- attachment rules
- allowed modeling operation classes
- parameter slots and valid ranges
- variant-generation rules
- geometric constraints
- export requirements
- compatible style-kit IDs

The family layer must not contain Roman, sci-fi, furniture, concrete marketplace policy, or game-balance assumptions.

## Style Kit

A style kit provides concrete shape language for one or more compatible families.

Examples:

- Roman timber engineering
- sci-fi industrial steel
- stylized fantasy wood
- elegant art-deco metal
- toy-like plastic

Style kits contain:

- role proportion guidance
- bevel policy
- taper and profile language
- part prototypes
- ornament and detail modules
- repetition density
- symmetry preferences
- silhouette and detail exaggeration

Materials can later attach to this layer, but geometry style exists before texturing.

## Runtime Or Export Profile

Runtime and export profiles describe destination-specific metadata. This is optional and adapter-owned.

Examples:

- Project Caesar: logical footprint, snap anchors, walkable surfaces, construction stages.
- Marketplace: pivot, collision proxy, LOD, preview requirements.
- Godot: scene structure and import conventions.
- Unity or Unreal: prefab or actor conventions.

A decorative lamp should not carry walkable-surface metadata unless a runtime profile asks for it.

## Architectural Rules

- Core crates do not contain Roman or Caesar-specific concepts.
- Caesar-authored assets live in `shape-caesar-assets` and future content-pack locations.
- Game/runtime contracts live in adapter-oriented crates such as `shape-gamekit`.
- Adding a new family should normally be data and recipes, not engine changes.
- A new modeling operator should serve at least two unrelated families before it is promoted into the general engine.
- Gameplay balance remains outside Shape Lab.

## Development Test

The near-term generality test is not "support all 3D objects." It is support several hard-surface and modular families with different styles:

- Roman field engineering for Project Caesar.
- Sci-fi industrial props.
- Stylized furniture and lighting.

If a missing operation improves two or three of those packs, it likely belongs in the engine. If it only helps the Roman pack, it should be scrutinized as content-pack logic or style-kit data first.
