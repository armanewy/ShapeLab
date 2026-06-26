# Focus Part Surface Readiness Integration

This document records the integration boundary between Focus Part modeling,
headless Surface evidence, and rig/motion readiness contracts.

The purpose is to keep the novice Visual Foundry workflow honest.

Focus Part is visual modeling guidance.

Surface evidence is headless package evidence.

Rig and motion are readiness contracts only.

## Merged Branches

The integration branch combines these branches:

- `codex/surface-preview-rig-motion-readiness`
- `codex/focus-part-perceptual-variation-v0`

The surface branch adds Sci-Fi Crate textured preview evidence.

It also adds material-only variants, surface delta reports,
surface-aware package data, and rig/motion readiness contracts.

The Focus Part branch adds semantic part-group targeting.

It also adds focused modeling candidates, render-based visible-delta gates,
and model-first UI affordances.

## Readiness Boundaries

Shape Lab distinguishes four separate readiness states.

1. Static surface package availability:

   The Sci-Fi Crate export can emit UVs.

   It can emit material slots.

   It can emit procedural texture files.

   It can emit evidence images and validation reports.

2. Headless surface visual evidence:

   Textured preview images exist.

   Contact sheets exist.

   Material-variant delta reports exist.

   These artifacts exist outside the novice app workflow.

3. Visual Foundry Surface variation:

   This is still unavailable.

   It remains unavailable until the app can show and compare textured
   material/surface candidates.

4. Focus Part Surface editing:

   This is still unavailable.

   It remains unavailable until part-specific surface candidate generation
   and preview evidence exist.

## Delta Boundary

Headless material-only variants do not automatically become Visual Foundry
candidates.

Shape candidates cannot pass by material or surface deltas alone.

Surface variants cannot claim shape changes.

Complete Looks may combine shape and surface later only when both visual
channels are supported in the app.

Hidden or internal changes do not count as user-facing variation.

## Product Boundary

The app may say:

- Static prop surface package available.
- Exports a frozen crate mesh with UVs.
- Exports material slots.
- Exports simple procedural texture files.
- Exports evidence images.
- Exports a validation report.
- Still blocked from full game-ready status until manual review.
- Still blocked until engine import proof exists.
- Still blocked until engine-native package handoff is complete.

The app must not claim:

- textured Visual Foundry previews;
- Surface mode readiness;
- game-ready output;
- rigging;
- skinning;
- animation;
- retargeting;
- engine-native Unity packages;
- engine-native Unreal packages;
- engine-native Godot packages.

## Rig And Motion

Rig and motion artifacts are contracts only.

They are validation surfaces only.

They may appear in docs.

They may appear in schemas.

They may appear in package metadata.

They may appear in headless validation.

They are not novice UI features.

They do not imply rigging support.

They do not imply skinning support.

They do not imply animation curve generation.

They do not imply humanoid retargeting support.

## Focus Part

Focus Part remains semantic part-group targeting.

It is not raw mesh-part editing.

The v0 interaction uses part chips and product-safe labels.

True mesh picking still requires semantic masks or hit testing in a later
branch.

## Integration Rule

Headless evidence can make exports more inspectable.

Headless evidence does not unlock a novice-facing mode by itself.

A Visual Foundry mode becomes available only when the app can show the relevant
candidates at product scale.

A Visual Foundry mode also needs validation that the visible differences are
real without exposing backend terminology.
