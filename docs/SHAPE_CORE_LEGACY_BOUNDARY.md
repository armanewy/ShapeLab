# Shape Core Legacy Boundary

Date: 2026-07-01

Status: Phase A contract hardening.

## Summary

`shape-core` owns low-level modeling conventions and legacy compatibility. It
can define shared scalar types, IDs, transforms, bounding boxes, primitive and
CSG conventions, existing `ShapeDocument` helpers, and compatibility behavior
for current compile and preview paths.

`shape-core::ShapeDocument` is not the canonical A-J product IR. Future Object
Orchard product semantics belong in `shape-asset::AssetRecipe` / Orchard IR or
in explicit semantic crates built around that lane.

## What Shape Core May Own

- low-level geometry and modeling conventions;
- legacy implicit/SDF document compatibility;
- helpers needed by existing compile paths;
- stable primitive and CSG vocabulary already used by existing code;
- validation primitives that do not make product readiness claims.

## What Shape Core Must Not Become

`shape-core` must not become the product backbone for new A-J work. Do not add
canonical product semantics here for:

- ObjectPlan approval or review state;
- authoring operation logs;
- relationship or pattern authoring semantics;
- material/surface workflow;
- UV unwrapping or texture authoring;
- collision or gameplay metadata;
- terrain readiness or terrain authoring;
- rigging, skinning, animation, or motion channels;
- export readiness, Godot-ready status, or game-ready status;
- public catalog publishing or reviewed-kit promotion.

## Required Routing

New product semantics should route through explicit contracts:

- `shape-asset::AssetRecipe` / Orchard IR for semantic asset state;
- authoring-op contracts for replayable product-visible edits;
- relationship contracts for attachments and composition;
- pattern contracts for repetition;
- report contracts for export includes, proof status, and realization.

If a future branch needs a low-level helper in `shape-core`, it should keep that
helper product-neutral and document which semantic crate owns the product
meaning.

## Testable Boundary

Docs and tests should fail if they imply that `ShapeDocument` is the new
canonical product IR, that terrain is just a generic mesh primitive, that
runtime LLM behavior is approved, that public catalog publishing is approved,
or that game-ready output is approved.
