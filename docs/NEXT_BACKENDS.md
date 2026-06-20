# Next Backends

## Cage And Mesh Deformation

Add a backend for prepared imported meshes using deformation cages, weights, and landmark constraints. It should share the same project history, candidate, lock, and search abstractions, while clearly distinguishing prepared meshes from raw unstructured imports.

## Structural Graph Mutations

Extend candidates from scalar edits to safe graph operations such as adding repeated supports, replacing a component family, or changing a CSG relationship. This requires stronger validation, previews, and user-facing explanations before it belongs in the main loop.

## Curve Backend

Represent cables, handles, foliage, rails, profiles, and architectural strokes as editable curves or swept surfaces. Curve parameters can participate in the same Refine/Explore search as shape-graph scalars.

## GPU Field Sampling And Rendering

Move field sampling, meshing support data, thumbnails, and viewport rendering toward `wgpu` once the interaction loop is stable. The CPU path should remain a deterministic reference implementation.

## Preference Learning

Persist user choices as pairwise comparisons and use them to bias future candidate generation toward accepted directions while still preserving novelty and exploration.

## Optional DCC Adapter

Add an export/import adapter for a DCC only after the core model remains useful without it. The adapter should project Shape Lab state outward rather than making external scene files the source of truth.

