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

Style kits expose role-specific shape language through family-scoped facets. This lets one style kit support multiple unrelated families without validating bridge role references against crate schemas or lamp role references against bridge schemas. Global style policies remain role-independent. `StyleImplementation` is explicitly tied to one family ID so its default providers and executable prototypes cannot be reused accidentally against another family.

Executable binding documents are versioned independently of the descriptive family/style schemas:

- `FAMILY_IMPLEMENTATION_SCHEMA_VERSION`
- `STYLE_IMPLEMENTATION_SCHEMA_VERSION`
- `RECIPE_FRAGMENT_SCHEMA_VERSION`

Family parameter slots carry semantic default values and an execution policy. Instantiation fills omitted request values from defaults before resolving choices, presence toggles, scalar bindings, and deterministic identity. `RequiredBinding` slots must have at least one executable binding. `AdvisoryOnly` and `RuntimeOnly` slots may be accepted without directly changing geometry.

Implementation validation rejects unbound required slots, duplicate provider-selection bindings for one role, duplicate presence bindings for one role, non-finite scalar transforms, and degenerate scalar transforms that collapse every input.

Provider selection must not depend on map order. Style implementations declare `default_role_providers` explicitly, while family implementations retain explicit family defaults. Style-required roles use the style default unless a choice binding overrides them. Family-or-style roles prefer style defaults and fall back to family defaults.

Style kit schema v4 removes flat, global role-scoped style data. `RoleProportion`, `PartPrototype`, and `DetailModule` records live under `StyleKit::family_facets`, so one kit can support unrelated families without a shared role namespace. Top-level legacy fields are rejected even when empty. Per-family global-policy differences use `FamilyStyleFacet::policy_overrides`.

Recipe fragments declare `RecipeFragmentExports`, which contains exported `role_occurrence_roots`, `internal_roots`, socket ports, and surface ports. Role cardinality counts exported occurrence roots only. Presence toggles operate on those roots and their subtrees. This keeps helper geometry inside a fragment from becoming an accidental family-role occurrence.

Export roots must be disjoint. A fragment cannot list nested occurrence roots, overlapping occurrence roots, internal roots inside exported occurrence subtrees, or exported occurrence roots inside internal subtrees. Cardinality uses effective enabled state through ancestor chains. Unsupported fragment metadata is rejected during implementation validation for every executable fragment, not only for fragments selected by the current request.

The compiler derives the instantiated `AssetId` from a domain-separated BLAKE3 geometry-input fingerprint. Geometry identity includes executable geometry contracts, selected providers, selected fragment content, required-binding parameter values, and seed. Advisory and runtime-only parameter values are part of the foundry-intent fingerprint but are not allowed to drive geometry and do not change `AssetId`. Instantiation reports expose separate foundry-intent, geometry-input, recipe, and artifact fingerprints as 64-character lowercase hex strings.

The initial binding language deliberately supports only direct scalar, scale-offset, ratio, integer count, choice-to-prototype, and toggle-to-part-presence mappings.

## Consequences

- Family and style schemas remain portable metadata.
- Content packs can provide executable fragments without putting theme-specific behavior in the core schema crate.
- The first cross-domain proof covers bridge/Roman timber, crate/sci-fi industrial, and lamp/stylized furniture.
- Content packs can add internal fragment detail without breaking family role cardinality.
- Prototype ID ordering no longer changes default provider selection.
- Content-only recipe changes alter instantiated identity without requiring schema-version bumps.
- Multi-family style kits can use family-scoped role vocabularies.
- User-facing semantic slots cannot silently do nothing unless they are explicitly advisory or runtime-only.
- More complex generation policies can be added later without committing to an unrestricted expression language now.
