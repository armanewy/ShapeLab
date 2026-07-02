# Architecture Status

Date: 2026-07-01

## Current Architecture Direction

Object Orchard is moving toward a semantic asset compiler. The active user
experience remains direct primitive editing, safe-anchor composition, offline
ObjectPlan drafting/review, and geometry-only export proofs. The architecture
target is that those flows converge on one canonical semantic asset lane.

## Canonical Lane

The canonical lane for future A-J product work is `orchard-asset::AssetRecipe` /
Orchard IR. This lane should carry semantic asset state, authoring operation
logs, relationships, patterns, review status, validation reports, and
export/proof includes.

## Legacy Lane

`orchard-core-legacy::ShapeDocument` remains the legacy/implicit compatibility lane. It
is useful and should remain stable, but it is not the new canonical product IR
for new Orchard semantics.

## Active Phase

Phase A-D semantic compiler hardening has passed. Cleanup Wave 1 has removed
obsolete pivots, split oversized files, retired unreleased compatibility code,
and kept boundary decisions durable before adding more UI handles, surface
work, terrain, collision, motion, or prototype-pack expansion.

## Rename Status

Object Orchard is the product name. The Rust crates, folders, command names,
environment variables, and some repository-local paths still use legacy
implementation names until the technical rename branches land.

## Documentation Status

Active docs are indexed by `docs/README.md`. Old pivot reports are not
architecture truth unless that index names them and a current test or contract
still validates them.

## Blocked Product Claims

Object Orchard must not claim Godot-ready, game-ready, textured, UV-unwrapped,
collision-enabled, rigged, animated, terrain-ready, public catalog publishing,
or runtime LLM support unless a later phase gate produces tested evidence.
