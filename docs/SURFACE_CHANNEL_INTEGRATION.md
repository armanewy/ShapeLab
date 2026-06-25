# Surface Channel Integration

This integration connects the Foundry variation/channel vocabulary to the
Surface Lab v1 static-prop package path without making Surface a visual
direction mode.

## Boundary

- Foundry `VariationScope` and `VariationChannel` remain the product/control
  abstraction.
- Surface Lab v1 remains a headless backend artifact provider.
- The Sci-Fi Crate static-prop export can report that a surface package is
  available.
- Surface visual variation remains unavailable until Shape Lab can render and
  compare textured/material candidate previews.
- Focus Part Surface remains unavailable until part-specific surface editing
  exists.

## Current State

Sci-Fi Crate has a static-prop surface package path. The package contains UVs,
material slots, simple procedural texture files, evidence images, validation
reports, and a surface-aware GLB sidecar. This is package evidence, not a
Visual Foundry textured preview workflow.

The Directions board must keep Complete Looks as the default mode. Shape may
remain available where candidate generation supports visible geometry,
structure, or silhouette changes. Surface must stay disabled with this reason:

```text
Surface package export exists for this kit. Visual surface variation needs textured previews and material candidate support.
```

## Sidecar Mapping

`surface/surface-capabilities.json` maps into a product-facing
`FoundrySurfaceCapabilityView`. The parser validates schema shape, rejects
absolute local paths, converts texture channel keys to plain labels, and never
enables Surface mode from raw sidecar strings alone.

Capability mapping is intentionally limited:

- `surface` maps to `VariationChannel::Surface` as capability evidence only.
- `wear` maps to `VariationChannel::Wear` as capability evidence only.
- material slot evidence maps to material-slot capability counts only.
- part-surface evidence cannot enable Focus Part Surface without actual
  part-specific surface editing support.

## Delta Separation

Shape and Surface deltas stay separate. Shape candidates cannot pass by relying
on surface delta. Surface candidates cannot claim shape changes. Complete Looks
may eventually combine shape and surface once both are visually supported.

## Export Copy

The Export screen may say:

```text
Static prop surface package available
```

It may describe the Sci-Fi Crate export as:

```text
Exports a frozen crate mesh with UVs, material slots, simple procedural texture files, evidence images, and a validation report.
```

It must also keep the blocked-full-ready note:

```text
Still blocked from full game-ready status until manual review, engine import proof, and engine-native package handoff are complete.
```

The app must not imply that all profiles now have UVs/textures, that Visual
Foundry previews are textured, or that Unity, Unreal, or Godot package export
exists.

