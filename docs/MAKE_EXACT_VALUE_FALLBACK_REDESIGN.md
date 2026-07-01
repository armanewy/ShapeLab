# Make Exact-Value Fallback Redesign

Date: 2026-07-01

## Result

Direct primitive Make now uses a compact two-column exact-value fallback panel
while object-anchored Orchard handles are still future work. The fallback keeps
bounded sliders usable without presenting the right panel as the primary long
term editing surface.

## Covered Workflows

- Box Primitive: Width, Depth, Height, and Edge Softness.
- Flat Panel Primitive: Width, Height, Thickness, and Edge Softness.
- Sphere Primitive: Width, Height, Depth, Front Flatten, Back Flatten, and the
  Knob-like form preset.
- Panel with Knob: panel values, knob size/flatten values, and safe knob
  position values.

## UX Rules

- Direct handles remain the target primary interaction.
- Exact values are a fallback for precision and current usability.
- User-facing control descriptions use plain shape language, not repeated
  generic property copy.
- Add to Pack and Export remain visible in the Make panel when the current
  asset is ready.
- Stale background candidate-result warnings remain suppressed for direct
  primitive workflows.

## Screenshot Evidence

Expected screenshot evidence paths:

- `target/make-exact-value-fallback-redesign/screenshots/box_exact_values_compact.png`
- `target/make-exact-value-fallback-redesign/screenshots/flat_panel_exact_values_compact.png`
- `target/make-exact-value-fallback-redesign/screenshots/sphere_exact_values_compact.png`
- `target/make-exact-value-fallback-redesign/screenshots/panel_knob_exact_values_compact.png`
- `target/make-exact-value-fallback-redesign/screenshots/box_ready_actions.png`

## Non-Goals

This branch does not add full Orchard handles, generated variation UI, runtime
LLM integration, material editor UI, UV editing, rigging, animation, public
catalog publishing, or game-ready claims.
