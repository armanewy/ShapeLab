# ADR 0014: Content Identity Scopes

## Status

Accepted.

## Context

Shape Lab needs stable identities for catalogs, foundry requests, generated geometry, recipes, and compiled artifacts. A single instantiation hash was too coarse: advisory UI intent and runtime metadata could perturb geometry IDs, while executable geometry changes needed stronger content-addressed tracking.

## Decision

Use domain-separated BLAKE3-256 content fingerprints with canonical JSON serialization. Fingerprints serialize as 64 lowercase hexadecimal characters.

The identity scopes are:

- `CatalogContentFingerprint`: catalog/document content.
- `FoundryIntentFingerprint`: complete semantic foundry intent, including advisory and runtime-only values.
- `GeometryInputFingerprint`: only values consumed by executable geometry generation.
- `ConformanceContractFingerprint`: validation, rejection, and grading rules that govern whether generated output is acceptable.
- `BuildFingerprint`: geometry input plus conformance contract plus compiler/schema versions.
- `RecipeFingerprint`: an instantiated `AssetRecipe` snapshot.
- `ArtifactFingerprint`: a compiled artifact snapshot.

`AssetRecipe.id` is derived from the geometry-input fingerprint. `AdvisoryOnly` and `RuntimeOnly` family parameters are accepted as intent but may not be consumed by executable parameter bindings, and changing their values does not change geometry identity. Request seeds, validation-only family constraints, export requirements, recipe-level constraints and relationships, selected-fragment export tags, and advisory style policies remain visible in foundry intent or conformance/build identity, but they do not perturb geometry identity until an executable compiler path consumes them.

Selected fragment exports are split by scope. Geometry identity includes only the occurrence roots, internal roots, and socket-port selectors needed to instantiate the concrete recipe. Compatibility tags and semantic surface-port tags are conformance metadata because they can accept or reject attachment and validation contracts without changing the generated geometry when the concrete selectors are unchanged.

## Consequences

- Geometry IDs are stable under non-geometry intent changes.
- Advisory/runtime information remains traceable through foundry intent.
- Validation and export contract changes can invalidate builds without changing geometry IDs.
- Content-only fragment changes still alter geometry identity without schema-version bumps.
- Future caches can key by the narrowest correct identity scope.
