# AuthoringOpLog v0

Date: 2026-07-01

Status: contracts/replay only.

## Goal

`orchard-authoring` defines the semantic authoring operation log for Object
Orchard. Product-visible edits should move toward typed `AuthoringOp`s over an
`AssetRecipe` clone instead of UI-local mutation or direct recipe edits.

## Implemented In v0

- `AuthoringOpLog`
- `AuthoringOpLogEntry`
- `AuthoringOpSource`
- `AuthoringEffect`
- `AuthoringOp`
- `AuthoringOutcome`
- `AuthoringRejection`
- `ReplayValidationReport`
- deterministic recipe hashes with `blake3`
- drag sample/coalescing DTOs

`AuthoringOp::SetProperty` is the first meaningful operation. It validates the
target parameter descriptor, calls `orchard_asset::set_scalar`, validates the
edited recipe, records before/after hashes, and adds an authoring shell entry to
the recipe.

The other required operation families are present as shell/no-op replay
operations or review-tier shell changes. They do not implement app behavior,
relationship authoring, export behavior, surface work, collision, motion, or
terrain.

## Boundary

This crate is the mutation boundary. It does not change the current Make UI.
Future UI controls should emit typed authoring operations and then replay or
apply those operations through this crate.

## Non-Goals

- no app UI wiring
- no visual handles
- no material/surface behavior
- no collision, motion, or terrain behavior
- no runtime LLM integration
- no public catalog publishing
- no game-ready claims
