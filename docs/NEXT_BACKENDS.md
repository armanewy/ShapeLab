# Next Backends

## Cage And Mesh Deformation

Wave 25 adds the first prepared known-base character customization contract in
`shape-character::prepared`. It validates authored deformation cages, weights,
landmark constraints, base fingerprints, and the six novice-facing whole-model
controls before emitting a deterministic no-raw-mesh customization program.

Next steps for this backend are prop templates, app/history integration, and
candidate generation over the same prepared controls. It must continue to
clearly distinguish prepared templates from raw unstructured imports.

## Structural Graph Mutations

Extend candidates from scalar edits to safe graph operations such as adding repeated supports, replacing a component family, or changing a CSG relationship. This requires stronger validation, previews, and user-facing explanations before it belongs in the main loop.

## Curve Backend

Represent cables, handles, foliage, rails, profiles, and architectural strokes as editable curves or swept surfaces. Curve parameters can participate in the same Refine/Explore search as shape-graph scalars.

## GPU Field Sampling And Rendering

Move field sampling, meshing support data, thumbnails, and viewport rendering toward `wgpu` once the interaction loop is stable. The CPU path should remain a deterministic reference implementation.

## Preference Learning

Persist user choices as pairwise comparisons and use them to bias future candidate generation toward accepted directions while still preserving novelty and exploration.

Wave 29 adds the first local-only version of this backend. Foundry sessions can
record explicit accept/reject, lock/reset, export, and pack-membership signals as
visible control IDs. Candidate generation may consume a same-scope
`FoundryPreferenceProfile`, but the profile contributes only a bounded
post-validation selection bonus. It does not mutate semantic documents, store
geometry or paths, or replace novelty/diversity gates.

## Optional DCC Adapter

Add an export/import adapter for a DCC only after the core model remains useful without it. The adapter should project Shape Lab state outward rather than making external scene files the source of truth.

Wave 28 adds the output side of this boundary to canonical model packages:
`dcc-adapter.json`, `dcc_rebuild.py`, and `dcc-verification.json`. These sidecars
project semantic parts, collections, metadata, and variant-control labels outward
while recording that DCC scenes are not source documents and edited DCC scene
import is unsupported.
