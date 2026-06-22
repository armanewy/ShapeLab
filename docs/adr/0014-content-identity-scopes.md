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
- `RecipeFingerprint`: an instantiated `AssetRecipe` snapshot.
- `ArtifactFingerprint`: a compiled artifact snapshot.

`AssetRecipe.id` is derived from the geometry-input fingerprint. `AdvisoryOnly` and `RuntimeOnly` family parameters are accepted as intent but may not be consumed by executable parameter bindings, and changing their values does not change geometry identity.

## Consequences

- Geometry IDs are stable under non-geometry intent changes.
- Advisory/runtime information remains traceable through foundry intent.
- Content-only fragment changes still alter geometry identity without schema-version bumps.
- Future caches can key by the narrowest correct identity scope.
