# ADR 0015: Foundry Document And Control Layer

## Status

Accepted.

## Context

Executable family bindings can instantiate semantic asset recipes, but a novice
workflow also needs a source document that records catalog references, control
state, provider choices, locks, local overrides, reproducible build stamps, and
pack membership. Storing only the generated `AssetRecipe` would erase semantic
intent and make later style/provider changes ambiguous.

## Decision

Add `orchard-foundry` as the semantic source layer and
`orchard-foundry-catalog` as the catalog manifest layer.

`FoundryAssetDocument` stores exact content references for family, style, family
implementation, style implementation, and customizer profile. It also stores
`ControlValue` control state, provider overrides, foundry locks, local recipe
overrides, a seed, an optional catalog lock, and an optional build stamp.

Generated recipes are persisted as build snapshots, but they are not the source
of truth.

Local overrides carry:

- an override ID,
- the base `GeometryInputFingerprint`,
- an `AssetEditProgram`,
- touched semantic targets,
- a survival policy.

## Consequences

The foundry runtime can reject catalog mismatches, preserve or drop overrides
deliberately, and reproduce builds from exact content references. Later UI,
CLI, and automation surfaces can share the same command contracts.
