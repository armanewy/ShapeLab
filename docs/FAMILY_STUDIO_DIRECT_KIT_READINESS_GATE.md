# Family Studio Direct-Kit Readiness Gate

Date: 2026-07-01

## Verdict

`FAMILY_STUDIO_DIRECT_KIT_READY_FOR_CONTRACTS`

Family Studio Lite v0 is scoped as an internal preview flow for local reusable
Direct Kits. It is not a public approval surface, not broad family generation,
not a generated variation tray, and not runtime LLM integration.

## What Is A Direct Kit?

A Direct Kit is a local reusable kit made from one supported primitive or safe
composition. It exposes a selected subset of bounded properties as user-changeable
controls, keeps the remaining properties fixed, may include deterministic
presets, references evidence reports, and stays Draft or PersonalOnly.

Direct Kits are intended to answer plain user questions:

- what stays the same
- what can change
- which tests were run
- whether evidence exists
- whether the result is private and review-required

## Allowed Starting Points

Family Studio Lite v0 may start from:

- current Box Primitive
- current Flat Panel Primitive
- current Sphere Primitive
- current Panel with Knob safe-anchor composition
- a supported ObjectPlan Draft selected from internal review

Unsupported ObjectPlans, arbitrary mesh payloads, imported meshes, free
transforms, and public catalog entries are not valid starting points.

## Changeable Properties

Only properties already exposed by primitive schemas may become changeable:

- Box Primitive: Width, Depth, Height, Edge Softness
- Flat Panel Primitive: Width, Height, Thickness, Edge Softness
- Sphere Primitive: Width, Height, Depth, Front Flatten, Back Flatten
- Panel with Knob: panel dimensions, knob form, and bounded knob safe position

Family Studio Lite v0 does not expose vertices, faces, loops, cages, booleans,
raw transforms, free placement, or Blender-like modeling tools.

## Deterministic Presets

Deterministic presets may be included when they validate against the matching
primitive schema. Current built-ins include Box, Flat Panel, and Sphere presets,
including the Sphere Primitive Knob-like form preset.

Presets are named property bundles. They are not generated variations, random
candidates, or automatic quality approval.

## ObjectPlan Evidence

Supported ObjectPlan outputs may be used as evidence when they remain Draft and
review-required:

- normalized ObjectPlan JSON
- materialization reports
- render evidence reports
- PNG previews and contact sheets for supported render paths
- geometry-only export reports
- blocked reports for unsupported paths

ObjectPlan evidence does not approve a kit and does not make it public,
Godot-ready, or game-ready.

## What "Test The Kit" Means

"Test variations" means deterministic bounded checks only:

- property endpoint tests
- preset contact sheets where available
- ObjectPlan render evidence where available
- composition attachment validation
- geometry export/report truth checks

It does not mean generated candidate magic, randomized sampling, runtime LLM
drafting, or a returned tray of generated ideas.

## Blocked From V0

Family Studio Lite v0 blocks:

- public product approval
- public catalog publishing
- broad family generation
- generated candidate trays
- Blender-like primitive editing
- material editor UI
- UV editing UI
- rigging or animation UI
- runtime LLM integration
- showcase/reviewed promotion
- game-ready or marketplace-ready claims

The next allowed work is Direct Kit contracts, capability cards, local/private
Personal Kit storage, deterministic kit tests, and then an internal preview UI.
