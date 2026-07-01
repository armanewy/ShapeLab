# Architecture Status

Date: 2026-07-01

## Current Architecture Direction

Shape Lab / Object Orchard is moving toward a semantic asset compiler. The
active user experience remains direct primitive editing, safe-anchor
composition, offline ObjectPlan drafting/review, and geometry-only export
proofs. The architecture target is that those flows converge on one canonical
semantic asset lane.

## Canonical Lane

The canonical lane for future A-J product work is `shape-asset::AssetRecipe` /
Orchard IR. This lane should carry semantic asset state, authoring operation
logs, relationships, patterns, review status, validation reports, and
export/proof includes.

## Legacy Lane

`shape-core::ShapeDocument` remains the legacy/implicit compatibility lane. It
is useful and should remain stable, but it is not the target product backbone
for new Orchard semantics.

## Active Phase

Phase A contract hardening is in progress. This phase is not a feature wave. It
exists to make boundary decisions durable before adding more UI handles,
surface work, terrain, collision, motion, or prototype-pack expansion.

## Blocked Product Claims

Shape Lab must not claim Godot-ready, game-ready, textured, UV-unwrapped,
collision-enabled, rigged, animated, terrain-ready, public catalog publishing,
or runtime LLM support unless a later phase gate produces tested evidence.
