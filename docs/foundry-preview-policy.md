# Foundry Preview Policy

Foundry previews are CPU-rendered whole-model images. Candidate cards, slider
filmstrips, integer and discrete strips, provider galleries, and changed-role
overlays all render the complete model mesh for the sampled state.

## Comparison Sets

Every preview in a comparison set uses one shared camera. When the caller does
not supply a camera, the renderer fits the camera to the union of all complete
model bounds in the set. This keeps scale and framing stable across candidate
cards, filmstrip samples, discrete options, and provider-gallery options.

Preview output is square and limited to these sizes:

- 64x64
- 96x96
- 128x128

The selected preview resolution is authoritative. The base render settings are
copied, then width and height are replaced by the selected square size.

## Cache Key

The in-memory preview cache key includes:

- document geometry fingerprint
- sampled control state
- provider choices
- shared camera
- render settings after resolution is applied
- preview resolution

Changed-role overlay metadata is not part of the base image cache key. Cached
base-image reuse must still compose overlay pixels and return overlay metadata
from the current request.

## Cache Behavior

The cache is bounded in memory and uses least-recently-used eviction. Cache hits
move an entry to the most-recently-used end of the order. Inserting past the
configured capacity evicts least-recently-used entries until the cache is back
within capacity. A capacity of zero disables storage while still allowing
previews to render.

## Rendering Order

Render work may run in bounded parallel batches, but results are returned in
the exact input order. Parallelism is an implementation detail and must not
change filmstrip order, gallery option order, candidate-card order, or changed
role overlay metadata association.
