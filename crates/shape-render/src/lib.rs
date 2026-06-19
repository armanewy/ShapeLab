#![forbid(unsafe_code)]

//! CPU preview renderer contracts.

use glam::Vec3;
use serde::{Deserialize, Serialize};
use shape_core::Aabb;
use shape_mesh::TriangleMesh;
use thiserror::Error;

/// Orbit camera.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrbitCamera {
    /// Camera target point.
    pub target: Vec3,
    /// Horizontal orbit angle.
    pub yaw_degrees: f32,
    /// Vertical orbit angle.
    pub pitch_degrees: f32,
    /// Distance from target.
    pub distance: f32,
    /// Vertical field of view.
    pub vertical_fov_degrees: f32,
}

impl Default for OrbitCamera {
    fn default() -> Self {
        Self {
            target: Vec3::ZERO,
            yaw_degrees: 35.0,
            pitch_degrees: 25.0,
            distance: 4.0,
            vertical_fov_degrees: 45.0,
        }
    }
}

/// Render settings.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RenderSettings {
    /// Output width in pixels.
    pub width: u32,
    /// Output height in pixels.
    pub height: u32,
    /// Background RGBA.
    pub background: [u8; 4],
    /// Ambient light amount.
    pub ambient: f32,
    /// Directional light.
    pub light_direction: Vec3,
    /// Draw wireframe overlay.
    pub wireframe: bool,
}

impl Default for RenderSettings {
    fn default() -> Self {
        Self {
            width: 512,
            height: 512,
            background: [24, 26, 28, 255],
            ambient: 0.35,
            light_direction: Vec3::new(-0.4, -0.8, -0.3).normalize(),
            wireframe: false,
        }
    }
}

/// Rendered image buffer.
#[derive(Debug, Clone, PartialEq)]
pub struct RenderedImage {
    /// Width in pixels.
    pub width: u32,
    /// Height in pixels.
    pub height: u32,
    /// RGBA8 pixel data.
    pub rgba8: Vec<u8>,
}

/// Render errors.
#[derive(Debug, Error)]
pub enum RenderError {
    /// The requested operation belongs to a later wave.
    #[error("not implemented: {0}")]
    NotImplemented(&'static str),
}

/// Fit an orbit camera to bounds.
#[must_use]
pub fn fit_camera_to_bounds(bounds: Aabb) -> OrbitCamera {
    let extent = bounds.extent();
    OrbitCamera {
        target: bounds.center(),
        distance: extent.length().max(1.0) * 1.8,
        ..OrbitCamera::default()
    }
}

/// Render a mesh to an RGBA8 image.
pub fn render_mesh(
    _mesh: &TriangleMesh,
    _camera: &OrbitCamera,
    settings: &RenderSettings,
) -> Result<RenderedImage, RenderError> {
    let pixel_count = settings.width.saturating_mul(settings.height) as usize;
    let mut rgba8 = Vec::with_capacity(pixel_count.saturating_mul(4));
    for _ in 0..pixel_count {
        rgba8.extend_from_slice(&settings.background);
    }
    Ok(RenderedImage {
        width: settings.width,
        height: settings.height,
        rgba8,
    })
}
