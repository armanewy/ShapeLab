# Contract Boundaries

Date: 2026-07-01

Status: Phase A contract hardening.

## Decision

Object Orchard is converging on a semantic asset compiler architecture.
Beginner-facing controls must become typed semantic operations over an
`AssetRecipe` / Orchard asset graph. The UI, ObjectPlan tooling, export tools,
and future review flows must not mutate separate ad hoc product state.

`shape-asset::AssetRecipe` is the canonical semantic asset lane for future
A-J product work. A future `shape-orchard-ir` crate may refine that lane, but
new product semantics should still flow through explicit semantic contracts.

`shape-core::ShapeDocument` remains useful for low-level conventions, legacy
implicit/SDF compatibility, geometry helpers, and existing compile paths. It is
not the new canonical product IR for Orchard asset authoring.

## Ownership Rule

Future branches must declare the contract they consume and the contract they
produce. Product-visible edits should move toward typed authoring operations,
not UI-local state changes or raw document mutations.

Allowed Phase A targets:

- `AssetRecipe` / Orchard IR semantic shells;
- `AuthoringOpLog` and replay boundaries;
- relationship and pattern contracts;
- export/proof report includes and realization summaries;
- product-claim gates that make unsupported claims fail tests.

Blocked Phase A targets:

- new app UI behavior;
- expanding `shape-core::ShapeDocument` into the product backbone;
- material/surface, UV, collision, terrain, rigging, animation, or motion
  implementation;
- runtime LLM integration;
- public catalog publishing;
- game-ready, Godot-ready, textured, rigged, animated, collision-enabled, or
  terrain-ready claims without passed evidence gates.

## Canonical Semantic Lane

The semantic asset compiler lane owns durable product meaning:

- primitive nodes and bounded properties;
- relationship contracts for composition;
- pattern contracts for repetition;
- authoring operation logs and replay breadcrumbs;
- surface, collision, motion, and terrain capability shells until their gates
  pass;
- export includes/excludes and realization reports.

UI controls are views over this lane. ObjectPlan is an offline draft input into
this lane. Geometry export is an output proof from this lane. None of those
paths should invent their own canonical semantics.

## ShapeDocument Boundary

`ShapeDocument` can continue to support existing rendering, compile, preview,
and low-level modeling compatibility. New A-J product concepts must not be
added to `ShapeDocument` as the canonical source of truth. Examples of blocked
ShapeDocument-owned product semantics include terrain patches, material looks,
collision bodies, motion channels, ObjectPlan approval state, export readiness,
public kit publishing, and game-ready status.

## Terrain Boundary

Terrain must not be collapsed into "just a generic mesh primitive" as a product
claim. Terrain needs explicit contracts for patch identity, scale, placement,
collision/readiness reports, and export includes before it becomes
product-facing.

## Product Claim Gate

Positive claims are blocked until their phase gates pass. The following words
may appear only as explicit negative or blocked statements unless evidence is
present and tested:

- Godot-ready;
- game-ready;
- textured;
- UV unwrapped;
- collision-enabled;
- rigged;
- animated;
- terrain-ready;
- reviewed/public kit;
- runtime LLM;
- public catalog publishing.

## Definition Of Done For Phase A

Phase A is done when the repository documents and tests that:

- `AssetRecipe` / Orchard IR is the canonical future semantic lane;
- `ShapeDocument` is legacy/implicit compatibility for A-J work;
- semantic shells can carry relationship, pattern, surface, collision, motion,
  terrain, export, authoring, validation, and review state;
- authoring operations are the mutation boundary for product-visible edits;
- relationship and pattern contracts exist before new handles or composition
  UI;
- export/proof reports state included and excluded capabilities truthfully.
