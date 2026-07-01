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

## Unsupported Workflows

- Active primitive Make must not rely on generated variation trays.
- Active primitive Make must not expose random candidate generation.
- General Visual Foundry assets do not have material/surface work,
  UV/texturing, rigging, skinning, animation, runtime LLM behavior, or full
  game-ready export support.
- Surface/material preview workflows are not part of the active primitive
  baseline.
- ObjectPlan Materialization v1 is offline review infrastructure. It can
  materialize and render supported primitive plans, but it does not generate
  broad families, publish catalog entries, or call LLMs in the app.
- ObjectPlan rendering reports honest blocked output for invalid or unsupported
  plans; contact sheets must not be faked.
- ObjectPlan outputs remain Draft and review-required.
- ObjectPlan outputs are not Godot-ready or game-ready engine packages.
- Geometry-only GLB export is the next proof and is not implemented for
  ObjectPlan yet.
- Godot import proof is required before any Godot-ready geometry claim.
- ObjectPlan review UI is internal-only and dev-gated.
- Public catalog publishing is blocked.
- Family Studio Lite is paused until direct primitive and composition flows are
  stable.

## Editing Limits

- Primitive editing is property-schema based.
- Users edit bounded properties such as Width, Height, Depth, Radius,
  Thickness, Edge Softness, and Flattening.
- Users do not edit vertices, faces, loops, cages, booleans, raw mesh
  transforms, or Blender-like modeling controls.
- Invalid values must not become current state.
- Future suggestions may return only as deterministic property presets.

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
