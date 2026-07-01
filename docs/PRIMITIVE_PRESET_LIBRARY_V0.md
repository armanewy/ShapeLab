# Primitive Preset Library v0

Date: 2026-07-01

Primitive presets are deterministic bundles of legal primitive property values.
They are the safe replacement path for the old generated-variation workflow.

## Scope

Primitive Preset Library v0 supports reviewed built-in presets for:

- Box Primitive
- Flat Panel Primitive
- Sphere Primitive

Each preset stores:

- a stable preset ID
- a product-facing display name
- one approved primitive kind
- property values validated against that primitive schema
- product-safe user description and use tags
- review tier
- source

## Built-In Presets

Box Primitive:

- Compact Box
- Wide Box
- Tall Box
- Flat Box

Flat Panel Primitive:

- Narrow Panel
- Wide Panel
- Tall Panel
- Short Panel

Sphere Primitive:

- Round Sphere
- Squashed Sphere
- Flattened Back Sphere
- Knob-Like Form

The library deliberately does not use Door Knob naming.

## Validation Rules

Every preset must validate against the primitive property schema for its
primitive kind. Validation rejects unknown properties, out-of-domain values,
unsupported primitive kinds, raw payload fields, unsafe copy, and unsupported
capability claims.

Presets cannot contain raw mesh data. Presets cannot publish to a public
catalog automatically. Built-in presets are reviewed local property bundles,
not public catalog assets.

## ObjectPlan Use

ObjectPlan may use a reviewed preset by copying the preset's validated property
values into an ObjectPlan node. Draft presets, including future ObjectPlanDraft
or offline LLM suggestions, must be reviewed before they can seed a plan node.

LLMs may suggest presets later, but validators enforce primitive schemas and
review gates. Runtime LLM integration is not part of this library.
