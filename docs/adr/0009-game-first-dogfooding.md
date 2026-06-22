# ADR 0009: Game-First Dogfooding

## Status

Accepted

## Context

Shape Lab has been developing a general explicit modeling foundation: semantic recipes, part-aware history, sockets, regions, boundary loops, validation, candidate search, deterministic export, and Blender reconstruction. The next risk is building broad modeling capability without a real production customer.

Project Caesar's River Bend prototype has a concrete need for modular engineering assets: piles, bridge decks, ramps, palisades, gates, towers, roads, marching camps, decoy worksites, and formation-level command pieces. Its fallback model is a grammar of semantic modules, anchors, and layered walkable surfaces, which matches Shape Lab's current architecture.

## Decision

Shape Lab will use Project Caesar as its primary near-term internal customer. The first production target is a coherent, editable, validated River Bend asset pack that replaces graybox engineering modules and command pieces.

This does not make Shape Lab's core Project Caesar-specific. Asset-family and style-kit contracts live in `shape-family`; game-neutral runtime contracts live in `shape-gamekit`; Project Caesar authored content lives in `shape-caesar-assets`.

## Consequences

- Near-term modeling work is prioritized by real camera, runtime, export, and validation needs.
- Gameplay balance remains in game code, not Shape Lab metadata.
- Non-Caesar reference packs should be used as generality checks whenever new operators or family contracts are added.
- GPU viewport work is deferred unless CPU interaction becomes a demonstrated blocker.
