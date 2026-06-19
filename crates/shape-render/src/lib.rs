#![forbid(unsafe_code)]

//! Deterministic CPU preview renderer.
//!
//! The renderer writes row-major RGBA8 pixels with `(0, 0)` at the top-left
//! of the image. Triangle depth and normals are perspective-correctly
//! interpolated from screen-space barycentric coordinates before shading.

use glam::{Mat4, Vec2, Vec3};
use serde::{Deserialize, Serialize};
use shape_core::Aabb;
use shape_mesh::TriangleMesh;
use thiserror::Error;

const MIN_PITCH_DEGREES: f32 = -89.0;
const MAX_PITCH_DEGREES: f32 = 89.0;
const MIN_DISTANCE: f32 = 0.05;
const MAX_DISTANCE: f32 = 1_000_000.0;
const MIN_FOV_DEGREES: f32 = 5.0;
const MAX_FOV_DEGREES: f32 = 120.0;
const NEAR_PLANE: f32 = 0.01;
const FAR_PLANE: f32 = 10_000_000.0;
const MAX_PIXELS: u64 = 16_777_216;
const MIN_AREA: f32 = 1.0e-5;
const MIN_NORMAL_LENGTH_SQUARED: f32 = 1.0e-12;
const EDGE_EPSILON: f32 = 1.0e-4;
const WIRE_DISTANCE_PIXELS: f32 = 0.85;
const MATERIAL_COLOR: Vec3 = Vec3::new(184.0, 190.0, 182.0);
const WIREFRAME_MIX: Vec3 = Vec3::new(34.0, 36.0, 38.0);

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

impl OrbitCamera {
    /// Return a camera with finite, normalized orbit parameters.
    #[must_use]
    pub fn clamped(&self) -> Self {
        let default = Self::default();
        let target = if is_finite_vec3(self.target) {
            self.target
        } else {
            Vec3::ZERO
        };
        let yaw_degrees = if self.yaw_degrees.is_finite() {
            normalize_degrees(self.yaw_degrees)
        } else {
            default.yaw_degrees
        };
        let pitch_degrees = if self.pitch_degrees.is_finite() {
            self.pitch_degrees
                .clamp(MIN_PITCH_DEGREES, MAX_PITCH_DEGREES)
        } else {
            default.pitch_degrees
        };
        let distance = if self.distance.is_finite() {
            self.distance.clamp(MIN_DISTANCE, MAX_DISTANCE)
        } else {
            default.distance
        };
        let vertical_fov_degrees = if self.vertical_fov_degrees.is_finite() {
            self.vertical_fov_degrees
                .clamp(MIN_FOV_DEGREES, MAX_FOV_DEGREES)
        } else {
            default.vertical_fov_degrees
        };

        Self {
            target,
            yaw_degrees,
            pitch_degrees,
            distance,
            vertical_fov_degrees,
        }
    }

    /// Return the world-space camera eye position.
    #[must_use]
    pub fn eye(&self) -> Vec3 {
        let camera = self.clamped();
        let yaw = camera.yaw_degrees.to_radians();
        let pitch = camera.pitch_degrees.to_radians();
        let pitch_cos = pitch.cos();
        let offset = Vec3::new(pitch_cos * yaw.sin(), pitch.sin(), pitch_cos * yaw.cos());
        camera.target + offset * camera.distance
    }

    /// Return the right-handed view matrix for this camera.
    #[must_use]
    pub fn view_matrix(&self) -> Mat4 {
        let camera = self.clamped();
        Mat4::look_at_rh(camera.eye(), camera.target, Vec3::Y)
    }

    /// Return a right-handed perspective projection matrix.
    #[must_use]
    pub fn projection_matrix(&self, aspect_ratio: f32) -> Mat4 {
        let camera = self.clamped();
        let aspect_ratio = if aspect_ratio.is_finite() && aspect_ratio > 0.0 {
            aspect_ratio
        } else {
            1.0
        };
        Mat4::perspective_rh_gl(
            camera.vertical_fov_degrees.to_radians(),
            aspect_ratio,
            NEAR_PLANE,
            FAR_PLANE,
        )
    }

    /// Return projection multiplied by view for this camera.
    #[must_use]
    pub fn view_projection_matrix(&self, aspect_ratio: f32) -> Mat4 {
        self.projection_matrix(aspect_ratio) * self.view_matrix()
    }

    /// Orbit around the target in degrees, then clamp pitch and normalize yaw.
    pub fn orbit(&mut self, delta_yaw_degrees: f32, delta_pitch_degrees: f32) {
        if delta_yaw_degrees.is_finite() {
            self.yaw_degrees += delta_yaw_degrees;
        }
        if delta_pitch_degrees.is_finite() {
            self.pitch_degrees += delta_pitch_degrees;
        }
        *self = self.clamped();
    }

    /// Pan the target along the camera's right and up vectors.
    pub fn pan(&mut self, right_delta: f32, up_delta: f32) {
        if !right_delta.is_finite() || !up_delta.is_finite() {
            *self = self.clamped();
            return;
        }

        let camera = self.clamped();
        let eye = camera.eye();
        let forward = normalize_or(camera.target - eye, Vec3::NEG_Z);
        let right = normalize_or(forward.cross(Vec3::Y), Vec3::X);
        let up = normalize_or(right.cross(forward), Vec3::Y);
        self.target = camera.target + right * right_delta + up * up_delta;
        self.yaw_degrees = camera.yaw_degrees;
        self.pitch_degrees = camera.pitch_degrees;
        self.distance = camera.distance;
        self.vertical_fov_degrees = camera.vertical_fov_degrees;
    }

    /// Scale camera distance by `factor`, then clamp to the supported range.
    pub fn zoom(&mut self, factor: f32) {
        if factor.is_finite() && factor > 0.0 {
            let next = self.distance * factor;
            self.distance = if next.is_finite() { next } else { MAX_DISTANCE };
        }
        *self = self.clamped();
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
    ///
    /// This is interpreted as the direction light rays travel. Lambert shading
    /// uses `normal dot -light_direction`.
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

/// Rendered row-major image buffer with `(0, 0)` at the top-left.
#[derive(Debug, Clone, PartialEq)]
pub struct RenderedImage {
    /// Width in pixels.
    pub width: u32,
    /// Height in pixels.
    pub height: u32,
    /// RGBA8 pixel data.
    pub rgba8: Vec<u8>,
}

impl RenderedImage {
    /// Return immutable access to the raw RGBA8 bytes.
    #[must_use]
    pub fn pixels(&self) -> &[u8] {
        &self.rgba8
    }

    /// Return the RGBA pixel at `x, y`, or `None` when out of bounds.
    #[must_use]
    pub fn pixel(&self, x: u32, y: u32) -> Option<[u8; 4]> {
        if x >= self.width || y >= self.height {
            return None;
        }
        let pixel_index = u64::from(y)
            .checked_mul(u64::from(self.width))?
            .checked_add(u64::from(x))?;
        let byte_index = pixel_index.checked_mul(4)?;
        let byte_index = usize::try_from(byte_index).ok()?;
        let end = byte_index.checked_add(4)?;
        let pixel = self.rgba8.get(byte_index..end)?;
        Some([pixel[0], pixel[1], pixel[2], pixel[3]])
    }
}

/// Render errors.
#[derive(Debug, Clone, Error, PartialEq, Eq)]
pub enum RenderError {
    /// Render settings are invalid.
    #[error("invalid render settings: {0}")]
    InvalidSettings(&'static str),
    /// Camera data is invalid.
    #[error("invalid camera: {0}")]
    InvalidCamera(&'static str),
    /// Mesh data is invalid.
    #[error("invalid mesh: {0}")]
    InvalidMesh(&'static str),
    /// The requested operation belongs to a later wave.
    #[error("not implemented: {0}")]
    NotImplemented(&'static str),
}

#[derive(Debug, Copy, Clone)]
struct RenderVertex {
    view: Vec3,
    normal: Vec3,
}

#[derive(Debug, Copy, Clone)]
struct ProjectedVertex {
    screen: Vec2,
    ndc: Vec2,
    inv_depth: f32,
    normal_over_depth: Vec3,
}

#[derive(Debug, Copy, Clone)]
struct ProjectionParams {
    focal_x: f32,
    focal_y: f32,
    width: u32,
    height: u32,
}

#[derive(Debug)]
struct RenderTarget {
    width: usize,
    rgba8: Vec<u8>,
    depth: Vec<f32>,
}

/// Fit an orbit camera to bounds.
#[must_use]
pub fn fit_camera_to_bounds(bounds: Aabb) -> OrbitCamera {
    let mut camera = OrbitCamera::default();
    if bounds.is_empty() || !is_finite_vec3(bounds.min) || !is_finite_vec3(bounds.max) {
        return camera;
    }

    let extent = bounds.extent();
    let center = bounds.center();
    if !is_finite_vec3(extent) || !is_finite_vec3(center) {
        return camera;
    }

    let radius = (extent.length() * 0.5).max(MIN_DISTANCE);
    let padded_radius = radius * 1.35;
    let half_fov = (camera.vertical_fov_degrees * 0.5).to_radians();
    camera.target = center;
    camera.distance = (padded_radius / half_fov.sin()).clamp(MIN_DISTANCE, MAX_DISTANCE);
    camera.clamped()
}

/// Render a mesh to an RGBA8 image.
pub fn render_mesh(
    mesh: &TriangleMesh,
    camera: &OrbitCamera,
    settings: &RenderSettings,
) -> Result<RenderedImage, RenderError> {
    let pixel_count = validate_settings(settings)?;
    let camera = validate_camera(camera)?;
    validate_mesh(mesh)?;

    let mut target = RenderTarget::new(settings, pixel_count)?;
    if mesh.indices.is_empty() {
        return Ok(target.into_image(settings));
    }

    let light_direction = settings.light_direction.normalize();
    let view = camera.view_matrix();
    let projection = ProjectionParams::new(settings, &camera);

    for triangle in mesh.indices.chunks_exact(3) {
        let i0 = index_to_usize(triangle[0])?;
        let i1 = index_to_usize(triangle[1])?;
        let i2 = index_to_usize(triangle[2])?;
        let p0 = vec3_from_array(mesh.positions[i0]);
        let p1 = vec3_from_array(mesh.positions[i1]);
        let p2 = vec3_from_array(mesh.positions[i2]);
        let Some(face_normal) = face_normal(p0, p1, p2) else {
            continue;
        };

        let vertices = [
            RenderVertex {
                view: view.transform_point3(p0),
                normal: vertex_normal(mesh, i0).unwrap_or(face_normal),
            },
            RenderVertex {
                view: view.transform_point3(p1),
                normal: vertex_normal(mesh, i1).unwrap_or(face_normal),
            },
            RenderVertex {
                view: view.transform_point3(p2),
                normal: vertex_normal(mesh, i2).unwrap_or(face_normal),
            },
        ];

        let clipped = clip_triangle_to_near(vertices);
        if clipped.len() < 3 {
            continue;
        }

        for fan_index in 1..(clipped.len() - 1) {
            rasterize_triangle(
                [clipped[0], clipped[fan_index], clipped[fan_index + 1]],
                &projection,
                settings,
                light_direction,
                &mut target,
            );
        }
    }

    Ok(target.into_image(settings))
}

impl ProjectionParams {
    fn new(settings: &RenderSettings, camera: &OrbitCamera) -> Self {
        let aspect = settings.width as f32 / settings.height as f32;
        let focal_y = 1.0 / (camera.vertical_fov_degrees.to_radians() * 0.5).tan();
        Self {
            focal_x: focal_y / aspect,
            focal_y,
            width: settings.width,
            height: settings.height,
        }
    }
}

impl RenderTarget {
    fn new(settings: &RenderSettings, pixel_count: usize) -> Result<Self, RenderError> {
        let width = usize::try_from(settings.width)
            .map_err(|_| RenderError::InvalidSettings("width is too large"))?;
        let byte_len = pixel_count
            .checked_mul(4)
            .ok_or(RenderError::InvalidSettings("image byte length overflows"))?;
        let mut rgba8 = Vec::with_capacity(byte_len);
        for _ in 0..pixel_count {
            rgba8.extend_from_slice(&settings.background);
        }

        Ok(Self {
            width,
            rgba8,
            depth: vec![f32::INFINITY; pixel_count],
        })
    }

    fn into_image(self, settings: &RenderSettings) -> RenderedImage {
        RenderedImage {
            width: settings.width,
            height: settings.height,
            rgba8: self.rgba8,
        }
    }
}

fn validate_settings(settings: &RenderSettings) -> Result<usize, RenderError> {
    if settings.width == 0 || settings.height == 0 {
        return Err(RenderError::InvalidSettings(
            "width and height must be positive",
        ));
    }
    let pixel_count = u64::from(settings.width) * u64::from(settings.height);
    if pixel_count > MAX_PIXELS {
        return Err(RenderError::InvalidSettings(
            "image dimensions are too large",
        ));
    }
    let pixel_count = usize::try_from(pixel_count)
        .map_err(|_| RenderError::InvalidSettings("image dimensions are too large"))?;
    if !settings.ambient.is_finite() {
        return Err(RenderError::InvalidSettings("ambient must be finite"));
    }
    if !is_finite_vec3(settings.light_direction)
        || settings.light_direction.length_squared() <= MIN_NORMAL_LENGTH_SQUARED
    {
        return Err(RenderError::InvalidSettings(
            "light direction must be finite and non-zero",
        ));
    }
    Ok(pixel_count)
}

fn validate_camera(camera: &OrbitCamera) -> Result<OrbitCamera, RenderError> {
    if !is_finite_vec3(camera.target) {
        return Err(RenderError::InvalidCamera("target must be finite"));
    }
    if !camera.yaw_degrees.is_finite()
        || !camera.pitch_degrees.is_finite()
        || !camera.distance.is_finite()
        || !camera.vertical_fov_degrees.is_finite()
    {
        return Err(RenderError::InvalidCamera("orbit values must be finite"));
    }
    Ok(camera.clamped())
}

fn validate_mesh(mesh: &TriangleMesh) -> Result<(), RenderError> {
    if !mesh.indices.len().is_multiple_of(3) {
        return Err(RenderError::InvalidMesh(
            "triangle index count must be divisible by three",
        ));
    }
    for position in &mesh.positions {
        if !is_finite_vec3(vec3_from_array(*position)) {
            return Err(RenderError::InvalidMesh("positions must be finite"));
        }
    }
    for index in &mesh.indices {
        let index = index_to_usize(*index)?;
        if index >= mesh.positions.len() {
            return Err(RenderError::InvalidMesh("triangle index out of bounds"));
        }
    }
    Ok(())
}

fn rasterize_triangle(
    vertices: [RenderVertex; 3],
    projection: &ProjectionParams,
    settings: &RenderSettings,
    light_direction: Vec3,
    target: &mut RenderTarget,
) {
    if should_cull_backface(vertices) {
        return;
    }

    let Some(projected) = project_triangle(vertices, projection) else {
        return;
    };
    if is_outside_viewport(projected) {
        return;
    }

    let area = edge(
        projected[0].screen,
        projected[1].screen,
        projected[2].screen,
    );
    if !area.is_finite() || area.abs() <= MIN_AREA {
        return;
    }

    let min_x = projected
        .iter()
        .map(|vertex| vertex.screen.x)
        .fold(f32::INFINITY, f32::min)
        .floor()
        .max(0.0);
    let max_x = projected
        .iter()
        .map(|vertex| vertex.screen.x)
        .fold(f32::NEG_INFINITY, f32::max)
        .ceil()
        .min((projection.width - 1) as f32);
    let min_y = projected
        .iter()
        .map(|vertex| vertex.screen.y)
        .fold(f32::INFINITY, f32::min)
        .floor()
        .max(0.0);
    let max_y = projected
        .iter()
        .map(|vertex| vertex.screen.y)
        .fold(f32::NEG_INFINITY, f32::max)
        .ceil()
        .min((projection.height - 1) as f32);

    if min_x > max_x || min_y > max_y {
        return;
    }

    let min_x = min_x as usize;
    let max_x = max_x as usize;
    let min_y = min_y as usize;
    let max_y = max_y as usize;
    let edge_lengths = [
        (projected[2].screen - projected[1].screen)
            .length()
            .max(1.0),
        (projected[0].screen - projected[2].screen)
            .length()
            .max(1.0),
        (projected[1].screen - projected[0].screen)
            .length()
            .max(1.0),
    ];

    for y in min_y..=max_y {
        for x in min_x..=max_x {
            let sample = Vec2::new(x as f32, y as f32);
            let edges = [
                edge(projected[1].screen, projected[2].screen, sample),
                edge(projected[2].screen, projected[0].screen, sample),
                edge(projected[0].screen, projected[1].screen, sample),
            ];
            if !is_inside(edges, area) {
                continue;
            }

            let barycentric = [edges[0] / area, edges[1] / area, edges[2] / area];
            let depth_denominator = barycentric[0] * projected[0].inv_depth
                + barycentric[1] * projected[1].inv_depth
                + barycentric[2] * projected[2].inv_depth;
            if !depth_denominator.is_finite() || depth_denominator <= 0.0 {
                continue;
            }

            let depth = 1.0 / depth_denominator;
            let pixel_index = y * target.width + x;
            if depth >= target.depth[pixel_index] {
                continue;
            }

            let normal = (projected[0].normal_over_depth * barycentric[0]
                + projected[1].normal_over_depth * barycentric[1]
                + projected[2].normal_over_depth * barycentric[2])
                / depth_denominator;
            let wireframe =
                settings.wireframe && is_wire_pixel(edges, edge_lengths, area.is_sign_negative());
            let color = shade(normal, settings.ambient, light_direction, wireframe);
            let byte_index = pixel_index * 4;
            target.rgba8[byte_index..byte_index + 4].copy_from_slice(&color);
            target.depth[pixel_index] = depth;
        }
    }
}

fn should_cull_backface(vertices: [RenderVertex; 3]) -> bool {
    let normal = (vertices[1].view - vertices[0].view).cross(vertices[2].view - vertices[0].view);
    !is_finite_vec3(normal) || normal.z <= MIN_AREA
}

fn project_triangle(
    vertices: [RenderVertex; 3],
    projection: &ProjectionParams,
) -> Option<[ProjectedVertex; 3]> {
    Some([
        project_vertex(vertices[0], projection)?,
        project_vertex(vertices[1], projection)?,
        project_vertex(vertices[2], projection)?,
    ])
}

fn project_vertex(vertex: RenderVertex, projection: &ProjectionParams) -> Option<ProjectedVertex> {
    let depth = -vertex.view.z;
    if !depth.is_finite() || depth <= NEAR_PLANE {
        return None;
    }
    let ndc = Vec2::new(
        vertex.view.x * projection.focal_x / depth,
        vertex.view.y * projection.focal_y / depth,
    );
    if !is_finite_vec2(ndc) {
        return None;
    }
    let screen = Vec2::new(
        (ndc.x * 0.5 + 0.5) * projection.width as f32 - 0.5,
        (0.5 - ndc.y * 0.5) * projection.height as f32 - 0.5,
    );
    if !is_finite_vec2(screen) {
        return None;
    }

    let inv_depth = 1.0 / depth;
    let normal = normalize_or(vertex.normal, Vec3::Y);
    Some(ProjectedVertex {
        screen,
        ndc,
        inv_depth,
        normal_over_depth: normal * inv_depth,
    })
}

fn is_outside_viewport(vertices: [ProjectedVertex; 3]) -> bool {
    vertices
        .iter()
        .all(|vertex| vertex.ndc.x < -1.0 || vertex.ndc.x > 1.0)
        || vertices
            .iter()
            .all(|vertex| vertex.ndc.y < -1.0 || vertex.ndc.y > 1.0)
}

fn is_inside(edges: [f32; 3], area: f32) -> bool {
    if area.is_sign_positive() {
        edges.iter().all(|edge| *edge >= -EDGE_EPSILON)
    } else {
        edges.iter().all(|edge| *edge <= EDGE_EPSILON)
    }
}

fn is_wire_pixel(edges: [f32; 3], edge_lengths: [f32; 3], negative_area: bool) -> bool {
    let sign = if negative_area { -1.0 } else { 1.0 };
    edges
        .iter()
        .zip(edge_lengths)
        .map(|(edge_value, edge_length)| (edge_value * sign).abs() / edge_length)
        .fold(f32::INFINITY, f32::min)
        <= WIRE_DISTANCE_PIXELS
}

fn shade(normal: Vec3, ambient: f32, light_direction: Vec3, wireframe: bool) -> [u8; 4] {
    let normal = normalize_or(normal, Vec3::Y);
    let ambient = ambient.clamp(0.0, 1.0);
    let lambert = normal.dot(-light_direction).max(0.0);
    let intensity = (ambient + (1.0 - ambient) * lambert).clamp(0.0, 1.0);
    let mut color = MATERIAL_COLOR * intensity;
    if wireframe {
        color = color * 0.45 + WIREFRAME_MIX * 0.55;
    }
    [
        color_channel(color.x),
        color_channel(color.y),
        color_channel(color.z),
        255,
    ]
}

fn color_channel(value: f32) -> u8 {
    if value.is_finite() {
        value.clamp(0.0, 255.0).round() as u8
    } else {
        0
    }
}

fn clip_triangle_to_near(vertices: [RenderVertex; 3]) -> Vec<RenderVertex> {
    let mut output = Vec::with_capacity(4);
    let mut previous = vertices[2];
    let mut previous_inside = is_inside_near(previous);

    for current in vertices {
        let current_inside = is_inside_near(current);
        if current_inside != previous_inside {
            output.push(intersect_near(previous, current));
        }
        if current_inside {
            output.push(current);
        }
        previous = current;
        previous_inside = current_inside;
    }

    output
}

fn is_inside_near(vertex: RenderVertex) -> bool {
    vertex.view.z <= -NEAR_PLANE
}

fn intersect_near(a: RenderVertex, b: RenderVertex) -> RenderVertex {
    let denominator = b.view.z - a.view.z;
    let t = if denominator.abs() > f32::EPSILON {
        ((-NEAR_PLANE - a.view.z) / denominator).clamp(0.0, 1.0)
    } else {
        0.0
    };
    let normal = normalize_or(a.normal.lerp(b.normal, t), a.normal);
    RenderVertex {
        view: a.view.lerp(b.view, t),
        normal,
    }
}

fn face_normal(p0: Vec3, p1: Vec3, p2: Vec3) -> Option<Vec3> {
    let normal = (p1 - p0).cross(p2 - p0);
    if is_finite_vec3(normal) && normal.length_squared() > MIN_NORMAL_LENGTH_SQUARED {
        Some(normal.normalize())
    } else {
        None
    }
}

fn vertex_normal(mesh: &TriangleMesh, index: usize) -> Option<Vec3> {
    if mesh.normals.len() != mesh.positions.len() {
        return None;
    }
    let normal = vec3_from_array(*mesh.normals.get(index)?);
    if is_finite_vec3(normal) && normal.length_squared() > MIN_NORMAL_LENGTH_SQUARED {
        Some(normal.normalize())
    } else {
        None
    }
}

fn edge(a: Vec2, b: Vec2, c: Vec2) -> f32 {
    (c.x - a.x) * (b.y - a.y) - (c.y - a.y) * (b.x - a.x)
}

fn normalize_degrees(degrees: f32) -> f32 {
    let normalized = degrees.rem_euclid(360.0);
    if normalized == 360.0 { 0.0 } else { normalized }
}

fn normalize_or(vector: Vec3, fallback: Vec3) -> Vec3 {
    if is_finite_vec3(vector) && vector.length_squared() > MIN_NORMAL_LENGTH_SQUARED {
        vector.normalize()
    } else {
        fallback
    }
}

fn is_finite_vec2(value: Vec2) -> bool {
    value.x.is_finite() && value.y.is_finite()
}

fn is_finite_vec3(value: Vec3) -> bool {
    value.x.is_finite() && value.y.is_finite() && value.z.is_finite()
}

fn vec3_from_array(value: [f32; 3]) -> Vec3 {
    Vec3::new(value[0], value[1], value[2])
}

fn index_to_usize(index: u32) -> Result<usize, RenderError> {
    usize::try_from(index).map_err(|_| RenderError::InvalidMesh("triangle index is too large"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use glam::Vec4;

    fn settings(width: u32, height: u32) -> RenderSettings {
        RenderSettings {
            width,
            height,
            background: [3, 5, 7, 255],
            ambient: 0.25,
            light_direction: Vec3::new(0.0, -1.0, -1.0).normalize(),
            wireframe: false,
        }
    }

    fn camera() -> OrbitCamera {
        OrbitCamera {
            target: Vec3::ZERO,
            yaw_degrees: 0.0,
            pitch_degrees: 0.0,
            distance: 3.0,
            vertical_fov_degrees: 45.0,
        }
    }

    fn empty_mesh() -> TriangleMesh {
        TriangleMesh {
            positions: Vec::new(),
            normals: Vec::new(),
            indices: Vec::new(),
            bounds: Aabb::empty(),
        }
    }

    fn triangle_mesh() -> TriangleMesh {
        TriangleMesh {
            positions: vec![[-0.8, -0.8, 0.0], [0.8, -0.8, 0.0], [0.0, 0.8, 0.0]],
            normals: vec![[0.0, 0.0, 1.0]; 3],
            indices: vec![0, 1, 2],
            bounds: Aabb {
                min: Vec3::new(-0.8, -0.8, 0.0),
                max: Vec3::new(0.8, 0.8, 0.0),
            },
        }
    }

    #[test]
    fn fit_camera_sees_known_cube() {
        let bounds = Aabb {
            min: Vec3::splat(-1.0),
            max: Vec3::splat(1.0),
        };
        let camera = fit_camera_to_bounds(bounds);
        let view_projection = camera.view_projection_matrix(1.0);

        for x in [-1.0, 1.0] {
            for y in [-1.0, 1.0] {
                for z in [-1.0, 1.0] {
                    let clip = view_projection * Vec4::new(x, y, z, 1.0);
                    let ndc = clip.truncate() / clip.w;
                    assert!(ndc.x.abs() <= 1.0, "x outside view: {ndc:?}");
                    assert!(ndc.y.abs() <= 1.0, "y outside view: {ndc:?}");
                    assert!(ndc.z >= -1.0 && ndc.z <= 1.0, "z outside view: {ndc:?}");
                }
            }
        }
    }

    #[test]
    fn rendering_triangle_changes_foreground_pixels() {
        let image = render_mesh(&triangle_mesh(), &camera(), &settings(64, 64))
            .expect("triangle render should succeed");

        assert!(
            image
                .rgba8
                .chunks_exact(4)
                .any(|pixel| pixel != settings(64, 64).background)
        );
    }

    #[test]
    fn nearer_triangle_wins_depth_test() {
        let mesh = TriangleMesh {
            positions: vec![
                [-0.8, -0.8, -0.2],
                [0.8, -0.8, -0.2],
                [0.0, 0.8, -0.2],
                [-0.8, -0.8, 0.2],
                [0.8, -0.8, 0.2],
                [0.0, 0.8, 0.2],
            ],
            normals: vec![
                [0.0, 0.0, 1.0],
                [0.0, 0.0, 1.0],
                [0.0, 0.0, 1.0],
                [0.0, 1.0, 0.0],
                [0.0, 1.0, 0.0],
                [0.0, 1.0, 0.0],
            ],
            indices: vec![0, 1, 2, 3, 4, 5],
            bounds: Aabb {
                min: Vec3::new(-0.8, -0.8, -0.2),
                max: Vec3::new(0.8, 0.8, 0.2),
            },
        };
        let mut render_settings = settings(64, 64);
        render_settings.ambient = 0.0;
        render_settings.light_direction = Vec3::NEG_Y;

        let image = render_mesh(&mesh, &camera(), &render_settings)
            .expect("overlapping triangles should render");
        let center = image.pixel(32, 32).expect("center pixel exists");

        assert!(
            center[0] > 120,
            "near bright triangle should win: {center:?}"
        );
    }

    #[test]
    fn backface_culling_rejects_reversed_triangle() {
        let mesh = TriangleMesh {
            positions: vec![[-0.8, -0.8, 0.0], [0.0, 0.8, 0.0], [0.8, -0.8, 0.0]],
            normals: vec![[0.0, 0.0, -1.0]; 3],
            indices: vec![0, 1, 2],
            bounds: Aabb {
                min: Vec3::new(-0.8, -0.8, 0.0),
                max: Vec3::new(0.8, 0.8, 0.0),
            },
        };
        let render_settings = settings(64, 64);
        let image = render_mesh(&mesh, &camera(), &render_settings)
            .expect("backface render should succeed");

        assert!(
            image
                .rgba8
                .chunks_exact(4)
                .all(|pixel| pixel == render_settings.background)
        );
    }

    #[test]
    fn image_dimensions_and_byte_length_are_correct() {
        let image = render_mesh(&empty_mesh(), &camera(), &settings(7, 5))
            .expect("empty render should succeed");

        assert_eq!(image.width, 7);
        assert_eq!(image.height, 5);
        assert_eq!(image.rgba8.len(), 7 * 5 * 4);
        assert_eq!(image.pixel(6, 4), Some([3, 5, 7, 255]));
        assert_eq!(image.pixel(7, 4), None);
    }

    #[test]
    fn empty_mesh_produces_only_background() {
        let render_settings = settings(8, 8);
        let image = render_mesh(&empty_mesh(), &camera(), &render_settings)
            .expect("empty mesh should render");

        assert!(
            image
                .rgba8
                .chunks_exact(4)
                .all(|pixel| pixel == render_settings.background)
        );
    }

    #[test]
    fn orbit_and_zoom_remain_finite_and_clamped() {
        let mut camera = OrbitCamera::default();
        camera.orbit(725.0, 500.0);
        camera.zoom(0.000_001);
        camera.pan(0.25, -0.5);

        assert!(is_finite_vec3(camera.target));
        assert!(camera.yaw_degrees >= 0.0 && camera.yaw_degrees < 360.0);
        assert!(camera.pitch_degrees <= MAX_PITCH_DEGREES);
        assert!(camera.distance >= MIN_DISTANCE);

        camera.zoom(1.0e30);
        assert!(camera.distance <= MAX_DISTANCE);
        assert!(camera.distance.is_finite());
    }

    #[test]
    fn identical_inputs_produce_identical_bytes() {
        let mesh = triangle_mesh();
        let camera = camera();
        let settings = settings(64, 64);
        let first = render_mesh(&mesh, &camera, &settings).expect("first render should succeed");
        let second = render_mesh(&mesh, &camera, &settings).expect("second render should succeed");

        assert_eq!(first.rgba8, second.rgba8);
    }

    #[test]
    fn malformed_mesh_returns_error_instead_of_panicking() {
        let mesh = TriangleMesh {
            positions: vec![[0.0, 0.0, 0.0]],
            normals: Vec::new(),
            indices: vec![0, 1, 2],
            bounds: Aabb {
                min: Vec3::ZERO,
                max: Vec3::ZERO,
            },
        };

        assert!(matches!(
            render_mesh(&mesh, &camera(), &settings(16, 16)),
            Err(RenderError::InvalidMesh("triangle index out of bounds"))
        ));
    }
}
