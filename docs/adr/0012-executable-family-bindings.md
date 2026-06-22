# ADR 0012: Executable Family Bindings

## Status

Accepted.

## Context

`shape-family` defines theme-neutral asset-family and style-kit contracts. That is necessary but not sufficient for generation: descriptive prototypes and parameter slots do not say which `AssetRecipe` fragments to merge or which concrete scalar paths a novice-facing control should edit.

The foundry needs an executable layer that can prove:

```text
family schema
+ compatible style kit
+ semantic parameter values
-> concrete AssetRecipe
-> compiled geometry
```

## Decision

Keep `shape-family` as a pure schema and validation crate.

Add `shape-family-compile` for executable bindings:

- `FamilyImplementation`
- `StyleImplementation`
- `RecipeFragment`
- `FamilyInstantiationRequest`
- simple `ParameterBinding` variants
- deterministic instantiation reports

The compiler validates the family/style pair, resolves required role providers, merges recipe fragments with deterministic ID remapping, applies semantic parameters to concrete recipe fields, validates the resulting `AssetRecipe`, compiles geometry, and rejects compiled artifacts with validation issues.

Executable binding documents are versioned independently of the descriptive family/style schemas:

- `FAMILY_IMPLEMENTATION_SCHEMA_VERSION`
- `STYLE_IMPLEMENTATION_SCHEMA_VERSION`
- `RECIPE_FRAGMENT_SCHEMA_VERSION`

Family parameter slots carry semantic default values. Instantiation fills omitted request values from those defaults before resolving choices, presence toggles, scalar bindings, and deterministic identity.

Provider selection must not depend on map order. Style implementations declare `default_role_providers` explicitly, while family implementations retain explicit family defaults. Style-required roles use the style default unless a choice binding overrides them. Family-or-style roles prefer style defaults and fall back to family defaults.

Recipe fragments declare exported `role_occurrence_roots` separately from `internal_instances`. Role cardinality counts exported occurrence roots only. Presence toggles operate on those roots and their subtrees. This keeps helper geometry inside a fragment from becoming an accidental family-role occurrence.

The compiler derives the instantiated `AssetId` from a deterministic hash of the family ID, style ID, effective semantic parameters, seed, implementation schema versions, and selected fragment versions. The seed remains an input, not the ID.

The initial binding language deliberately supports only direct scalar, scale-offset, ratio, integer count, choice-to-prototype, and toggle-to-part-presence mappings.

## Consequences

- Family and style schemas remain portable metadata.
- Content packs can provide executable fragments without putting theme-specific behavior in the core schema crate.
- The first cross-domain proof covers bridge/Roman timber, crate/sci-fi industrial, and lamp/stylized furniture.
- Content packs can add internal fragment detail without breaking family role cardinality.
- Prototype ID ordering no longer changes default provider selection.
- More complex generation policies can be added later without committing to an unrestricted expression language now.
