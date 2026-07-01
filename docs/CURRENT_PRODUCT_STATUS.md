# Current Product Status

Date: 2026-07-01

## Verdict

`GEOMETRY_EXPORT_V0_REVIEW_READY_GODOT_BLOCKED`

Shape Lab has retired active variation UI for current primitives and is moving
the active product surface toward direct primitive property editing.
ObjectPlan Materialization v1 now exists as offline validation and review
infrastructure for supported primitive and safe-anchor composition plans.
Supported ObjectPlans can now be materialized into Draft internal asset graphs
and rendered into contact-sheet evidence for human review.
Geometry Export v0 now exports geometry-only GLB packages for supported
ObjectPlan drafts. Godot import proof exists as a harness, but local proof is
blocked when no Godot binary is available.

## Current Truth

- Box Primitive is the direct box primitive baseline.
- Flat Panel Primitive is the direct panel primitive baseline.
- Sphere Primitive is the direct round primitive baseline.
- Panel with Knob is the first safe-anchor composition prototype: Flat Panel
  plus a knob-like Sphere form.
- Box Primitive exposes bounded Width, Depth, Height, and Edge Softness
  controls in Make.
- Flat Panel Primitive exposes bounded Width, Height, Thickness, and Edge
  Softness controls in Make.
- Sphere Primitive exposes bounded Width, Height, Depth, Front Flatten, and
  Back Flatten controls in Make.
- Panel with Knob exposes bounded panel size, knob form, and knob position
  controls in Make.
- Lidded Box and Hinged Panel are feature proofs, not the future shape of the
  active primitive workflow.
- Handled Panel evidence is treated as paused historical proof, not the current
  active direction.
- Generated idea workflows are retired from active primitive UI.
- Candidate generation is inactive in the current primitive product flow.
- The active Make workflow exposes direct property controls before suggestions.
- ObjectPlan Materialization v1 exists for offline validation, materialization,
  render evidence, and batch review of supported primitive plans and
  safe-anchor composition plans.
- ObjectPlan CLI materializes supported primitive plans into Draft internal
  asset graphs and produces contact-sheet evidence for supported render paths.
- Unsupported or invalid ObjectPlans produce honest blocked reports and no fake
  contact sheets.
- ObjectPlan Geometry Export v0 exports geometry-only GLB packages for
  supported Box Primitive, Flat Panel Primitive, Sphere Primitive, and Panel
  with Knob plans.
- Geometry-only export is scoped to mesh data only and does not include UVs,
  textures, material looks, collision, rigging, animation, or game-ready
  status.
- Godot import proof is required before claiming Godot-ready geometry. The
  current local Godot proof is `Blocked` because no Godot binary was available.
- Offline LLMs may draft ObjectPlan JSON outside the app, but Object Orchard
  validates every plan and LLM drafts remain Draft until reviewed.
- ObjectPlan review UI is internal-only and dev-gated; it is not part of the
  default novice UI and does not publish catalog entries.
- Deterministic presets are allowed only when they are named sets of legal
  property values. Sphere Primitive includes a Knob-like form preset.
- Primitive editing is property-schema based and bounded.
- Direct property controls show visible domains, use bounded numeric steppers,
  and support reset to authored defaults.
- Invalid property values must be rejected or clamped before they become
  current primitive state.
- Prior valid previews remain visible while a direct property edit rebuilds.
- Users may edit Width, Height, Depth, Radius, Thickness, Edge Softness, and
  Flattening when a primitive schema exposes those properties.
- Users do not edit vertices, faces, loops, cages, booleans, raw mesh
  transforms, or Blender-like modeling controls.
- Future suggestions may return only as deterministic property presets.
- Internal candidate-like machinery may remain for legacy tests, quality
  evidence, and contact sheets until a deliberate cleanup branch removes or
  repurposes it.
- Composition will happen through safe anchors and constrained attachment
  zones, not arbitrary free transforms.
- Family Studio Lite is paused until direct primitive and composition flows are
  stable.
- Material/surface work, UV/texturing, rigging, animation, runtime LLM
  integration, public catalog publishing, and game-ready UI remain blocked.

## Allowed Product Claims

- Shape Lab can start from a Box Primitive profile.
- Shape Lab can start from a Flat Panel Primitive profile.
- Shape Lab can start from a Sphere Primitive profile.
- Shape Lab can compile clay primitive previews and exports.
- Shape Lab can expose bounded primitive properties through direct controls.
- Lidded Box may be referenced as Box Primitive plus one visible Lid Seam proof.
- Hinged Panel may be referenced as Flat Panel Primitive plus one visible Hinge
  Edge proof.
- Direct primitive editing is the current product baseline.
- Deterministic presets are the only approved future suggestion form for active
  primitives.
- Current primitive Make shows direct property controls, view controls, Add to
  Pack, and Export rather than Try ideas or selected-candidate comparison.
- Current Box and Flat Panel Make screens allow direct dimension edits through
  bounded controls with visible domains.
- Current Sphere Make screens allow direct dimension and flattening edits
  through bounded controls with visible domains.
- The Knob-like form preset may be referenced only as a Sphere Primitive
  property preset, not as a door or composition claim.
- Panel with Knob may be referenced only as a constrained composition proof,
  not as a Door, motion, rigging, animation, material, or game-ready claim.
- View controls are inspection-only: orbit, reset view, and axis orientation.
- ObjectPlan may be referenced as structured offline validation and review
  infrastructure for supported primitives and safe-anchor compositions.
- ObjectPlan can be described as producing reviewable Draft prototype geometry
  for supported primitive plans only.
- ObjectPlan render evidence can be described as contact-sheet evidence, not
  approval.
- ObjectPlan batch review may be referenced as offline review infrastructure
  that classifies Keep / Regenerate / Simplify / Blocked, not Prototype Pack
  Mode and not automatic approval.
- ObjectPlan Geometry Export v0 may be described as geometry-only GLB export
  for supported ObjectPlan drafts.
- Godot import proof may be described only as Passed when the harness reports
  `Passed`; a `Blocked` report does not make output Godot-ready.
- Offline LLM drafting may be referenced only as external draft JSON
  production. The app does not call LLMs at runtime.

## Current Milestone Sequence

Use one visible operation per milestone:

1. direct-edit Box
2. direct-edit Flat Panel
3. direct-edit Sphere
4. make knob-like form from Sphere
5. attach knob-like form to panel through safe anchor

## Still Blocked

- Generated variation trays in the active primitive workflow.
- Random candidate generation as a product-visible primitive Make action.
- Vertex, face, loop, cage, boolean, sculpt, or raw transform editing.
- Mesh transform gizmos, object handles, vertex selection, and face selection.
- Arbitrary Blender-like scene modeling.
- Door naming before a later gate explicitly approves it.
- Open/close motion.
- Material/surface editor work.
- UV/texturing UI.
- Rigging, skinning, or animation UI.
- Runtime LLM integration.
- Current ObjectPlan outputs being described as Godot-ready or game-ready.
- Godot-ready claims until a real Godot import proof passes.
- Public ObjectPlan authoring UI, automatic offline LLM drafting in the app,
  and any automatic ObjectPlan approval flow.
- Public catalog publishing.
- Full game-ready or marketplace-ready claims.
- Family Studio Lite until direct primitive and composition flows are stable.
