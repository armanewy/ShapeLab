# Choose and Make UI Redesign Plan

Date: 2026-07-01

## Design stance

The next UI pass should use the mockups as visual inspiration, but it must obey
the product rules first:

- direct primitive editing before generated suggestions;
- object-anchored controls before inspector sliders;
- deterministic presets before candidate generation;
- safe anchors before freeform object transforms;
- review/private storage before public publishing.

## Recommended direction

Use a hybrid of the provenance-sidebar and focused-canvas concepts:

- dark Object Orchard shell;
- warm studio canvas;
- left provenance/library rail on Choose;
- full-stage Make canvas;
- compact right-side exact-value drawer only as fallback;
- direct Orchard handles on the model as the primary editing surface.

## Choose page target

The Choose page should group starting points by provenance:

```text
Primitives
  Box Primitive
    Lidded Box        derived from Box Primitive + Lid Seam
    Trimmed Box       internal evidence only

  Flat Panel Primitive
    Hinged Panel      derived from Flat Panel + Hinge Edge
    Panel with Knob   derived from Flat Panel + Sphere attachment

  Sphere Primitive
    Knob-like Form    preset, not an asset family
```

V0 does not need full shape-decompiler visualization. A `derived from` label and
light ancestry indentation are enough. Decompiler diagrams become useful later
when provenance spans many user-authored steps.

## Make page target

The Make page should center the object and show interaction on the object:

- no permanent coordinate grid;
- no red/green axis line crossing the object by default;
- soft studio background or stage plate;
- subtle contact shadow;
- optional tiny orientation triad;
- visible object-anchored handles for active controls;
- exact-value inspector collapsed or secondary.

Primary actions should be obvious:

- Add to Pack
- Export
- Reset working copy
- Save personal kit when in Family Studio Lite

## Control grammar

Every user-facing tunable must map to the finite Orchard control grammar:

- stretch handles;
- profile/corner handles;
- band handles;
- pattern handles;
- attachment anchors;
- option chips;
- precision fallback.

This grammar keeps the product aligned with kit-based, bounded asset creation.
It is not a Blender clone and should not expose raw mesh operations.

## Immediate UX sequence

Recommended implementation order:

1. Apply direct Make stale-warning correctness fix.
2. Redesign Choose around provenance and grouped starting points.
3. Clean up the Make stage so it is warm, centered, and not grid-first.
4. Make the exact-value fallback compact and useful.
5. Add Orchard stretch handles for Width / Height / Depth / Thickness.

## Explicit non-goals

This redesign must not add:

- generated variation trays;
- runtime LLM integration;
- public catalog publishing;
- material editor UI;
- UV editing UI;
- rigging or animation UI;
- game-ready claims;
- free transform gizmos;
- vertex or face editing.
