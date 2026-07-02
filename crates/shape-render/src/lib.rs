#![forbid(unsafe_code)]

//! Deterministic CPU preview renderer.
//!
//! The renderer writes row-major RGBA8 pixels with `(0, 0)` at the top-left
//! of the image. Triangle depth and normals are perspective-correctly
//! interpolated from screen-space barycentric coordinates before shading.
//! `RenderWorkspace` and `RenderCache` allow callers that render repeatedly
//! to reuse allocation-heavy buffers while keeping the default `render_mesh`
//! convenience function.

pub mod foundry;
pub mod surface_preview;

use glam::{Mat4, Vec2, Vec3};
use serde::{Deserialize, Serialize};
use shape_mesh::TriangleMesh;
use thiserror::Error;

pub use shape_core::Aabb;

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
const WIRE_DISTANCE_PIXELS: f32 = 1.65;
const FRAME_PADDING: f32 = 1.18;
const EDGE_OUTLINE_NORMAL_DOT_THRESHOLD: f32 = 0.88;
const EDGE_OUTLINE_DEPTH_THRESHOLD: f32 = 0.028;
const EDGE_OUTLINE_MIX_AMOUNT: f32 = 0.72;
const CACHE_HASH_OFFSET: u64 = 14_695_981_039_346_656_037;
const CACHE_HASH_PRIME: u64 = 1_099_511_628_211;
const MATERIAL_COLOR: Vec3 = Vec3::new(196.0, 202.0, 193.0);
const WIREFRAME_MIX: Vec3 = Vec3::new(70.0, 74.0, 76.0);
const EDGE_OUTLINE_MIX: Vec3 = Vec3::new(58.0, 64.0, 64.0);

/// Number of fixed views used by mesh visual descriptors.
pub const VISUAL_DESCRIPTOR_CAMERA_COUNT: usize = 4;
/// Width and height of descriptor silhouette masks.
pub const VISUAL_DESCRIPTOR_MASK_SIZE: u32 = 64;
/// Number of u64 words in one descriptor silhouette mask.
pub const VISUAL_DESCRIPTOR_MASK_WORDS: usize =
    (VISUAL_DESCRIPTOR_MASK_SIZE as usize * VISUAL_DESCRIPTOR_MASK_SIZE as usize) / 64;
/// Number of bins in each fixed-view depth histogram.
pub const VISUAL_DESCRIPTOR_DEPTH_BINS: usize = 8;

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
    /// Draw a display-only crease/silhouette edge aid over the clay preview.
    #[serde(default)]
    pub edge_outline: bool,
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
            edge_outline: false,
        }
    }
}

/// Return pure-clay viewport settings tuned for readable broad faces and edges.
#[must_use]
pub fn clay_readability_render_settings(width: u32, height: u32) -> RenderSettings {
    RenderSettings {
        width,
        height,
        ambient: 0.24,
        light_direction: Vec3::new(-0.55, -0.78, -0.34).normalize(),
        wireframe: false,
        edge_outline: true,
        ..RenderSettings::default()
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

/// Mesh visual descriptors derived from low-resolution fixed-camera renders.
#[derive(Debug, Clone, PartialEq)]
pub struct MeshVisualDescriptor {
    /// Foreground occupancy per fixed view.
    pub silhouette_occupancy: [f32; VISUAL_DESCRIPTOR_CAMERA_COUNT],
    /// Normalized boundary length per fixed view.
    pub silhouette_perimeter: [f32; VISUAL_DESCRIPTOR_CAMERA_COUNT],
    /// One packed 64x64 binary silhouette mask per fixed view.
    pub silhouette_masks: [[u64; VISUAL_DESCRIPTOR_MASK_WORDS]; VISUAL_DESCRIPTOR_CAMERA_COUNT],
    /// Fixed-view histograms derived from visible z-buffer depth samples.
    pub depth_histogram: [[f32; VISUAL_DESCRIPTOR_DEPTH_BINS]; VISUAL_DESCRIPTOR_CAMERA_COUNT],
}

/// Stable key for the effective render inputs used by `RenderCache`.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub struct RenderCacheKey {
    /// Hash of render-relevant mesh positions, normals, and indices.
    pub mesh_hash: u64,
    /// Hash of the clamped camera values used for rendering.
    pub camera_hash: u64,
    /// Hash of render settings after applying render-equivalent normalization.
    pub settings_hash: u64,
}

impl RenderCacheKey {
    /// Build a cache key for render inputs.
    ///
    /// The key uses the same validation and camera clamping as rendering. It is
    /// intended for in-process cache reuse, not as a persisted file format.
    pub fn new(
        mesh: &TriangleMesh,
        camera: &OrbitCamera,
        settings: &RenderSettings,
    ) -> Result<Self, RenderError> {
        validate_settings(settings)?;
        let camera = validate_camera(camera)?;
        validate_mesh(mesh)?;

        Ok(Self {
            mesh_hash: hash_mesh(mesh),
            camera_hash: hash_camera(&camera),
            settings_hash: hash_settings(settings),
        })
    }
}

/// Reusable scratch buffers for repeated CPU renders.
#[derive(Debug, Default)]
pub struct RenderWorkspace {
    depth: Vec<f32>,
    normals: Vec<Vec3>,
}

impl RenderWorkspace {
    /// Render a mesh, reusing workspace scratch buffers where possible.
    pub fn render_mesh(
        &mut self,
        mesh: &TriangleMesh,
        camera: &OrbitCamera,
        settings: &RenderSettings,
    ) -> Result<RenderedImage, RenderError> {
        let mut image = RenderedImage {
            width: 0,
            height: 0,
            rgba8: Vec::new(),
        };
        self.render_mesh_into(&mut image, mesh, camera, settings)?;
        Ok(image)
    }

    /// Render into an existing image allocation.
    ///
    /// The image dimensions and byte length are rewritten to match `settings`.
    /// Existing capacity is retained when it is large enough.
    pub fn render_mesh_into(
        &mut self,
        image: &mut RenderedImage,
        mesh: &TriangleMesh,
        camera: &OrbitCamera,
        settings: &RenderSettings,
    ) -> Result<(), RenderError> {
        let pixel_count = validate_settings(settings)?;
        let camera = validate_camera(camera)?;
        validate_mesh(mesh)?;

        let mut target = RenderTarget::prepare(settings, pixel_count, image, self)?;
        if mesh.indices.is_empty() {
            return Ok(());
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
                    normal: render_normal(mesh, i0, face_normal),
                },
                RenderVertex {
                    view: view.transform_point3(p1),
                    normal: render_normal(mesh, i1, face_normal),
                },
                RenderVertex {
                    view: view.transform_point3(p2),
                    normal: render_normal(mesh, i2, face_normal),
                },
            ];

            if is_fully_outside_depth(vertices) {
                continue;
            }

            let clipped = clip_triangle_to_near(vertices);
            if clipped.len() < 3 || is_outside_frustum(clipped.as_slice(), &projection) {
                continue;
            }

            for fan_index in 1..(clipped.len() - 1) {
                rasterize_triangle(
                    [
                        clipped.vertex(0),
                        clipped.vertex(fan_index),
                        clipped.vertex(fan_index + 1),
                    ],
                    &projection,
                    settings,
                    light_direction,
                    &mut target,
                );
            }
        }

        if settings.edge_outline {
            apply_edge_outline(&mut target);
        }

        Ok(())
    }
}

/// Result of a cached render request.
#[derive(Debug)]
pub struct CachedRender<'a> {
    /// Cache key used for this request.
    pub key: RenderCacheKey,
    /// Rendered image, either reused from the previous request or freshly rendered.
    pub image: &'a RenderedImage,
    /// True when the image was reused without rerasterizing.
    pub reused: bool,
}

/// Single-entry render cache for camera/mesh/settings rerenders.
#[derive(Debug, Default)]
pub struct RenderCache {
    workspace: RenderWorkspace,
    last: Option<CachedImage>,
}

impl RenderCache {
    /// Render the mesh unless the previous image has the same cache key.
    pub fn render_mesh(
        &mut self,
        mesh: &TriangleMesh,
        camera: &OrbitCamera,
        settings: &RenderSettings,
    ) -> Result<CachedRender<'_>, RenderError> {
        let key = RenderCacheKey::new(mesh, camera, settings)?;
        if self.last.as_ref().is_some_and(|cached| cached.key == key) {
            let image = &self
                .last
                .as_ref()
                .expect("cache hit should retain an image")
                .image;
            return Ok(CachedRender {
                key,
                image,
                reused: true,
            });
        }

        let mut image = self.last.take().map_or_else(
            || RenderedImage {
                width: 0,
                height: 0,
                rgba8: Vec::new(),
            },
            |cached| cached.image,
        );
        self.workspace
            .render_mesh_into(&mut image, mesh, camera, settings)?;
        self.last = Some(CachedImage { key, image });
        let image = &self
            .last
            .as_ref()
            .expect("cache miss should store the rendered image")
            .image;
        Ok(CachedRender {
            key,
            image,
            reused: false,
        })
    }

    /// Return the key for the cached image, if any.
    #[must_use]
    pub fn last_key(&self) -> Option<RenderCacheKey> {
        self.last.as_ref().map(|cached| cached.key)
    }

    /// Clear the cached image while retaining reusable workspace allocations.
    pub fn clear(&mut self) {
        self.last = None;
    }
}

#[derive(Debug)]
struct CachedImage {
    key: RenderCacheKey,
    image: RenderedImage,
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

#[derive(Debug, Copy, Clone)]
struct ClippedTriangle {
    vertices: [RenderVertex; 4],
    len: usize,
}

impl ClippedTriangle {
    fn new(seed: RenderVertex) -> Self {
        Self {
            vertices: [seed; 4],
            len: 0,
        }
    }

    fn push(&mut self, vertex: RenderVertex) {
        if self.len < self.vertices.len() {
            self.vertices[self.len] = vertex;
            self.len += 1;
        }
    }

    fn len(&self) -> usize {
        self.len
    }

    fn vertex(&self, index: usize) -> RenderVertex {
        self.vertices[index]
    }

    fn as_slice(&self) -> &[RenderVertex] {
        &self.vertices[..self.len]
    }
}

#[derive(Debug, Copy, Clone)]
struct PixelBounds {
    min_x: usize,
    max_x: usize,
    min_y: usize,
    max_y: usize,
}

#[derive(Debug)]
struct RenderTarget<'a> {
    width: usize,
    height: usize,
    rgba8: &'a mut [u8],
    depth: &'a mut [f32],
    normals: &'a mut [Vec3],
}

/// Fit an orbit camera to bounds.
#[must_use]
pub fn fit_camera_to_bounds(bounds: Aabb) -> OrbitCamera {
    fit_camera_to_bounds_with_aspect(bounds, 1.0)
}

/// Fit an orbit camera to bounds for a known viewport aspect ratio.
#[must_use]
pub fn fit_camera_to_bounds_with_aspect(bounds: Aabb, aspect_ratio: f32) -> OrbitCamera {
    fit_camera_to_bounds_with_target(bounds, bounds.center(), aspect_ratio)
}

/// Fit an orbit camera to bounds while keeping a fixed world-space target.
#[must_use]
pub fn fit_camera_to_bounds_with_target(
    bounds: Aabb,
    target: Vec3,
    aspect_ratio: f32,
) -> OrbitCamera {
    let mut camera = OrbitCamera::default();
    if bounds.is_empty() || !is_finite_vec3(bounds.min) || !is_finite_vec3(bounds.max) {
        return camera;
    }

    if !is_finite_vec3(target) {
        return camera;
    }

    camera.target = target;
    camera.distance = fit_distance_for_bounds(bounds, &camera, aspect_ratio);
    camera.clamped()
}

/// Fit an orbit camera with explicit yaw and pitch to bounds.
#[must_use]
pub fn fit_camera_to_bounds_from_angles(
    bounds: Aabb,
    yaw_degrees: f32,
    pitch_degrees: f32,
    aspect_ratio: f32,
) -> OrbitCamera {
    let mut camera = fit_camera_to_bounds_with_target(bounds, bounds.center(), aspect_ratio);
    camera.yaw_degrees = yaw_degrees;
    camera.pitch_degrees = pitch_degrees;
    camera = camera.clamped();
    camera.distance = fit_distance_for_bounds(bounds, &camera, aspect_ratio);
    camera.clamped()
}

/// Fit an explicit-angle orbit camera while keeping world origin at the viewport center.
#[must_use]
pub fn fit_camera_to_bounds_from_angles_around_origin(
    bounds: Aabb,
    yaw_degrees: f32,
    pitch_degrees: f32,
    aspect_ratio: f32,
) -> OrbitCamera {
    let mut camera = fit_camera_to_bounds_with_target(bounds, Vec3::ZERO, aspect_ratio);
    camera.yaw_degrees = yaw_degrees;
    camera.pitch_degrees = pitch_degrees;
    camera = camera.clamped();
    camera.distance = fit_distance_for_bounds(bounds, &camera, aspect_ratio);
    camera.clamped()
}

/// Derive fixed-camera visual descriptors from the rendered mesh silhouette.
pub fn visual_descriptor_for_mesh(
    mesh: &TriangleMesh,
) -> Result<MeshVisualDescriptor, RenderError> {
    let mut descriptor = MeshVisualDescriptor {
        silhouette_occupancy: [0.0; VISUAL_DESCRIPTOR_CAMERA_COUNT],
        silhouette_perimeter: [0.0; VISUAL_DESCRIPTOR_CAMERA_COUNT],
        silhouette_masks: [[0; VISUAL_DESCRIPTOR_MASK_WORDS]; VISUAL_DESCRIPTOR_CAMERA_COUNT],
        depth_histogram: [[0.0; VISUAL_DESCRIPTOR_DEPTH_BINS]; VISUAL_DESCRIPTOR_CAMERA_COUNT],
    };
    if mesh.indices.is_empty() || mesh.bounds.is_empty() {
        return Ok(descriptor);
    }

    let settings = RenderSettings {
        width: VISUAL_DESCRIPTOR_MASK_SIZE,
        height: VISUAL_DESCRIPTOR_MASK_SIZE,
        background: [0, 0, 0, 0],
        ambient: 1.0,
        wireframe: false,
        ..RenderSettings::default()
    };
    let mut workspace = RenderWorkspace::default();
    let mut image = RenderedImage {
        width: 0,
        height: 0,
        rgba8: Vec::new(),
    };
    for (view_index, (yaw, pitch)) in visual_descriptor_views().into_iter().enumerate() {
        let camera = fit_camera_to_bounds_from_angles(mesh.bounds, yaw, pitch, 1.0);
        workspace.render_mesh_into(&mut image, mesh, &camera, &settings)?;
        descriptor.silhouette_masks[view_index] = silhouette_mask(&image);
        descriptor.silhouette_occupancy[view_index] = silhouette_occupancy(&image);
        descriptor.silhouette_perimeter[view_index] = silhouette_perimeter(&image);
        descriptor.depth_histogram[view_index] =
            visible_depth_histogram(&image, &workspace.depth, mesh.bounds, &camera);
    }

    Ok(descriptor)
}

/// Render a mesh to an RGBA8 image.
pub fn render_mesh(
    mesh: &TriangleMesh,
    camera: &OrbitCamera,
    settings: &RenderSettings,
) -> Result<RenderedImage, RenderError> {
    RenderWorkspace::default().render_mesh(mesh, camera, settings)
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

impl<'a> RenderTarget<'a> {
    fn prepare(
        settings: &RenderSettings,
        pixel_count: usize,
        image: &'a mut RenderedImage,
        workspace: &'a mut RenderWorkspace,
    ) -> Result<Self, RenderError> {
        let width = usize::try_from(settings.width)
            .map_err(|_| RenderError::InvalidSettings("width is too large"))?;
        let height = usize::try_from(settings.height)
            .map_err(|_| RenderError::InvalidSettings("height is too large"))?;
        let byte_len = pixel_count
            .checked_mul(4)
            .ok_or(RenderError::InvalidSettings("image byte length overflows"))?;
        image.width = settings.width;
        image.height = settings.height;
        image.rgba8.resize(byte_len, 0);
        fill_background(&mut image.rgba8, settings.background);
        workspace.depth.resize(pixel_count, f32::INFINITY);
        workspace.depth.fill(f32::INFINITY);
        workspace.normals.resize(pixel_count, Vec3::ZERO);
        workspace.normals.fill(Vec3::ZERO);

        Ok(Self {
            width,
            height,
            rgba8: image.rgba8.as_mut_slice(),
            depth: workspace.depth.as_mut_slice(),
            normals: workspace.normals.as_mut_slice(),
        })
    }
}

impl PixelBounds {
    fn for_triangle(
        projected: [ProjectedVertex; 3],
        projection: &ProjectionParams,
    ) -> Option<Self> {
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
            return None;
        }

        Some(Self {
            min_x: min_x as usize,
            max_x: max_x as usize,
            min_y: min_y as usize,
            max_y: max_y as usize,
        })
    }
}

fn visual_descriptor_views() -> [(f32, f32); VISUAL_DESCRIPTOR_CAMERA_COUNT] {
    [(0.0, 0.0), (90.0, 0.0), (35.0, 25.0), (35.0, 80.0)]
}

fn silhouette_mask(image: &RenderedImage) -> [u64; VISUAL_DESCRIPTOR_MASK_WORDS] {
    let mut words = [0_u64; VISUAL_DESCRIPTOR_MASK_WORDS];
    for index in 0..foreground_pixel_count(image)
        .min(VISUAL_DESCRIPTOR_MASK_SIZE as usize * VISUAL_DESCRIPTOR_MASK_SIZE as usize)
    {
        let byte_index = index * 4;
        if image.rgba8.get(byte_index + 3).copied().unwrap_or(0) != 0 {
            words[index / 64] |= 1_u64 << (index % 64);
        }
    }
    words
}

fn silhouette_occupancy(image: &RenderedImage) -> f32 {
    let pixel_count = foreground_pixel_count(image);
    if pixel_count == 0 {
        return 0.0;
    }
    let foreground = (0..pixel_count)
        .filter(|index| {
            let byte_index = index * 4;
            image.rgba8.get(byte_index + 3).copied().unwrap_or(0) != 0
        })
        .count();
    foreground as f32 / pixel_count as f32
}

fn silhouette_perimeter(image: &RenderedImage) -> f32 {
    let width = image.width as i32;
    let height = image.height as i32;
    if width <= 0 || height <= 0 {
        return 0.0;
    }
    let mut perimeter = 0_usize;
    for y in 0..height {
        for x in 0..width {
            if !is_foreground(image, x, y) {
                continue;
            }
            let boundary = [(x - 1, y), (x + 1, y), (x, y - 1), (x, y + 1)]
                .into_iter()
                .any(|(nx, ny)| !is_foreground(image, nx, ny));
            if boundary {
                perimeter += 1;
            }
        }
    }
    (perimeter as f32 / (2.0 * (width + height) as f32)).clamp(0.0, 4.0)
}

fn foreground_pixel_count(image: &RenderedImage) -> usize {
    let pixel_count = u64::from(image.width) * u64::from(image.height);
    usize::try_from(pixel_count).unwrap_or(0)
}

fn is_foreground(image: &RenderedImage, x: i32, y: i32) -> bool {
    if x < 0 || y < 0 || x >= image.width as i32 || y >= image.height as i32 {
        return false;
    }
    let index = (y as usize * image.width as usize + x as usize) * 4 + 3;
    image.rgba8.get(index).copied().unwrap_or(0) != 0
}

fn visible_depth_histogram(
    image: &RenderedImage,
    depth_buffer: &[f32],
    bounds: Aabb,
    camera: &OrbitCamera,
) -> [f32; VISUAL_DESCRIPTOR_DEPTH_BINS] {
    let mut histogram = [0.0_f32; VISUAL_DESCRIPTOR_DEPTH_BINS];
    let pixel_count = foreground_pixel_count(image).min(depth_buffer.len());
    if pixel_count == 0 {
        return histogram;
    }
    let (minimum, maximum) = view_depth_range(bounds, camera);
    let span = (maximum - minimum).max(f32::EPSILON);
    let mut total = 0_usize;
    for (index, depth) in depth_buffer.iter().copied().take(pixel_count).enumerate() {
        if !visible_depth_sample(image, index, depth) {
            continue;
        }
        let normalized = ((depth - minimum) / span).clamp(0.0, 1.0);
        let bin = ((normalized * VISUAL_DESCRIPTOR_DEPTH_BINS as f32).floor() as usize)
            .min(VISUAL_DESCRIPTOR_DEPTH_BINS - 1);
        histogram[bin] += 1.0;
        total += 1;
    }
    if total == 0 {
        return histogram;
    }
    let total = total as f32;
    for value in &mut histogram {
        *value /= total;
    }
    histogram
}

fn visible_depth_sample(image: &RenderedImage, index: usize, depth: f32) -> bool {
    if !depth.is_finite() || depth <= 0.0 {
        return false;
    }
    let alpha_index = index * 4 + 3;
    image.rgba8.get(alpha_index).copied().unwrap_or(0) != 0
}

fn view_depth_range(bounds: Aabb, camera: &OrbitCamera) -> (f32, f32) {
    let view = camera.view_matrix();
    let mut minimum = f32::INFINITY;
    let mut maximum = f32::NEG_INFINITY;
    for corner in bounds_corners(bounds) {
        let depth = -view.transform_point3(corner).z;
        if depth.is_finite() && depth > 0.0 {
            minimum = minimum.min(depth);
            maximum = maximum.max(depth);
        }
    }
    if minimum.is_finite() && maximum.is_finite() && maximum > minimum {
        (minimum, maximum)
    } else {
        (NEAR_PLANE, FAR_PLANE)
    }
}

fn fill_background(rgba8: &mut [u8], background: [u8; 4]) {
    for pixel in rgba8.chunks_exact_mut(4) {
        pixel.copy_from_slice(&background);
    }
}

fn fit_distance_for_bounds(bounds: Aabb, camera: &OrbitCamera, aspect_ratio: f32) -> f32 {
    let camera = camera.clamped();
    let aspect_ratio = if aspect_ratio.is_finite() && aspect_ratio > 0.0 {
        aspect_ratio
    } else {
        1.0
    };
    let half_fov = (camera.vertical_fov_degrees * 0.5).to_radians();
    let tan_y = half_fov.tan().max(f32::EPSILON);
    let tan_x = (tan_y * aspect_ratio).max(f32::EPSILON);
    let eye = camera.eye();
    let forward = normalize_or(camera.target - eye, Vec3::NEG_Z);
    let right = normalize_or(forward.cross(Vec3::Y), Vec3::X);
    let up = normalize_or(right.cross(forward), Vec3::Y);
    let mut required = MIN_DISTANCE;

    for corner in bounds_corners(bounds) {
        let delta = corner - camera.target;
        let along_forward = delta.dot(forward);
        let horizontal = delta.dot(right).abs() / tan_x - along_forward;
        let vertical = delta.dot(up).abs() / tan_y - along_forward;
        let near = NEAR_PLANE - along_forward;
        required = required.max(horizontal).max(vertical).max(near);
    }

    (required * FRAME_PADDING).clamp(MIN_DISTANCE, MAX_DISTANCE)
}

fn bounds_corners(bounds: Aabb) -> [Vec3; 8] {
    [
        Vec3::new(bounds.min.x, bounds.min.y, bounds.min.z),
        Vec3::new(bounds.min.x, bounds.min.y, bounds.max.z),
        Vec3::new(bounds.min.x, bounds.max.y, bounds.min.z),
        Vec3::new(bounds.min.x, bounds.max.y, bounds.max.z),
        Vec3::new(bounds.max.x, bounds.min.y, bounds.min.z),
        Vec3::new(bounds.max.x, bounds.min.y, bounds.max.z),
        Vec3::new(bounds.max.x, bounds.max.y, bounds.min.z),
        Vec3::new(bounds.max.x, bounds.max.y, bounds.max.z),
    ]
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
    target: &mut RenderTarget<'_>,
) {
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

    let Some(bounds) = PixelBounds::for_triangle(projected, projection) else {
        return;
    };
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

    for y in bounds.min_y..=bounds.max_y {
        for x in bounds.min_x..=bounds.max_x {
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
            let normal = normalize_or(normal, Vec3::Y);
            let wireframe =
                settings.wireframe && is_wire_pixel(edges, edge_lengths, area.is_sign_negative());
            let color = shade(normal, settings.ambient, light_direction, wireframe);
            let byte_index = pixel_index * 4;
            target.rgba8[byte_index..byte_index + 4].copy_from_slice(&color);
            target.depth[pixel_index] = depth;
            target.normals[pixel_index] = normal;
        }
    }
}

fn apply_edge_outline(target: &mut RenderTarget<'_>) {
    let original = target.rgba8.to_vec();
    for y in 0..target.height {
        for x in 0..target.width {
            let pixel_index = y * target.width + x;
            if !target.depth[pixel_index].is_finite() {
                continue;
            }
            let strength = edge_outline_strength(target, x, y);
            if strength <= 0.0 {
                continue;
            }
            let byte_index = pixel_index * 4;
            let base = Vec3::new(
                f32::from(original[byte_index]),
                f32::from(original[byte_index + 1]),
                f32::from(original[byte_index + 2]),
            );
            let mix = (EDGE_OUTLINE_MIX_AMOUNT * strength).clamp(0.0, 1.0);
            let color = base * (1.0 - mix) + EDGE_OUTLINE_MIX * mix;
            target.rgba8[byte_index] = color_channel(color.x);
            target.rgba8[byte_index + 1] = color_channel(color.y);
            target.rgba8[byte_index + 2] = color_channel(color.z);
        }
    }
}

fn edge_outline_strength(target: &RenderTarget<'_>, x: usize, y: usize) -> f32 {
    let pixel_index = y * target.width + x;
    let depth = target.depth[pixel_index];
    let normal = normalize_or(target.normals[pixel_index], Vec3::Y);
    let mut strength: f32 = 0.0;

    for (nx, ny) in edge_outline_neighbors(x, y, target.width, target.height) {
        let Some((nx, ny)) = nx.zip(ny) else {
            strength = strength.max(1.0);
            continue;
        };
        let neighbor_index = ny * target.width + nx;
        let neighbor_depth = target.depth[neighbor_index];
        if !neighbor_depth.is_finite() {
            strength = strength.max(1.0);
            continue;
        }

        let neighbor_normal = normalize_or(target.normals[neighbor_index], normal);
        let normal_dot = normal.dot(neighbor_normal).clamp(-1.0, 1.0);
        if normal_dot < EDGE_OUTLINE_NORMAL_DOT_THRESHOLD {
            strength = strength.max(0.95);
        }

        let depth_scale = depth.abs().max(neighbor_depth.abs()).max(1.0);
        let depth_delta = (depth - neighbor_depth).abs() / depth_scale;
        if depth_delta > EDGE_OUTLINE_DEPTH_THRESHOLD {
            strength = strength.max(0.7);
        }
    }

    strength
}

fn edge_outline_neighbors(
    x: usize,
    y: usize,
    width: usize,
    height: usize,
) -> [(Option<usize>, Option<usize>); 4] {
    [
        (x.checked_sub(1), Some(y)),
        ((x + 1 < width).then_some(x + 1), Some(y)),
        (Some(x), y.checked_sub(1)),
        (Some(x), (y + 1 < height).then_some(y + 1)),
    ]
}

fn is_fully_outside_depth(vertices: [RenderVertex; 3]) -> bool {
    vertices
        .iter()
        .all(|vertex| !is_finite_vec3(vertex.view) || vertex.view.z > -NEAR_PLANE)
        || vertices
            .iter()
            .all(|vertex| !is_finite_vec3(vertex.view) || -vertex.view.z > FAR_PLANE)
}

fn is_outside_frustum(vertices: &[RenderVertex], projection: &ProjectionParams) -> bool {
    if vertices.is_empty() || vertices.iter().any(|vertex| !is_finite_vec3(vertex.view)) {
        return true;
    }

    vertices.iter().all(|vertex| -vertex.view.z > FAR_PLANE)
        || vertices.iter().all(|vertex| {
            let depth = -vertex.view.z;
            vertex.view.x * projection.focal_x < -depth
        })
        || vertices.iter().all(|vertex| {
            let depth = -vertex.view.z;
            vertex.view.x * projection.focal_x > depth
        })
        || vertices.iter().all(|vertex| {
            let depth = -vertex.view.z;
            vertex.view.y * projection.focal_y < -depth
        })
        || vertices.iter().all(|vertex| {
            let depth = -vertex.view.z;
            vertex.view.y * projection.focal_y > depth
        })
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
    if !depth.is_finite() || depth < NEAR_PLANE {
        return None;
    }
    let depth = depth.max(NEAR_PLANE);
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
        color = color * 0.25 + WIREFRAME_MIX * 0.75;
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

fn clip_triangle_to_near(vertices: [RenderVertex; 3]) -> ClippedTriangle {
    let mut output = ClippedTriangle::new(vertices[0]);
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
    let mut view = a.view.lerp(b.view, t);
    view.z = -NEAR_PLANE;
    let normal = normalize_or(a.normal.lerp(b.normal, t), a.normal);
    RenderVertex { view, normal }
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

fn render_normal(mesh: &TriangleMesh, index: usize, face_normal: Vec3) -> Vec3 {
    let normal = vertex_normal(mesh, index).unwrap_or(face_normal);
    if normal.dot(face_normal) < 0.0 {
        -normal
    } else {
        normal
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

fn hash_mesh(mesh: &TriangleMesh) -> u64 {
    let mut hasher = StableHasher::new(b"shape-render-mesh-v1");
    hasher.write_u64(mesh.positions.len() as u64);
    for position in &mesh.positions {
        hasher.write_f32(position[0]);
        hasher.write_f32(position[1]);
        hasher.write_f32(position[2]);
    }

    hasher.write_u64(mesh.normals.len() as u64);
    for normal in &mesh.normals {
        hasher.write_f32(normal[0]);
        hasher.write_f32(normal[1]);
        hasher.write_f32(normal[2]);
    }

    hasher.write_u64(mesh.indices.len() as u64);
    for index in &mesh.indices {
        hasher.write_u32(*index);
    }
    hasher.finish()
}

fn hash_camera(camera: &OrbitCamera) -> u64 {
    let camera = camera.clamped();
    let mut hasher = StableHasher::new(b"shape-render-camera-v1");
    hasher.write_vec3(camera.target);
    hasher.write_f32(camera.yaw_degrees);
    hasher.write_f32(camera.pitch_degrees);
    hasher.write_f32(camera.distance);
    hasher.write_f32(camera.vertical_fov_degrees);
    hasher.finish()
}

fn hash_settings(settings: &RenderSettings) -> u64 {
    let mut hasher = StableHasher::new(b"shape-render-settings-v1");
    hasher.write_u32(settings.width);
    hasher.write_u32(settings.height);
    hasher.write_bytes(&settings.background);
    hasher.write_f32(settings.ambient.clamp(0.0, 1.0));
    hasher.write_vec3(settings.light_direction.normalize());
    hasher.write_bool(settings.wireframe);
    hasher.write_bool(settings.edge_outline);
    hasher.finish()
}

#[derive(Debug, Copy, Clone)]
struct StableHasher {
    value: u64,
}

impl StableHasher {
    fn new(domain: &[u8]) -> Self {
        let mut hasher = Self {
            value: CACHE_HASH_OFFSET,
        };
        hasher.write_bytes(domain);
        hasher
    }

    fn write_bool(&mut self, value: bool) {
        self.write_u8(u8::from(value));
    }

    fn write_u8(&mut self, value: u8) {
        self.write_bytes(&[value]);
    }

    fn write_u32(&mut self, value: u32) {
        self.write_bytes(&value.to_le_bytes());
    }

    fn write_u64(&mut self, value: u64) {
        self.write_bytes(&value.to_le_bytes());
    }

    fn write_f32(&mut self, value: f32) {
        self.write_u32(canonical_f32_bits(value));
    }

    fn write_vec3(&mut self, value: Vec3) {
        self.write_f32(value.x);
        self.write_f32(value.y);
        self.write_f32(value.z);
    }

    fn write_bytes(&mut self, bytes: &[u8]) {
        for byte in bytes {
            self.value ^= u64::from(*byte);
            self.value = self.value.wrapping_mul(CACHE_HASH_PRIME);
        }
    }

    fn finish(self) -> u64 {
        self.value
    }
}

fn canonical_f32_bits(value: f32) -> u32 {
    if value == 0.0 {
        0.0f32.to_bits()
    } else if value.is_nan() {
        f32::NAN.to_bits()
    } else {
        value.to_bits()
    }
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
            edge_outline: false,
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

    fn cube_mesh() -> TriangleMesh {
        let faces = [
            (
                [
                    [-1.0, -1.0, 1.0],
                    [1.0, -1.0, 1.0],
                    [1.0, 1.0, 1.0],
                    [-1.0, 1.0, 1.0],
                ],
                [0.0, 0.0, 1.0],
            ),
            (
                [
                    [1.0, -1.0, -1.0],
                    [-1.0, -1.0, -1.0],
                    [-1.0, 1.0, -1.0],
                    [1.0, 1.0, -1.0],
                ],
                [0.0, 0.0, -1.0],
            ),
            (
                [
                    [1.0, -1.0, 1.0],
                    [1.0, -1.0, -1.0],
                    [1.0, 1.0, -1.0],
                    [1.0, 1.0, 1.0],
                ],
                [1.0, 0.0, 0.0],
            ),
            (
                [
                    [-1.0, -1.0, -1.0],
                    [-1.0, -1.0, 1.0],
                    [-1.0, 1.0, 1.0],
                    [-1.0, 1.0, -1.0],
                ],
                [-1.0, 0.0, 0.0],
            ),
            (
                [
                    [-1.0, 1.0, 1.0],
                    [1.0, 1.0, 1.0],
                    [1.0, 1.0, -1.0],
                    [-1.0, 1.0, -1.0],
                ],
                [0.0, 1.0, 0.0],
            ),
            (
                [
                    [-1.0, -1.0, -1.0],
                    [1.0, -1.0, -1.0],
                    [1.0, -1.0, 1.0],
                    [-1.0, -1.0, 1.0],
                ],
                [0.0, -1.0, 0.0],
            ),
        ];
        let mut positions = Vec::with_capacity(24);
        let mut normals = Vec::with_capacity(24);
        let mut indices = Vec::with_capacity(36);
        for (face_index, (face_positions, normal)) in faces.into_iter().enumerate() {
            let base = u32::try_from(face_index * 4).expect("cube face index fits");
            positions.extend(face_positions);
            normals.extend([normal; 4]);
            indices.extend([base, base + 1, base + 2, base, base + 2, base + 3]);
        }
        TriangleMesh {
            positions,
            normals,
            indices,
            bounds: Aabb {
                min: Vec3::splat(-1.0),
                max: Vec3::splat(1.0),
            },
        }
    }

    fn darkened_pixel_count(base: &RenderedImage, outlined: &RenderedImage) -> usize {
        base.rgba8
            .chunks_exact(4)
            .zip(outlined.rgba8.chunks_exact(4))
            .filter(|(base, outlined)| {
                let base_luma = luma(base);
                let outlined_luma = luma(outlined);
                outlined[3] == 255 && outlined_luma + 18.0 < base_luma
            })
            .count()
    }

    fn luma(pixel: &[u8]) -> f32 {
        f32::from(pixel[0]) * 0.2126 + f32::from(pixel[1]) * 0.7152 + f32::from(pixel[2]) * 0.0722
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
    fn origin_centered_camera_fit_sees_offset_bounds() {
        let bounds = Aabb {
            min: Vec3::new(1.0, -1.0, -0.5),
            max: Vec3::new(3.0, 1.0, 0.5),
        };
        let camera = fit_camera_to_bounds_from_angles_around_origin(bounds, 35.0, 20.0, 1.0);
        let view_projection = camera.view_projection_matrix(1.0);

        assert_eq!(camera.target, Vec3::ZERO);
        for x in [bounds.min.x, bounds.max.x] {
            for y in [bounds.min.y, bounds.max.y] {
                for z in [bounds.min.z, bounds.max.z] {
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
    fn clay_readability_settings_add_display_edge_outline() {
        let mesh = cube_mesh();
        let camera = fit_camera_to_bounds(mesh.bounds);
        let mut base_settings = clay_readability_render_settings(128, 128);
        base_settings.edge_outline = false;
        let outlined_settings = clay_readability_render_settings(128, 128);

        let base = render_mesh(&mesh, &camera, &base_settings).expect("base cube render");
        let outlined =
            render_mesh(&mesh, &camera, &outlined_settings).expect("outlined cube render");

        assert!(outlined_settings.edge_outline);
        assert!(!outlined_settings.wireframe);
        assert_ne!(base.rgba8, outlined.rgba8);
        assert!(
            darkened_pixel_count(&base, &outlined) > 96,
            "edge outline should darken crease/silhouette pixels"
        );
    }

    #[test]
    fn visual_descriptor_is_deterministic_and_view_dependent() {
        let first =
            visual_descriptor_for_mesh(&triangle_mesh()).expect("visual descriptor should render");
        let second =
            visual_descriptor_for_mesh(&triangle_mesh()).expect("visual descriptor should render");

        assert_eq!(first, second);
        assert!(first.silhouette_occupancy[0] > 0.01);
        assert!(first.silhouette_perimeter[0] > 0.0);
        assert_ne!(first.silhouette_masks[0], first.silhouette_masks[1]);
        assert!(
            first.depth_histogram[0]
                .iter()
                .all(|value| value.is_finite())
        );
    }

    #[test]
    fn visual_descriptor_depth_histogram_ignores_unreferenced_vertices() {
        let base = triangle_mesh();
        let mut topology_noise = base.clone();
        topology_noise.positions.extend([
            [-0.25, -0.25, 0.0],
            [0.25, -0.25, 0.0],
            [0.0, 0.25, 0.0],
        ]);
        topology_noise.normals.extend([[0.0, 0.0, 1.0]; 3]);

        let base_descriptor =
            visual_descriptor_for_mesh(&base).expect("base descriptor should render");
        let noisy_descriptor =
            visual_descriptor_for_mesh(&topology_noise).expect("noisy descriptor should render");

        assert_eq!(
            base_descriptor.depth_histogram,
            noisy_descriptor.depth_histogram
        );
        assert_eq!(
            base_descriptor.silhouette_masks,
            noisy_descriptor.silhouette_masks
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
    fn preview_renders_reversed_triangle_as_two_sided_clay() {
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
                .any(|pixel| pixel != render_settings.background),
            "reversed preview triangles should remain visible from orbit views"
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
    fn render_cache_key_is_stable_for_equivalent_inputs() {
        let mesh = triangle_mesh();
        let camera = camera();
        let settings = settings(64, 64);
        let first = RenderCacheKey::new(&mesh, &camera, &settings).expect("cache key should build");
        let second =
            RenderCacheKey::new(&mesh, &camera, &settings).expect("cache key should repeat");

        assert_eq!(first, second);

        let mut equivalent_camera = camera.clone();
        equivalent_camera.yaw_degrees += 360.0;
        let mut equivalent_settings = settings.clone();
        equivalent_settings.light_direction *= 2.0;

        assert_eq!(
            first.camera_hash,
            RenderCacheKey::new(&mesh, &equivalent_camera, &settings)
                .expect("equivalent camera should key")
                .camera_hash
        );
        assert_eq!(
            first.settings_hash,
            RenderCacheKey::new(&mesh, &camera, &equivalent_settings)
                .expect("equivalent settings should key")
                .settings_hash
        );
    }

    #[test]
    fn render_cache_key_changes_for_mesh_camera_and_settings() {
        let mesh = triangle_mesh();
        let camera = camera();
        let settings = settings(64, 64);
        let base = RenderCacheKey::new(&mesh, &camera, &settings).expect("base key should build");

        let mut changed_mesh = mesh.clone();
        changed_mesh.positions[0][0] -= 0.125;
        let mesh_key = RenderCacheKey::new(&changed_mesh, &camera, &settings)
            .expect("changed mesh should key");
        assert_ne!(base.mesh_hash, mesh_key.mesh_hash);

        let mut changed_camera = camera.clone();
        changed_camera.pitch_degrees += 5.0;
        let camera_key = RenderCacheKey::new(&mesh, &changed_camera, &settings)
            .expect("changed camera should key");
        assert_ne!(base.camera_hash, camera_key.camera_hash);

        let mut changed_settings = settings.clone();
        changed_settings.wireframe = true;
        let settings_key = RenderCacheKey::new(&mesh, &camera, &changed_settings)
            .expect("changed settings should key");
        assert_ne!(base.settings_hash, settings_key.settings_hash);
    }

    #[test]
    fn render_cache_reuses_matching_image_and_rerenders_after_key_change() {
        let mesh = triangle_mesh();
        let camera = camera();
        let mut settings = settings(64, 64);
        let mut cache = RenderCache::default();

        let first_reused = cache
            .render_mesh(&mesh, &camera, &settings)
            .expect("first cached render should succeed")
            .reused;
        assert!(!first_reused);

        let second_reused = cache
            .render_mesh(&mesh, &camera, &settings)
            .expect("second cached render should succeed")
            .reused;
        assert!(second_reused);

        settings.width = 65;
        let changed = cache
            .render_mesh(&mesh, &camera, &settings)
            .expect("changed settings render should succeed");
        assert!(!changed.reused);
        assert_eq!(changed.image.width, 65);
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
