# Current Product Status

Date: 2026-07-01

## Verdict

`CLEANUP_BASELINE_FREEZE`

Cleanup baseline freeze is starting. Cleanup does not add product capability;
it makes the repository physically match the semantic compiler architecture
before the Object Orchard rename.

Shape Lab has retired active variation UI for current primitives and is moving
the active product surface toward direct primitive property editing.
ObjectPlan Materialization v1 now exists as offline validation and review
infrastructure for supported primitive and safe-anchor composition plans.
Supported ObjectPlans can now be materialized into Draft internal asset graphs
and rendered into contact-sheet evidence for human review.
Geometry Export v0 now exports geometry-only GLB packages for supported
ObjectPlan drafts. Godot import proof exists as a harness, but local proof is
blocked when no Godot binary is available.
Family Studio Lite v0 now has an internal preview UI for local reusable Direct
Kits. That flow remains developer-gated and produces Draft / Personal Kits
only.
The current architecture phase has integrated the first semantic compiler
hardening stack: future A-J work targets `shape-asset::AssetRecipe` / Orchard
IR as the canonical semantic lane, while `shape-core::ShapeDocument` remains a
legacy/implicit compatibility lane rather than the new product backbone.
Cleanup Wave 1 is purging obsolete docs from old Sci-Fi Crate, Cargo Case,
crate-family, generated-variation, candidate, dogfood, showcase, and
game-ready package pivots. Those docs are not active status and must not be
restored as parallel product truth.

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
- Geometry export reports now include relationship realization summaries. A
  relationship-backed Panel with Knob export reports the child as part of the
  combined mesh, keeps `baked: false`, and preserves semantics in report/sidecar
  data for review.
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
- Choose now groups starting points by primitive provenance: primitives first,
  derived entries under their source primitive, and presets labeled as presets
  rather than separate asset families.
- Invalid property values must be rejected or clamped before they become
  current primitive state.
- Prior valid previews remain visible while a direct property edit rebuilds.
- Users may edit Width, Height, Depth, Radius, Thickness, Edge Softness, and
  Flattening when a primitive schema exposes those properties.
- Users do not edit vertices, faces, loops, cages, booleans, raw mesh
  transforms, or Blender-like modeling controls.
- Future suggestions may return only as deterministic property presets.
- Family Studio Lite v0 is scoped to local Direct Kits made from supported
  primitives, supported safe-anchor compositions, and supported ObjectPlan Draft
  evidence.
- Family Studio Lite v0 has a developer-gated internal preview UI for starting
  from the current supported shape, choosing what stays the same, choosing what
  can change, testing the kit, and saving Draft / Personal Kits.
- Personal Kit storage exists for local/private Direct Kits.
- Prototype Pack briefs can generate Draft ObjectPlan batches for offline
  review. They do not approve assets or publish kits.
- Family Studio Lite v0 uses deterministic property endpoint tests, preset
  contact sheets, ObjectPlan evidence, composition validation, and export-report
  truth checks. It will not restore generated candidate trays.
- Internal candidate-like machinery may remain for legacy tests, quality
  evidence, and contact sheets until a deliberate cleanup branch removes or
  repurposes it.
- Composition will happen through safe anchors and constrained attachment
  zones, not arbitrary free transforms.
- Material/surface work, UV/texturing, rigging, animation, runtime LLM
  integration, public catalog publishing, and game-ready UI remain blocked.
- Phase A-D semantic compiler hardening is integrated. It does not add new
  product-facing feature categories; it documents and tests that future
  controls, ObjectPlan work, export reports, terrain, surface, collision, and
  motion work must route through canonical semantic contracts.
- `shape-asset::AssetRecipe` / Orchard IR is the target canonical semantic
  asset lane for future A-J work.
- `shape-core::ShapeDocument` remains the legacy/implicit compatibility lane
  and must not receive new canonical product semantics for terrain, material,
  collision, motion, ObjectPlan approval, export readiness, or kit publishing.
- `shape-authoring::AuthoringOpLog` exists with replay support, and Box
  Primitive width is the first product-visible Direct Make edit bridged through
  `AuthoringOp::SetProperty`.
- Panel with Knob can be represented through `RelationshipContract`, including
  fixed-distance and proportional placement tests.
- `PatternContract` has a deterministic linear evaluation proof for internal
  compile/evaluation. It is not exposed as product-visible pattern handles.
- Terrain remains blocked as product-facing work until explicit terrain patch,
  placement, validation, collision/readiness, and export contracts pass. It is
  not approved as only a generic mesh primitive.
- Rename to Object Orchard is planned but not yet applied. Current repository,
  crate, and app metadata may still use Shape Lab / ShapeLab naming until the
  rename wave lands.

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
- Family Studio Lite v0 may be described only as internal preview infrastructure
  for creating local Draft / Personal Kits from supported direct primitives,
  safe-anchor compositions, and supported ObjectPlan Draft evidence.
- Personal Kit storage may be described only as local/private storage for Draft
  or PersonalOnly Direct Kits.
- Prototype Pack brief output may be described only as Draft ObjectPlans for
  offline review.
- "Test variations" in Family Studio Lite v0 may be described only as
  deterministic property endpoint, preset, ObjectPlan evidence, composition
  validation, and export-report checks.
- Offline LLM drafting may be referenced only as external draft JSON
  production. The app does not call LLMs at runtime.
- Semantic asset compiler architecture may be described only as the target
  contract lane. Phase A-D hardening does not mean new UI handles, terrain,
  materials, collision, motion, or game-ready output exists.

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
- Treating `shape-core::ShapeDocument` as the new canonical product IR.
- Representing product-facing terrain as only a generic mesh primitive.
- Family Studio Lite public authoring, broad family generation, generated candidate trays,
  reviewed/showcase promotion, and public kit publishing.
- Historical proof entries in default Choose.
