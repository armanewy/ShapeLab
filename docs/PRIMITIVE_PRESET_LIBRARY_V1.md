# Primitive Preset Library Hardening v1

Date: 2026-07-01

## Scope

Primitive presets are deterministic property bundles for approved primitive
schemas. They replace the old generated-variation UX for primitive editing.

Presets are not random generation, runtime LLM output, material/surface work,
UV/texturing, rigging, animation, public catalog assets, or hidden mesh edits.

## Preset Contract

Each `PrimitivePreset` contains:

- `preset_id`
- `display_name`
- `primitive_kind`
- `property_values`
- `user_description`
- `intended_use_tags`
- `review_tier`
- `source`

Supported `PresetSource` values:

- `BuiltIn`
- `UserSaved`
- `ObjectPlanDraft`
- `InternalTool`

## Review Rules

- Built-in presets must validate against the primitive property schema and use
  `Reviewed` tier.
- User-saved presets are personal/local only and use `Personal` tier.
- ObjectPlan-draft presets are Draft only.
- No preset becomes public catalog-visible automatically.
- No preset grants public catalog publishing.

## Built-In Presets

Box:

- Compact Box
- Wide Box
- Tall Box
- Flat Box

Flat Panel:

- Narrow Panel
- Wide Panel
- Tall Panel
- Short Panel

Sphere:

- Round Sphere
- Squashed Sphere
- Flattened Back Sphere
- Knob-Like Form

Do not use Door Knob naming in this preset set.

## ObjectPlan Boundary

ObjectPlan v1 does not support direct preset reference fields. A reviewed preset
may seed an ObjectPlan node only by expanding its validated property values into
the node. Draft presets, including ObjectPlanDraft presets, cannot seed an
ObjectPlan node through the reviewed-preset helper.

TODO for a later ObjectPlan preset-reference contract:

- validate the preset exists
- validate the preset primitive matches the node primitive
- expand deterministically into property values
- allow property overrides only under an explicit policy

## LLM Boundary

Offline LLMs may suggest draft presets outside the app, but validators enforce
primitive property schemas and review tiers. The app does not run an LLM at
runtime.

## Safety

Preset validation rejects:

- unknown properties
- out-of-domain values
- unsupported primitive kinds
- raw mesh payload fields
- internal/technical product copy
- blocked capability claims

Presets remain property bundles, not public catalog assets by default.
