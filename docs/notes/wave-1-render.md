# Wave 1 Render Notes

## Implemented

- Deterministic CPU triangle rasterizer in `shape-render`.
- Orbit camera clamping, yaw normalization, view/projection matrices, orbit, pan, zoom, and bounds fitting.
- RGBA8 top-left row-major image output with pixel access helpers.
- Depth-buffered triangle rasterization with near-plane clipping, backface culling, perspective-correct depth and normal interpolation, Lambert plus ambient shading, neutral material color, and optional wireframe overlay.
- Validation for dimensions, allocation size, camera finiteness, mesh positions, and triangle indices.
- Focused unit tests for camera fitting, triangle rendering, depth, culling, image layout, empty meshes, camera clamps, determinism, and malformed meshes.

## Assumptions and Limitations

- `RenderSettings::light_direction` is interpreted as the direction light rays travel; shading uses `normal dot -light_direction`.
- The renderer clips against the near plane but only rejects triangles outside the viewport by whole-triangle side tests; viewport edge clipping is handled by clamped raster bounds.
- Triangle fill uses a deterministic inclusive barycentric rule. It favors avoiding cracks in MVP thumbnails over exact top-left ownership of shared edges.
- There is no `image::RgbaImage` conversion method because `shape-render` does not currently depend on `image`; adding that dependency should be handled by an integration branch if needed.
- No custom GPU renderer, external 3D engine, unsafe code, or continuous rendering loop was added.

## Contract Issues

- No public contract changes were required. New camera and image helper methods are additive.
