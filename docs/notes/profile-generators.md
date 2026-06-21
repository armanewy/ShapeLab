# Profile Generators Notes

## Scope

This branch adds the first explicit profile-generator implementation for the
part-aware modeling lane:

- sweep of a closed 2D profile along ordered 3D path samples
- lathe of a radius/height profile around a local Y axis frame

The implementation lives in `crates/shape-modeling/src/generators/profile.rs`.
The root modeling dispatch stubs are intentionally unchanged; the existing
`shape-asset::GeometrySource::Sweep` and `GeometrySource::Lathe` variants do not
yet carry all Prompt 1.4 controls such as cap mode, roll, angular span, or seam
mode.

## Sweep Behavior

- Builds deterministic parallel-transport frames from path samples.
- Uses an explicit up hint for the initial frame and rejects hints parallel to
  the initial tangent.
- Accepts optional per-sample scale and roll values.
- Generates ring-to-ring quads with deterministic vertex and face ordering.
- Supports capped, uncapped, open, and closed paths.
- Rewinds closed profile input to a stable outward winding.
- Adds side, start cap, end cap, and corner regions where applicable.
- Adds start and end sockets.
- Adds edge metadata for hard profile features, open boundaries, region
  transitions, smooth edges, and seam candidates.

## Lathe Behavior

- Generates full and partial revolutions from ordered radius/height profiles.
- Uses the provided axis frame; local Y is the rotation axis.
- Produces quad strips for non-axis profile spans.
- Handles profile points on the axis with a single shared axis vertex so rings
  do not collapse.
- Generates triangular side faces only where an axis-touching span makes a quad
  topologically invalid.
- Supports profile end caps where the endpoint radius is non-zero.
- Adds stable seam regions and seam edge metadata for partial revolutions.
- Adds top and bottom sockets from the profile height range.

## Validation

The profile generators reject non-finite coordinates, collapsed profile edges,
zero-area sweep profiles, collapsed path rings, invalid up hints, negative lathe
radii, too few radial segments, invalid angular spans, and adjacent axis-only
lathe segments.
