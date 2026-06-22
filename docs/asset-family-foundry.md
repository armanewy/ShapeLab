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
- parameter slots, valid ranges, and semantic default values
- variant-generation rules
- geometric constraints
- export requirements
- compatible style-kit IDs

The family layer must not contain Roman, sci-fi, furniture, concrete marketplace policy, or game-balance assumptions.

Family schemas are descriptive contracts. They do not directly contain Shape Lab recipe fragments or generator code.

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

Style kits are also descriptive contracts. A `PartPrototype` names a compatible role and expected operation vocabulary; it is not itself executable geometry.

## Executable Bindings

Executable family compilation lives in `shape-family-compile`, not in `shape-family`.

That crate binds:

- an `AssetFamilySchema`
- a compatible `StyleKit`
- versioned `FamilyImplementation`, `StyleImplementation`, and `RecipeFragment` documents
- family-owned default recipe fragments
- explicit style default providers keyed by family role
- style-owned prototype recipe fragments
- exported role-occurrence roots for each fragment
- internal fragment instances that should not count toward role cardinality
- simple semantic parameter bindings
- a concrete `FamilyInstantiationRequest`

The compiler then validates the family/style pair, resolves required role providers, deterministically remaps fragment IDs into one `AssetRecipe`, applies semantic controls to concrete scalar paths or part presence, validates the recipe, compiles geometry, and returns an instantiation report. Omitted request parameters are filled from the family slot defaults before provider choice, presence toggles, and scalar bindings run.

Provider selection is explicit. A style-required role uses `StyleImplementation::default_role_providers` unless a choice binding selects a specific style prototype. A family-default role uses `FamilyImplementation::default_role_providers`. A family-or-style role prefers the style default and falls back to the family default. This avoids accidental selection changes when prototype IDs or `BTreeMap` ordering change.

Recipe fragments declare `role_occurrence_roots` separately from `internal_instances`. Cardinality checks and presence toggles operate on occurrence roots and their subtrees, so internal ribs, fasteners, helper geometry, or local construction pieces do not accidentally count as separate role occurrences.

Instantiated recipes derive `AssetId` from a deterministic hash of the family ID, style ID, effective semantic parameters, seed, implementation schema versions, and selected fragment versions. The ID is not the seed.

The first binding language is intentionally small:

- direct scalar
- scale plus offset
- ratio into range
- integer count
- choice-to-prototype
- toggle-to-part-presence

This avoids embedding an unrestricted expression language in pack data while still proving:

```text
bridge family
+ Roman timber style
+ span_length = 4
-> concrete AssetRecipe
```

Cross-domain acceptance tests currently cover bridge plus Roman timber, crate plus sci-fi industrial, and lamp plus stylized furniture bindings.

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
- Executable recipe fragments and semantic parameter mappings live in `shape-family-compile` or content-pack binding crates, not in `shape-family`.
- Adding a new family should normally be data and recipes, not engine changes.
- A new modeling operator should serve at least two unrelated families before it is promoted into the general engine.
- Gameplay balance remains outside Shape Lab.

## Development Test

The near-term generality test is not "support all 3D objects." It is support several hard-surface and modular families with different styles:

- Roman field engineering for Project Caesar.
- Sci-fi industrial props.
- Stylized furniture and lighting.

If a missing operation improves two or three of those packs, it likely belongs in the engine. If it only helps the Roman pack, it should be scrutinized as content-pack logic or style-kit data first.
