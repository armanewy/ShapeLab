# Foundry Make Canvas Product Stack

This branch is the base for the next Visual Foundry product-quality stack.

It is not ready for `main`.

## Included Stack

This stack is based on `codex/fix-foundry-legibility-and-crate-ui`.

It includes:

- Surface channel integration.
- Focus Part and perceptual variation work.
- Surface preview and rig/motion readiness work.
- The model-centric Visual Foundry UI pass.
- The Foundry legibility and crate UI fix.

## Current Blockers

Generated directions are not visually legible enough.

Directions and Customize are still separate primary screens, but they should
become one Make canvas.

Sci-Fi Crate provider geometry is still too weak and samey for the product
promise.

Screenshot and video verification are required before any merge to `main`.

## Product Boundaries

Surface Lab remains headless unless textured candidates are visually shown in
the app.

Rig and motion readiness remain hidden from the novice UI.

The novice product must expose curated visual choices, not internal modeling
machinery.

## Merge Rule

Do not merge this stack to `main` until:

- the Make canvas replaces the Directions/Customize split;
- whole-asset candidates are readable at app size;
- focused part changes are obvious;
- weak profiles are hidden from novice users;
- Surface remains honest and unavailable unless visual candidates exist;
- rig/motion contracts remain hidden;
- screenshot or video verification passes.
