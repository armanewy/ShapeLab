# Primitive Direct-Make Vision

Date: 2026-06-30

Object Orchard starts from direct primitive editing, not variation generation.
The active Make loop should feel like choosing a primitive, editing exact
properties, inspecting the result, and exporting it.

## Active Workflow

```text
Choose Primitive
-> Make
-> edit bounded primitive properties
-> orbit and inspect
-> Add to Pack
-> Export
```

Generated variations are removed from the active primitive workflow.
Candidate generation is inactive in the current primitive product flow.
Candidate-like machinery may remain as internal, legacy, quality-evidence,
contact-sheet, or test-only code until a later branch deliberately repurposes
it.

## Primitive Schemas

Current active primitives should expose immutable property schemas. Users
manipulate bounded properties such as:

- Width
- Height
- Depth
- Radius
- Thickness
- Edge Softness
- Flattening

Users do not manipulate vertices, faces, loops, cages, booleans, raw mesh
transforms, or arbitrary topology.

## Direct Property UI

Box Primitive, Flat Panel Primitive, and Sphere Primitive are the active
direct-edit baselines.
Their Make panels expose bounded numeric property controls with visible domains
and reset actions:

- Box Primitive: Width, Depth, Height, Edge Softness
- Flat Panel Primitive: Width, Height, Thickness, Edge Softness
- Sphere Primitive: Width, Height, Depth, Front Flatten, Back Flatten
- Panel with Knob composition: Panel Width, Panel Height, Panel Thickness,
  Panel Edge Softness, Knob Width, Knob Height, Knob Depth, Knob Front Flatten,
  Knob Back Flatten, Knob Horizontal Position, Knob Vertical Position

Invalid values cannot become current primitive state. During direct edits, the
previous valid preview remains visible while the exact updated build compiles,
and the UI labels the preview as updating instead of blanking the model stage.

View controls remain inspection-only: orbit around the asset, reset view, and
axis orientation. They are not mesh transform gizmos, vertex tools, face tools,
or object-level free transform handles.

## Future Presets

Future suggestions may return only as deterministic property presets. A preset
is a named set of legal property values that validators can inspect and reject.
Sphere Primitive's Knob-like form is the first visible preset and remains a
property preset only. It is not random candidate generation and it is not a
hidden mesh rewrite.

## Composition

Composition will happen through safe anchors, not arbitrary Blender-like
transforms. A primitive can expose named attachment places, and child placement
must be derived from bounded policies.

Panel with Knob is the first visible composition proof. It attaches a knob-like
Sphere form to a Flat Panel handle zone through a validated anchor attachment.
It is not a Door claim and it does not include open/close motion.

## LLM Boundary

LLMs may draft ObjectPlan JSON outside the app and may suggest preset/property
plans or repair suggestions. They do not control mesh generation, bypass
validators, publish catalog entries, or create runtime modeling behavior. The
active primitive workflow does not include runtime LLM integration.
Material/surface work, UV/texturing, rigging, and animation remain blocked.

ObjectPlan Materialization v1 validates structured offline plans, materializes
supported primitive plans into Draft internal asset graphs, renders
contact-sheet evidence for supported plans, and can run batches for human
review. Unsupported plans remain blocked with explicit reports. Geometry-only
GLB export exists for supported ObjectPlan drafts, and Godot import proof is
required before calling any output Godot-ready. Broad family generation,
automatic app-side drafting, public catalog publishing, automatic approval, and
Prototype Pack Mode are not part of the active primitive product yet.

## Milestone Rule

Use one visible operation per milestone:

1. direct-edit Box
2. direct-edit Flat Panel
3. direct-edit Sphere
4. make knob-like form from Sphere
5. attach knob-like form to panel through safe anchor
