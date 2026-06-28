# Stylized Lamp Dogfood Hardening v3

Decision: `Usable`.

## Rationale

The Stylized Lamp starter remains Usable because the default lamp prepares successfully, exposes a complete quick-control surface, and generates readable candidate ideas. The fixture uses authored lathe, sweep, primitive, and bevel operations only; the dogfood test now rejects SDF/remeshing fallback and keeps the prepared preview under a compact part and triangle budget.

## Quick Controls

The visible primary quick controls are:

- Overall Height
- Base Weight
- Stem Curvature
- Joint Size
- Shade Style
- Shade Scale
- Edge Softness

All seven controls have authored initial state, required geometry bindings, and either a continuous domain or the six-option Shade Style gallery.

## Idea Legibility

The v3 tests require the authored lamp directions and generated Explore candidates to show differences across the macro axes that matter in dogfood review:

- height
- shade style
- shade scale
- base weight
- stem curvature

The strategy labels are:

- Compact Task Lamp
- Tall Reading Lamp
- Playful Curved Lamp
- Heavy Base Lamp
- Minimal Studio Lamp
- Wide Shade Lamp

Each strategy can vary Overall Height, Base Weight, Stem Curvature, Shade Style, and Shade Scale. The compiled authored states must still produce at least four distinct whole-model silhouettes, at least four height bands, readable shade/body extents, readable base footprints, and readable stem curvature bands.

## PreviewOnly Fallback

No downgrade was applied. If a future change breaks preparation, removes meaningful quick controls, or fails the four-visible-ideas/readable macro-axis tests, `stylized-lamp` should be moved to `PreviewOnly` before it is exposed as a novice starter again.
