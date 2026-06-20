# Wave 3 Viewport Notes

## Interaction Mappings

- Primary-button drag orbits the camera.
  - Horizontal motion changes yaw at 0.35 degrees per point.
  - Upward motion raises pitch; pitch and yaw are clamped by `shape-render` camera logic.
- Secondary-button drag pans the camera target in camera-right and camera-up space.
  - Pan speed is derived from camera distance, vertical field of view, and viewport height so it scales with zoom.
- Mouse wheel zoom scales camera distance.
  - One octave is 240 scroll points.
  - Per-event zoom factors are clamped to 0.2 through 5.0 before applying `OrbitCamera::zoom`.
- Double-click and hovered `F` emit `FitToObject`.
- Hovered `Home` emits `ResetCamera`, sets the camera to `OrbitCamera::default`, and requests a final rerender.

## Render Request Policy

- Camera math is computed immediately in the viewport layer and returned with `ViewportAction` values.
- Dragging requests low-resolution interactive renders at a throttled cadence.
- Ending a drag, wheel zooming, and keyboard camera reset request full-resolution renders.
- Resize-driven rerenders are debounced and preserve the latest visible image while the new render is pending.
- Render requests carry the target pixel size and camera so the application coordinator can translate them into background jobs.

## UI Decisions

- The widget draws the latest `egui::TextureHandle` without stretching its aspect ratio.
- A neutral checker placeholder is shown when no image is available.
- The viewport paints lightweight overlays for project title, revision, selected node, job progress, recoverable errors, rendering state, and a compact interaction hint.
- The viewport does not perform picking, gizmo editing, geometry work, file I/O, or GPU rendering.

## Contract Issues

- `shape-app` currently has only a binary target, so `viewport_tests.rs` includes `src/viewport.rs` directly with `#[path = "../src/viewport.rs"]`.
- The existing `ViewportAction` stub did not carry render sizes. This branch extends render request actions with `ViewportRenderRequest` so resize and interactive render decisions are explicit for the Wave 4 integrator.
