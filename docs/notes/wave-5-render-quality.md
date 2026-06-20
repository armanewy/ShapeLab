# Wave 5 Render Quality Notes

## Implemented

- Added `RenderWorkspace` for repeated CPU renders that can reuse the depth buffer and caller-provided image allocation.
- Added `RenderCacheKey` and `RenderCache` so callers can reuse the last rendered image when mesh, clamped camera, and effective render settings are unchanged.
- Replaced near-plane clipping allocation with a fixed four-vertex clipped triangle.
- Added early view-depth/frustum rejection before raster fan emission.
- Moved viewport pixel bounds into a reusable helper so raster loops stay clamped to visible pixels.
- Made near-plane projection accept vertices clipped exactly onto the near plane.
- Aligned supplied vertex normals to the triangle face before smooth interpolation, reducing dark facets from inconsistent normal signs.
- Improved bounds fitting with orientation-aware projected extents and added `fit_camera_to_bounds_with_aspect`.
- Raised default material contrast slightly for small thumbnails while retaining the CPU renderer and existing lighting model.
- Lowered interactive viewport render cap from 512 to 384 pixels on the longest side and added a subtle viewport reference grid overlay.

## Tests

- Added cache-key tests covering stable repeated keys, equivalent clamped/normalized inputs, mesh invalidation, camera invalidation, settings invalidation, and cache hit/miss behavior.

## Bench Observations

- Before edits, `Measure-Command { cargo test -p shape-render | Out-Host }` reported 7121.41 ms while the workspace was still compiling and waiting on Cargo file locks.
- After edits, a warm `Measure-Command { cargo test -p shape-render | Out-Host }` reported 341.07 ms with dependencies already built.
- These are command timing observations only. They are not a controlled renderer benchmark and should not be read as a measured render-speed improvement.

## Integration Boundaries

- App-level render-cache reuse was not wired into `shape-app` because the relevant job/state modules are outside this worker's ownership.
- `RenderSettings::wireframe` already exists for the optional wireframe path, but adding a UI toggle would require editing non-owned app modules.
- No GPU renderer, external renderer, dependency, or root manifest change was added.
