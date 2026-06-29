# Utility Crate

Date: 2026-06-29

Utility Crate is a practical reusable clay crate family between Simple Crate
and Cargo Case. It keeps the crate readable and novice-safe while adding lid,
panel, trim, handle, latch, feet/skid, and detail-density variation.

## Roles

Required roles:

- body
- lid
- panel_fields

Optional reusable roles:

- trim_bands
- handles
- latches
- feet_or_skids
- detail_marks

The default parent uses every optional role, but `none` provider choices may
remove trim, handles, latches, or feet/skids from a variant.

## Primary Controls

Utility Crate has seven primary controls:

- Proportions
- Lid Style
- Panel Style
- Trim Style
- Handle Style
- Latch Detail
- Detail Density

Provider options:

- Lid Style: flat lid, raised lid, rimmed lid
- Panel Style: clean, shallow panels, framed panels
- Trim Style: none, simple band, reinforced band
- Handle Style: none, cutout grip, simple side handle
- Latch Detail: none, simple latch, double latch
- Feet: no feet, small feet, skids

Feet are intentionally not an eighth novice-facing control. Detail Density
selects low, medium, and high detail providers and also moves the internal
feet/skids provider from no feet to small feet to skids.

## Candidate Strategies

- Clean Storage Crate
- Reinforced Utility Crate
- Compact Carry Crate
- Wide Supply Crate
- Lidded Field Crate
- Minimal Workshop Crate

At least four generated ideas are expected to be visibly distinct in pure clay.

## Family Ladder

Utility Crate is richer than Simple Crate because it adds panel fields, optional
handles, latches, and detail-density/feet enrichment while preserving a small
novice control surface.

Utility Crate is simpler than Cargo Case because it omits Cargo Case systems
such as corner guards, vents, fastener sets, utility rails, side grilles, label
plates, hinge details, and multi-profile style bias.

## Scope

Included geometry is clay-only primitive crate geometry. The family does not
include browser code, server code, natural-language modeling, LLM runtime
integration, Blender integration, imported mesh editing, rigging, animation,
surface authoring, UV unwrapping, texture maps, decals, logos, or material-map
claims.

## Evidence

This branch writes evidence to `target/utility-crate-family-v1/`:

- `parent.png`
- `candidate-contact-sheet.png`
- `control-endpoint-sheet.png`
- `comparison-simple-vs-utility.png`
- `quality-report.json`

The profile is novice-catalog visible only because Utility Crate v1 evidence
passes the starter-template gate: seven readable controls, at least four
distinct ideas, no TooSubtle whole-asset candidates, no floating/broken parts,
and clean export conformance.
