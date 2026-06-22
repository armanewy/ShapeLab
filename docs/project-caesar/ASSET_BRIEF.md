# Project Caesar Asset Brief

Shape Lab's near-term production customer is Project Caesar's River Bend prototype. The goal is to use Project Caesar as the first demanding asset-pack customer while keeping Shape Lab a general asset-family foundry.

Project Caesar is a single-player operational engineering strategy game set during Caesar's campaigns in Gaul. The game uses formation-level command pieces, modular field engineering, continuous time with pause, a top-down or oblique 2.5D camera, and semantic runtime modules rather than unrestricted structure physics.

Shape Lab should initially produce placed structures and command pieces from generic family contracts and a Project Caesar style kit. Terrain remains owned by the game: rivers, ridges, forests, and ground generation are outside this first asset pack.

## First Proof

```text
Project Caesar graybox module
  -> generic asset family
  -> Roman timber style kit
  -> Shape Lab semantic template
  -> Refine / Explore variants
  -> accepted construction recipe
  -> game-scale validation
  -> Godot-ready asset bundle
  -> used in the River Bend scenario
```

## Runtime Module Keys

The River Bend prototype reserves these engineering module keys:

- `pile`
- `deck`
- `ramp`
- `palisade`
- `tower`
- `road`
- `gate`
- `marching_camp`
- `decoy_worksite`

Command pieces are formation-level symbols, not individual soldiers. Their first templates should favor clear silhouettes, facing indicators where relevant, and stable footprints over miniature detail.

## Scope

Keep developing explicit topology, semantic parts, sockets and attachment frames, arrays, construction operations, boundary-loop bevels, provenance, branchable variants, validation, and deterministic export.

Add game-module metadata, construction-phase metadata, game-scale readability tests, walkable/support surfaces, simple collision proxies, batch pack export, Godot adapter, and local dogfooding metrics.

Defer deeper deformation decompilation, arbitrary mesh reconstruction, universal booleans, materials, UVs, rigging, animation, marketplace packaging, and GPU viewport work unless CPU interaction becomes a real blocker. Do not defer generic family and style-kit contracts; they are the boundary that keeps Project Caesar content out of the core engine.
