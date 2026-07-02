# Known Limitations

## Catalog

- The active primitive baselines are Box Primitive and Flat Panel Primitive.
- Sphere Primitive is the active round primitive baseline.
- Panel with Knob is a safe-anchor composition proof, not a broad composition
  editor.
- Lidded Box and Hinged Panel are feature proofs, not the target active
  primitive workflow.
- Handled Panel evidence is paused historical proof and should not steer the
  next milestone.
- Box Primitive is intentionally a simple closed box-like clay volume.
- Flat Panel Primitive is intentionally one upright clay panel.
- Lidded Box must not be described as a crate.
- Flat Panel, Hinged Panel, and Handled Panel must not be described as Door.
- Sci-Fi Crate, Cargo Case, crate-family, generated-variation, candidate-tray,
  and old dogfood reports are not active product directions.

## Unsupported Workflows

- Cleanup and rename work does not add product capability, product UI,
  materials, UVs, collision, motion, terrain, runtime LLM, public catalog
  publishing, Godot-ready output, or game-ready output.
- Object Orchard is the product-facing name across product copy, crates,
  folders, commands, environment variables, project suffixes, metadata fields,
  generated evidence paths, and the GitHub repository host.
- Phase A-D semantic compiler hardening is integrated. It does not add new
  product-facing feature categories; it establishes that future A-J work should
  target `orchard-asset::AssetRecipe` / Orchard IR as the canonical semantic
  lane.
- `orchard-core-legacy::ShapeDocument` remains a legacy/implicit compatibility lane and
  is not the new canonical product IR for Object Orchard.
- Active primitive Make must not rely on generated variation trays.
- Active primitive Make must not expose random candidate generation.
- Object Orchard does not have material/surface product work, UV/texturing,
  collision/gameplay metadata, rigging, skinning, animation, runtime LLM
  behavior, or full game-ready export support.
- Surface/material preview workflows are not part of the active primitive
  baseline.
- ObjectPlan Materialization v1 is offline review infrastructure. It can
  materialize and render supported primitive plans, but it does not generate
  broad families, publish catalog entries, or call LLMs in the app.
- ObjectPlan rendering reports honest blocked output for invalid or unsupported
  plans; contact sheets must not be faked.
- ObjectPlan outputs remain Draft and review-required.
- ObjectPlan outputs are not Godot-ready or game-ready engine packages.
- Geometry-only GLB export exists only for supported ObjectPlan drafts and does
  not include UVs, textures, material looks, collision, rigging, animation, or
  game-ready status.
- Geometry export relationship realization reports are review evidence only.
  Current V0 combined-mesh GLB output does not prove separate Godot node
  hierarchy, submesh preservation, collision, materials, motion, or game-ready
  behavior.
- Godot import proof is required before any Godot-ready geometry claim. A
  blocked proof report is not a Godot-ready result.
- ObjectPlan review UI is internal-only and dev-gated.
- Public catalog publishing is blocked.
- Family Studio Lite v0 UI is limited to internal preview work for local Draft
  / Personal Direct Kits made from supported direct primitives, safe-anchor
  compositions, and supported ObjectPlan Draft evidence.
- Personal Kit storage is local/private only and does not publish, review, or
  promote kits.
- Prototype Pack brief output is limited to Draft ObjectPlans for offline
  review.
- Family Studio Lite v0 test results mean deterministic property endpoint
  checks, preset/contact-sheet evidence, ObjectPlan evidence, composition
  validation, and export-report truth checks. Generated candidate trays remain
  blocked.
- Terrain is not approved as only a generic mesh primitive. Product-facing
  terrain remains blocked until explicit terrain contracts, validation reports,
  collision/readiness evidence, and export includes pass.

## Editing Limits

- Primitive editing is property-schema based.
- Users edit bounded properties such as Width, Height, Depth, Radius,
  Thickness, Edge Softness, and Flattening.
- Users do not edit vertices, faces, loops, cages, booleans, raw mesh
  transforms, or Blender-like modeling controls.
- Invalid values must not become current state.
- Future suggestions may return only as deterministic property presets.
- Direct primitive Make is expected to evolve from inspector-first controls to
  model-anchored Orchard handles. The current exact-value inspector is a
  temporary fallback, not the target interaction model.
- Background candidate results must not surface as stale warning banners while
  users edit direct primitive properties.

## Product Boundaries

- Box Primitive does not prove a broad archetype library.
- Flat Panel Primitive does not prove a broad archetype library.
- Direct primitive editing does not approve imported mesh editing.
- Direct primitive editing does not approve cloud, collaboration, or telemetry
  features.
- Direct primitive editing does not approve material-look persistence.
- Direct primitive editing does not approve pack-level publishing.
- ObjectPlan Materialization v1 does not approve runtime LLM integration,
  public kit publishing, materials/surfaces, UV/texturing, collision,
  rigging, animation, Godot-ready claims, or game-ready claims.
- Geometry Export v0 does not approve material/surface work, UV/texturing,
  collision/gameplay metadata, rigging, animation, Godot-ready claims, or
  game-ready claims.
- Family Studio Lite v0 does not approve public catalog publishing,
  reviewed/showcase promotion, runtime LLM generation, broad family generation,
  material editor UI, UV editing UI, rigging, animation, or game-ready claims.
- Phase A-D semantic compiler hardening does not approve material/surface
  implementation, UV/texturing, collision/gameplay metadata, terrain
  implementation, motion, rigging, animation, runtime LLM integration, public
  catalog publishing, Godot-ready claims, or game-ready claims.
