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

Generated variations are removed from the active primitive workflow. Candidate
and variation code may remain as internal, legacy, or test-only machinery until
a later branch deliberately repurposes it.

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

## Future Presets

Future suggestions may return only as deterministic property presets. A preset
is a named set of legal property values that validators can inspect and reject.
It is not random candidate generation and it is not a hidden mesh rewrite.

## Composition

Composition will happen through safe anchors, not arbitrary Blender-like
transforms. A primitive can expose named attachment places, and child placement
must be derived from bounded policies.

## LLM Boundary

LLMs may later draft preset/property plans and repair suggestions. They do not
control mesh generation, bypass validators, or create runtime modeling behavior.
The active primitive workflow does not include runtime LLM integration.
Material/surface work, UV/texturing, rigging, and animation remain blocked.

## Milestone Rule

Use one visible operation per milestone:

1. direct-edit Box
2. direct-edit Flat Panel
3. direct-edit Sphere
4. make knob-like form from Sphere
5. attach knob-like form to panel through safe anchor
