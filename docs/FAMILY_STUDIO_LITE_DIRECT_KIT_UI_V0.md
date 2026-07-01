# Family Studio Lite Direct Kit UI v0

Date: 2026-07-01

## Status

Family Studio Lite v0 is an internal preview UI for creating local reusable
Direct Kits from supported direct primitives and the supported Panel with Knob
composition. The entry point is hidden by default and appears only when the
developer preview flag is enabled.

## Flow

The preview flow is:

1. Create reusable kit.
2. Start from the current supported shape.
3. Review what stays the same.
4. Toggle bounded controls that can change.
5. Test the kit with deterministic checks.
6. Save Draft or Use Personally.

Supported starting points in this UI gate are:

- current Box Primitive
- current Flat Panel Primitive
- current Sphere Primitive
- current Panel with Knob composition

Supported ObjectPlan Draft evidence remains a contract boundary for Direct
Kits, but this UI gate does not add a public ObjectPlan authoring surface.

## What The UI Shows

- Plain identity copy such as "This stays a box-like primitive."
- Capability cards from the Kit Capability Adapter.
- Bounded property controls and deterministic preset cards.
- Test status that keeps review required.
- Draft and PersonalOnly save outcomes.

The UI does not show kernel, module, provider, slot, topology, raw transform,
generated variation, or candidate wording.

## Test Meaning

"Test kit" means deterministic checks over Direct Kit contracts:

- property endpoint checks
- preset evidence references when available
- supported ObjectPlan evidence references when available
- composition validation for supported safe-anchor compositions
- export-report truth checks when available

Missing visual or export evidence is reported as a warning, not approval.

## Boundaries

Family Studio Lite Direct Kit UI v0 does not include:

- generated candidate trays
- runtime LLM behavior
- broad family authoring
- public catalog publishing
- material editor UI
- UV editing UI
- rigging or animation UI
- game-ready claims

Saved kits remain local Draft or PersonalOnly kits. They are not reviewed,
showcase-ready, public, or game-ready.
