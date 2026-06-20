//! Viewport action contract.

#![allow(dead_code)]

use shape_render::OrbitCamera;

/// UI action emitted by the viewport widget.
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum ViewportAction {
    Orbit { delta_yaw: f32, delta_pitch: f32 },
    Pan { delta_right: f32, delta_up: f32 },
    Zoom { factor: f32 },
    SetCamera(OrbitCamera),
    FitToObject,
    ResetCamera,
    RequestInteractiveRender,
    RequestFinalRender,
}
