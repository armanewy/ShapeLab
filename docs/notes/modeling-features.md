# Semantic Modeling Features

Prompt 3.1 adds first-class constructive detail feature builders in
`shape-modeling` without changing the shared asset schema or implicit-editor
paths.

## Scope

- `PanelFeature` builds raised or recessed panels on named planar sockets or
  authored planar region frames.
- `TrimFeature` sweeps a closed profile around declared edge loops or authored
  paths with offset, roll, and start/end cap controls.
- `RibFeature` creates repeated plate/profile ribs either as shared-definition
  instances or one combined generated part.
- `FastenerPattern` creates a cylinder/frustum prototype once and places
  repeated instances in linear, radial, or perimeter patterns.

## Contract

- Feature geometry is generated as explicit polygon topology, not as generic
  mesh booleans.
- Host geometry is never silently fused with feature geometry.
- Every generated face carries a semantic region, surface role, and operation
  provenance.
- Placement and parameter failures return feature validation errors.
- Region IDs and operation provenance stay stable across scalar parameter
  changes; polygon element IDs remain deterministic for a given topology.
- The module does not define texture, material, UV, bevel quality, export, or
  validation-pipeline behavior.
