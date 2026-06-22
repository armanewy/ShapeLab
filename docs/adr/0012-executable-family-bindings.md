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

The initial binding language deliberately supports only direct scalar, scale-offset, ratio, integer count, choice-to-prototype, and toggle-to-part-presence mappings.

## Consequences

- Family and style schemas remain portable metadata.
- Content packs can provide executable fragments without putting theme-specific behavior in the core schema crate.
- The first cross-domain proof covers bridge/Roman timber, crate/sci-fi industrial, and lamp/stylized furniture.
- More complex generation policies can be added later without committing to an unrestricted expression language now.
