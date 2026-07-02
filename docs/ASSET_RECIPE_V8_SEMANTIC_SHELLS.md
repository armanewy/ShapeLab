# AssetRecipe v8 Semantic Shells

Date: 2026-07-01

Status: schema/contracts only.

## Goal

AssetRecipe v8 adds empty semantic shells for the canonical Object Orchard
asset lane. The shells let future branches converge on `AssetRecipe` / Orchard
IR without expanding `orchard-core-legacy::ShapeDocument` or adding product-facing
features prematurely.

## Added Shells

AssetRecipe now carries a `semantic` field with stable ID shells for:

- relationship contracts;
- pattern contracts;
- surface slots;
- material slots;
- collision bodies;
- motion channels;
- terrain patches;
- export profiles;
- authoring operation entries;
- validation report entries;
- review state;
- copy-on-write lineage;
- effect hashes;
- export includes/excludes.

## Validation

Empty shells validate. Populated shells must use stable, non-zero IDs. Map keys
must match payload IDs. References to instances, definitions, surface slots,
export profiles, parameters, or reports fail validation when the target does
not exist.

Review state remains Draft / review-required. Public catalog visibility and
publishing are blocked. Export include shells must not claim UVs, textures,
material looks, collision, gameplay metadata, rigging, skinning, animation,
terrain collision, Godot scene output, or game-ready status.

## Migration

Schema versions 1 through 7 migrate to schema version 8 during deserialize.
Missing `semantic` fields and missing new ID counters default to empty shells
and fresh counters. Schema 0 remains unsupported by validation.

## Non-Goals

This branch does not implement app UI, export behavior, surface/material
behavior, collision, motion, terrain, public catalog publishing, runtime LLM
integration, or game-ready output.
