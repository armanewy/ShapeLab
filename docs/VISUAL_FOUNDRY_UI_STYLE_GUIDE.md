# Visual Foundry UI Style Guide

Wave 31.2 adds a small native design system for Shape Lab's Visual Foundry
surface. It is deliberately product-facing: users choose an asset family,
generate visual directions, adjust meaningful controls, pack variants, and
export assets. Technical implementation concepts stay out of the default UI.

## Product Flow

The default workflow is:

1. Choose
2. Directions
3. Customize
4. Pack
5. Export

Every screen should make the next action visible within five seconds. The center
of the app should be dominated by a whole-model preview or whole-model visual
cards. Isolated part cards are acceptable only when they are clearly secondary
to a whole asset preview.

## Shell

- Top bar: project or family name, saved/dirty state, undo/redo, Save, Export.
- Left rail: Visual Foundry workflow steps, project actions, recent projects.
- Center: whole-model preview, direction board, candidate cards, pack preview.
- Right panel: contextual Customize controls, lock/reset actions, status.
- Bottom strip: readiness, performance, build, and export messages.

At the 1280x800 reference viewport, the shell uses a 222 px left rail, a
392 px right panel, and a center region wide enough for the dominant preview and
six direction cards. On compact widths the right panel may collapse so the
preview and workflow remain usable.

## Theme

The native theme is a restrained dark product UI with blue action accents,
green readiness states, amber warnings, and red destructive states. Cards use
small radii, visible strokes, and stable dimensions. The theme avoids decorative
orbs, marketing gradients, and single-hue purple/blue-purple palettes.

Text contrast must meet at least 4.5:1 for body-sized UI labels. Large labels
may use the 3.0:1 threshold only when they are decorative or secondary, not for
status or controls.

## Components

The shared widget contracts cover:

- App bar actions.
- Step rail items.
- Primary, secondary, quiet, and danger buttons.
- Status pills and status banners.
- Profile, preview, direction, control, and option cards.
- Empty states.
- Inline disabled reasons.
- Progress pulses.
- Key/value rows.
- Action footers.

Disabled actions must provide a plain-language reason. For example:

```text
Add at least one asset before exporting.
Build the current model before exporting.
Choose a direction before customizing.
```

## Copy Rules

Use user-facing terms:

- template
- direction
- option
- control
- preview
- pack
- export
- ready
- saved

Do not expose implementation terms in the default product app:

- Legacy Implicit Mode
- Asset Modeling Lab
- Modeling Workspace
- Advanced Recipe
- raw scalar paths
- provider IDs
- semantic IDs
- operation IDs
- role providers
- conformance bindings
- compiler or decompiler wording

Engineering docs and tests may still use technical terms where they describe
internal contracts. The default product-visible copy inventory is tested
separately.

## Gate Checklist

Before claiming release readiness for UI changes:

- Run `cargo run -p shape-cli -- release-readiness --verify-product-ui-gate`.
- Confirm the product UI gate reports `app_shell: direct_visual_foundry`,
  `startup_blank: false`, zero default-visible pending kits, ten installed
  preview-catalog kits, no default Advanced Recipe, no raw technical terms, and
  pass states for directions, customize, pack, and export.
- Complete [`FOUNDRY_UI_MANUAL_GATE.md`](FOUNDRY_UI_MANUAL_GATE.md) with
  required screenshots for launch, profile selection, directions, customize,
  pack, export, disabled reasons, status, 1280x800, and 1440x900. Notes may add
  context, but cannot replace screenshots.
- For any kit-quality claim, run `shape-cli hq-quality-benchmark` and keep the
  quality tier separate from UI release readiness.
