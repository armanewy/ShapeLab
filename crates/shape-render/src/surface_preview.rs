#![forbid(unsafe_code)]

//! Deterministic CPU textured surface previews.
//!
//! The preview path accepts already-decoded RGBA texture payloads so this crate
//! stays independent of PNG decoding. `TextureSampling::Bilinear` is the
//! default for final previews because it reduces shimmering on generated UVs;
//! `TextureSampling::Nearest` is retained for tests and diagnostics where exact
//! texel selection matters.

use std::collections::{BTreeMap, BTreeSet};

use glam::{Vec2, Vec3, Vec4};
use serde::{Deserialize, Serialize};
use shape_mesh::TriangleMesh;
use thiserror::Error;

use crate::{
    OrbitCamera, RenderError, RenderSettings, RenderedImage, fit_camera_to_bounds_from_angles,
};

const SURFACE_PREVIEW_REPORT_SCHEMA_VERSION: u32 = 1;
const MAX_PIXELS: u64 = 16_777_216;
const MIN_AREA: f32 = 1.0e-5;
const EDGE_EPSILON: f32 = 1.0e-4;
const MIN_NORMAL_LENGTH_SQUARED: f32 = 1.0e-12;

/// Texture channel vocabulary used by the preview renderer.
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SurfacePreviewTextureChannel {
    /// Base color/albedo texture.
    BaseColor,
    /// Normal map texture. Parsed for validation, not used for tangent-space shading.
    Normal,
    /// Metallic/roughness texture.
    MetallicRoughness,
    /// Ambient occlusion texture.
    Occlusion,
}

/// Texture sampling mode.
#[derive(Debug, Copy, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TextureSampling {
    /// Select the nearest texel after wrapping UVs into 0..1.
    Nearest,
    /// Bilinearly blend the four nearest wrapped texels.
    #[default]
    Bilinear,
}

/// Decoded RGBA texture payload.
#[derive(Debug, Clone, PartialEq)]
pub struct SurfacePreviewTexture {
    /// Material recipe ID this texture belongs to.
    pub material_id: String,
    /// Texture channel.
    pub channel: SurfacePreviewTextureChannel,
    /// Width in pixels.
    pub width: u32,
    /// Height in pixels.
    pub height: u32,
    /// Row-major RGBA8 payload.
    pub rgba8: Vec<u8>,
}

/// Material slot to material recipe binding for preview rendering.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SurfacePreviewMaterialBinding {
    /// Stable material slot ID.
    pub slot_id: String,
    /// Human-facing slot name.
    pub display_name: String,
    /// Referenced material recipe ID.
    pub material_id: String,
}

/// Triangle to material slot binding.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SurfacePreviewTriangleBinding {
    /// Zero-based triangle index.
    pub triangle_index: u32,
    /// Stable material slot ID.
    pub material_slot_id: String,
}

/// Surface preview request.
#[derive(Debug, Clone, PartialEq)]
pub struct SurfacePreviewRequest {
    /// Static mesh to render.
    pub mesh: TriangleMesh,
    /// TEXCOORD_0 coordinates in exported vertex order.
    pub texcoord0: Vec<[f32; 2]>,
    /// Material slot bindings.
    pub material_bindings: Vec<SurfacePreviewMaterialBinding>,
    /// Triangle material slot bindings.
    pub triangle_bindings: Vec<SurfacePreviewTriangleBinding>,
    /// Decoded texture payloads.
    pub textures: Vec<SurfacePreviewTexture>,
    /// Camera.
    pub camera: OrbitCamera,
    /// Render settings.
    pub render_settings: RenderSettings,
    /// Texture sampling mode.
    pub sampling: TextureSampling,
}

/// Surface preview validation/render report.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SurfacePreviewReport {
    /// Report schema version.
    pub schema_version: u32,
    /// True when the preview rendered with all required payloads.
    pub valid: bool,
    /// Stable issue codes.
    pub issue_codes: Vec<String>,
    /// Texture sampling mode used.
    pub sampling: TextureSampling,
    /// Rendered visible surface pixels.
    pub visible_surface_pixel_count: u32,
    /// Material slots seen by visible pixels.
    pub visible_material_slots: Vec<String>,
}

/// Rendered textured preview plus report.
#[derive(Debug, Clone, PartialEq)]
pub struct SurfacePreviewOutput {
    /// Textured preview image.
    pub image: RenderedImage,
    /// Validation/render report.
    pub report: SurfacePreviewReport,
}

/// Surface preview errors.
#[derive(Debug, Clone, Error, PartialEq, Eq)]
pub enum SurfacePreviewError {
    /// Input validation failed.
    #[error("invalid surface preview input")]
    InvalidInput(SurfacePreviewReport),
    /// Render settings or mesh validation failed.
    #[error("surface preview render failed: {0}")]
    Render(#[from] RenderError),
}

/// Build a default square preview request from mesh bounds and three-quarter
/// camera framing.
#[must_use]
pub fn default_surface_preview_request(
    mesh: TriangleMesh,
    texcoord0: Vec<[f32; 2]>,
    material_bindings: Vec<SurfacePreviewMaterialBinding>,
    triangle_bindings: Vec<SurfacePreviewTriangleBinding>,
    textures: Vec<SurfacePreviewTexture>,
    size: u32,
) -> SurfacePreviewRequest {
    SurfacePreviewRequest {
        camera: fit_camera_to_bounds_from_angles(mesh.bounds, 38.0, 18.0, 1.0),
        mesh,
        texcoord0,
        material_bindings,
        triangle_bindings,
        textures,
        render_settings: RenderSettings {
            width: size,
            height: size,
            background: [20, 22, 24, 255],
            ambient: 0.38,
            light_direction: Vec3::new(-0.45, -0.8, -0.28).normalize(),
            wireframe: false,
        },
        sampling: TextureSampling::Bilinear,
    }
}

/// Validate surface preview inputs without rendering.
#[must_use]
pub fn validate_surface_preview_request(request: &SurfacePreviewRequest) -> SurfacePreviewReport {
    let mut issue_codes = Vec::new();
    validate_mesh(&request.mesh, &mut issue_codes);
    if request.texcoord0.len() != request.mesh.positions.len() {
        issue_codes.push("texcoord0_count_mismatch".to_owned());
    }
    if request
        .texcoord0
        .iter()
        .any(|uv| !uv[0].is_finite() || !uv[1].is_finite())
    {
        issue_codes.push("texcoord0_non_finite".to_owned());
    }
    if request.material_bindings.is_empty() {
        issue_codes.push("material_slot_bindings_missing".to_owned());
    }
    if request.triangle_bindings.len() != request.mesh.indices.len() / 3 {
        issue_codes.push("triangle_material_binding_count_mismatch".to_owned());
    }
    validate_materials_and_textures(request, &mut issue_codes);
    validate_render_settings(&request.render_settings, &mut issue_codes);

    SurfacePreviewReport {
        schema_version: SURFACE_PREVIEW_REPORT_SCHEMA_VERSION,
        valid: issue_codes.is_empty(),
        issue_codes,
        sampling: request.sampling,
        visible_surface_pixel_count: 0,
        visible_material_slots: Vec::new(),
    }
}

/// Render a deterministic textured preview.
pub fn render_surface_preview(
    request: &SurfacePreviewRequest,
) -> Result<SurfacePreviewOutput, SurfacePreviewError> {
    let report = validate_surface_preview_request(request);
    if !report.valid {
        return Err(SurfacePreviewError::InvalidInput(report));
    }

    let width = request.render_settings.width as usize;
    let height = request.render_settings.height as usize;
    let pixel_count = width
        .checked_mul(height)
        .ok_or(RenderError::InvalidSettings(
            "image dimensions are too large",
        ))?;
    let mut rgba8 = Vec::with_capacity(pixel_count * 4);
    for _ in 0..pixel_count {
        rgba8.extend_from_slice(&request.render_settings.background);
    }
    let mut depth = vec![f32::INFINITY; pixel_count];
    let mut visible_slots = BTreeSet::<String>::new();
    let mut visible_pixels = 0_u32;

    let slot_to_material = request
        .material_bindings
        .iter()
        .map(|binding| (binding.slot_id.as_str(), binding.material_id.as_str()))
        .collect::<BTreeMap<_, _>>();
    let textures = texture_lookup(&request.textures);
    let triangle_slots = request
        .triangle_bindings
        .iter()
        .map(|binding| {
            (
                binding.triangle_index as usize,
                binding.material_slot_id.as_str(),
            )
        })
        .collect::<BTreeMap<_, _>>();

    let view = request.camera.view_matrix();
    let projection = request.camera.projection_matrix(
        request.render_settings.width as f32 / request.render_settings.height as f32,
    );
    let view_projection = projection * view;
    let light_direction = request.render_settings.light_direction.normalize();

    for (triangle_index, indices) in request.mesh.indices.chunks_exact(3).enumerate() {
        let slot_id = triangle_slots
            .get(&triangle_index)
            .copied()
            .unwrap_or("missing-slot");
        let Some(material_id) = slot_to_material.get(slot_id).copied() else {
            continue;
        };
        let Some(texture) = textures.get(&(material_id, SurfacePreviewTextureChannel::BaseColor))
        else {
            continue;
        };
        let i0 = indices[0] as usize;
        let i1 = indices[1] as usize;
        let i2 = indices[2] as usize;
        let projected = [
            project_vertex(
                &request.mesh,
                &request.texcoord0,
                i0,
                view,
                view_projection,
                request.render_settings.width,
                request.render_settings.height,
            ),
            project_vertex(
                &request.mesh,
                &request.texcoord0,
                i1,
                view,
                view_projection,
                request.render_settings.width,
                request.render_settings.height,
            ),
            project_vertex(
                &request.mesh,
                &request.texcoord0,
                i2,
                view,
                view_projection,
                request.render_settings.width,
                request.render_settings.height,
            ),
        ];
        let Some(projected) = option_array(projected) else {
            continue;
        };
        let area = edge(
            projected[0].screen,
            projected[1].screen,
            projected[2].screen,
        );
        if area.abs() < MIN_AREA {
            continue;
        }
        let bounds = pixel_bounds(projected, width, height);
        for y in bounds.min_y..=bounds.max_y {
            for x in bounds.min_x..=bounds.max_x {
                let p = Vec2::new(x as f32 + 0.5, y as f32 + 0.5);
                let edges = [
                    edge(projected[1].screen, projected[2].screen, p),
                    edge(projected[2].screen, projected[0].screen, p),
                    edge(projected[0].screen, projected[1].screen, p),
                ];
                if !inside_triangle(edges, area) {
                    continue;
                }
                let weights = [edges[0] / area, edges[1] / area, edges[2] / area];
                let inv_depth = projected[0].inv_depth * weights[0]
                    + projected[1].inv_depth * weights[1]
                    + projected[2].inv_depth * weights[2];
                if inv_depth <= 0.0 {
                    continue;
                }
                let z = 1.0 / inv_depth;
                let pixel_index = y * width + x;
                if z >= depth[pixel_index] {
                    continue;
                }
                depth[pixel_index] = z;
                let uv = (projected[0].uv_over_depth * weights[0]
                    + projected[1].uv_over_depth * weights[1]
                    + projected[2].uv_over_depth * weights[2])
                    / inv_depth;
                let normal = (projected[0].normal_over_depth * weights[0]
                    + projected[1].normal_over_depth * weights[1]
                    + projected[2].normal_over_depth * weights[2])
                    / inv_depth;
                let texel = sample_texture(texture, uv, request.sampling);
                let shaded = shade(
                    texel,
                    normal,
                    request.render_settings.ambient,
                    light_direction,
                );
                let byte = pixel_index * 4;
                rgba8[byte..byte + 4].copy_from_slice(&shaded);
                visible_slots.insert(slot_id.to_owned());
                visible_pixels = visible_pixels.saturating_add(1);
            }
        }
    }

    let image = RenderedImage {
        width: request.render_settings.width,
        height: request.render_settings.height,
        rgba8,
    };
    let mut report = validate_surface_preview_request(request);
    report.visible_surface_pixel_count = visible_pixels;
    report.visible_material_slots = visible_slots.into_iter().collect();
    Ok(SurfacePreviewOutput { image, report })
}

/// Render a material-slot overlay using deterministic colors instead of
/// texture payloads.
pub fn render_material_slot_overlay(
    request: &SurfacePreviewRequest,
) -> Result<SurfacePreviewOutput, SurfacePreviewError> {
    let mut overlay_request = request.clone();
    overlay_request.textures = request
        .material_bindings
        .iter()
        .map(|binding| SurfacePreviewTexture {
            material_id: binding.material_id.clone(),
            channel: SurfacePreviewTextureChannel::BaseColor,
            width: 1,
            height: 1,
            rgba8: slot_color(&binding.slot_id).to_vec(),
        })
        .collect();
    render_surface_preview(&overlay_request)
}

/// Compose previews into a horizontal contact sheet.
#[must_use]
pub fn surface_preview_contact_sheet(images: &[RenderedImage]) -> RenderedImage {
    if images.is_empty() {
        return RenderedImage {
            width: 1,
            height: 1,
            rgba8: vec![20, 22, 24, 255],
        };
    }
    let padding = 12_u32;
    let cell_width = images.iter().map(|image| image.width).max().unwrap_or(1);
    let cell_height = images.iter().map(|image| image.height).max().unwrap_or(1);
    let width =
        padding + (cell_width + padding) * u32::try_from(images.len()).unwrap_or(u32::MAX - 1);
    let height = cell_height + padding * 2;
    let mut sheet = RenderedImage {
        width,
        height,
        rgba8: [18, 20, 22, 255].repeat((width as usize) * (height as usize)),
    };
    for (index, image) in images.iter().enumerate() {
        let origin_x = padding + u32::try_from(index).unwrap_or(0) * (cell_width + padding);
        blit(&mut sheet, image, origin_x, padding);
    }
    sheet
}

fn validate_mesh(mesh: &TriangleMesh, issue_codes: &mut Vec<String>) {
    if mesh.positions.is_empty() {
        issue_codes.push("mesh_positions_missing".to_owned());
    }
    if mesh.positions.len() != mesh.normals.len() {
        issue_codes.push("mesh_normals_count_mismatch".to_owned());
    }
    if !mesh.indices.len().is_multiple_of(3) {
        issue_codes.push("mesh_indices_not_triangles".to_owned());
    }
    if mesh
        .indices
        .iter()
        .any(|index| *index as usize >= mesh.positions.len())
    {
        issue_codes.push("mesh_index_out_of_bounds".to_owned());
    }
}

fn validate_materials_and_textures(request: &SurfacePreviewRequest, issue_codes: &mut Vec<String>) {
    let slot_ids = request
        .material_bindings
        .iter()
        .map(|binding| binding.slot_id.as_str())
        .collect::<BTreeSet<_>>();
    let material_ids = request
        .material_bindings
        .iter()
        .map(|binding| binding.material_id.as_str())
        .collect::<BTreeSet<_>>();
    for binding in &request.triangle_bindings {
        if !slot_ids.contains(binding.material_slot_id.as_str()) {
            issue_codes.push("triangle_binding_unknown_material_slot".to_owned());
        }
    }
    let textures = texture_lookup(&request.textures);
    for material_id in material_ids {
        if !textures.contains_key(&(material_id, SurfacePreviewTextureChannel::BaseColor)) {
            issue_codes.push("missing_base_color_texture".to_owned());
        }
    }
    for texture in &request.textures {
        let expected = (texture.width as usize)
            .saturating_mul(texture.height as usize)
            .saturating_mul(4);
        if texture.width == 0 || texture.height == 0 || texture.rgba8.len() != expected {
            issue_codes.push("texture_payload_dimensions_invalid".to_owned());
        }
    }
}

fn validate_render_settings(settings: &RenderSettings, issue_codes: &mut Vec<String>) {
    let pixels = u64::from(settings.width) * u64::from(settings.height);
    if settings.width == 0 || settings.height == 0 || pixels > MAX_PIXELS {
        issue_codes.push("surface_preview_dimensions_invalid".to_owned());
    }
    if !settings.ambient.is_finite() {
        issue_codes.push("surface_preview_ambient_invalid".to_owned());
    }
    if !settings.light_direction.x.is_finite()
        || !settings.light_direction.y.is_finite()
        || !settings.light_direction.z.is_finite()
        || settings.light_direction.length_squared() <= MIN_NORMAL_LENGTH_SQUARED
    {
        issue_codes.push("surface_preview_light_direction_invalid".to_owned());
    }
}

fn texture_lookup(
    textures: &[SurfacePreviewTexture],
) -> BTreeMap<(&str, SurfacePreviewTextureChannel), &SurfacePreviewTexture> {
    textures
        .iter()
        .map(|texture| ((texture.material_id.as_str(), texture.channel), texture))
        .collect()
}

#[derive(Debug, Copy, Clone)]
struct ProjectedSurfaceVertex {
    screen: Vec2,
    inv_depth: f32,
    uv_over_depth: Vec2,
    normal_over_depth: Vec3,
}

fn project_vertex(
    mesh: &TriangleMesh,
    texcoord0: &[[f32; 2]],
    index: usize,
    view: glam::Mat4,
    view_projection: glam::Mat4,
    width: u32,
    height: u32,
) -> Option<ProjectedSurfaceVertex> {
    let position = Vec3::from_array(*mesh.positions.get(index)?);
    let clip = view_projection * Vec4::new(position.x, position.y, position.z, 1.0);
    if !clip.w.is_finite() || clip.w <= 0.0 {
        return None;
    }
    let ndc = clip.truncate() / clip.w;
    if !ndc.x.is_finite() || !ndc.y.is_finite() || ndc.z < -1.0 || ndc.z > 1.0 {
        return None;
    }
    let view_position = view.transform_point3(position);
    let depth = (-view_position.z).max(0.0001);
    let inv_depth = 1.0 / depth;
    let uv = texcoord0.get(index).copied()?;
    let normal = mesh
        .normals
        .get(index)
        .map(|normal| Vec3::from_array(*normal))
        .filter(|normal| normal.length_squared() > MIN_NORMAL_LENGTH_SQUARED)
        .unwrap_or(Vec3::Y)
        .normalize();
    Some(ProjectedSurfaceVertex {
        screen: Vec2::new(
            (ndc.x * 0.5 + 0.5) * width as f32 - 0.5,
            (0.5 - ndc.y * 0.5) * height as f32 - 0.5,
        ),
        inv_depth,
        uv_over_depth: Vec2::new(uv[0], uv[1]) * inv_depth,
        normal_over_depth: normal * inv_depth,
    })
}

fn option_array<T: Copy>(items: [Option<T>; 3]) -> Option<[T; 3]> {
    Some([items[0]?, items[1]?, items[2]?])
}

#[derive(Debug, Copy, Clone)]
struct PixelBounds {
    min_x: usize,
    max_x: usize,
    min_y: usize,
    max_y: usize,
}

fn pixel_bounds(vertices: [ProjectedSurfaceVertex; 3], width: usize, height: usize) -> PixelBounds {
    let min_x = vertices
        .iter()
        .map(|vertex| vertex.screen.x.floor() as i32)
        .min()
        .unwrap_or(0)
        .clamp(0, width.saturating_sub(1) as i32) as usize;
    let max_x = vertices
        .iter()
        .map(|vertex| vertex.screen.x.ceil() as i32)
        .max()
        .unwrap_or(0)
        .clamp(0, width.saturating_sub(1) as i32) as usize;
    let min_y = vertices
        .iter()
        .map(|vertex| vertex.screen.y.floor() as i32)
        .min()
        .unwrap_or(0)
        .clamp(0, height.saturating_sub(1) as i32) as usize;
    let max_y = vertices
        .iter()
        .map(|vertex| vertex.screen.y.ceil() as i32)
        .max()
        .unwrap_or(0)
        .clamp(0, height.saturating_sub(1) as i32) as usize;
    PixelBounds {
        min_x,
        max_x,
        min_y,
        max_y,
    }
}

fn edge(a: Vec2, b: Vec2, c: Vec2) -> f32 {
    (c.x - a.x) * (b.y - a.y) - (c.y - a.y) * (b.x - a.x)
}

fn inside_triangle(edges: [f32; 3], area: f32) -> bool {
    if area.is_sign_positive() {
        edges.iter().all(|edge| *edge >= -EDGE_EPSILON)
    } else {
        edges.iter().all(|edge| *edge <= EDGE_EPSILON)
    }
}

fn sample_texture(texture: &SurfacePreviewTexture, uv: Vec2, sampling: TextureSampling) -> [u8; 4] {
    match sampling {
        TextureSampling::Nearest => sample_nearest(texture, uv),
        TextureSampling::Bilinear => sample_bilinear(texture, uv),
    }
}

fn sample_nearest(texture: &SurfacePreviewTexture, uv: Vec2) -> [u8; 4] {
    let x = (wrap01(uv.x) * texture.width.saturating_sub(1) as f32).round() as u32;
    let y = ((1.0 - wrap01(uv.y)) * texture.height.saturating_sub(1) as f32).round() as u32;
    texel(texture, x, y)
}

fn sample_bilinear(texture: &SurfacePreviewTexture, uv: Vec2) -> [u8; 4] {
    let x = wrap01(uv.x) * texture.width.saturating_sub(1) as f32;
    let y = (1.0 - wrap01(uv.y)) * texture.height.saturating_sub(1) as f32;
    let x0 = x.floor() as u32;
    let y0 = y.floor() as u32;
    let x1 = (x0 + 1).min(texture.width.saturating_sub(1));
    let y1 = (y0 + 1).min(texture.height.saturating_sub(1));
    let tx = x - x0 as f32;
    let ty = y - y0 as f32;
    let c00 = texel(texture, x0, y0);
    let c10 = texel(texture, x1, y0);
    let c01 = texel(texture, x0, y1);
    let c11 = texel(texture, x1, y1);
    let mut out = [0_u8; 4];
    for channel in 0..4 {
        let top = lerp(f32::from(c00[channel]), f32::from(c10[channel]), tx);
        let bottom = lerp(f32::from(c01[channel]), f32::from(c11[channel]), tx);
        out[channel] = lerp(top, bottom, ty).round().clamp(0.0, 255.0) as u8;
    }
    out
}

fn texel(texture: &SurfacePreviewTexture, x: u32, y: u32) -> [u8; 4] {
    let index = ((y as usize) * (texture.width as usize) + (x as usize)) * 4;
    [
        texture.rgba8[index],
        texture.rgba8[index + 1],
        texture.rgba8[index + 2],
        texture.rgba8[index + 3],
    ]
}

fn shade(texel: [u8; 4], normal: Vec3, ambient: f32, light_direction: Vec3) -> [u8; 4] {
    let normal = if normal.length_squared() > MIN_NORMAL_LENGTH_SQUARED {
        normal.normalize()
    } else {
        Vec3::Y
    };
    let lambert = normal.dot(-light_direction).max(0.0);
    let intensity =
        (ambient.clamp(0.0, 1.0) + (1.0 - ambient.clamp(0.0, 1.0)) * lambert).clamp(0.0, 1.0);
    [
        (f32::from(texel[0]) * intensity).round().clamp(0.0, 255.0) as u8,
        (f32::from(texel[1]) * intensity).round().clamp(0.0, 255.0) as u8,
        (f32::from(texel[2]) * intensity).round().clamp(0.0, 255.0) as u8,
        texel[3],
    ]
}

fn wrap01(value: f32) -> f32 {
    if value.is_finite() {
        value.rem_euclid(1.0)
    } else {
        0.0
    }
}

fn lerp(left: f32, right: f32, t: f32) -> f32 {
    left + (right - left) * t
}

fn slot_color(slot_id: &str) -> [u8; 4] {
    let mut hash = 14_695_981_039_346_656_037_u64;
    for byte in slot_id.as_bytes() {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(1_099_511_628_211);
    }
    [
        72 + (hash & 0x7f) as u8,
        72 + ((hash >> 8) & 0x7f) as u8,
        72 + ((hash >> 16) & 0x7f) as u8,
        255,
    ]
}

fn blit(target: &mut RenderedImage, source: &RenderedImage, origin_x: u32, origin_y: u32) {
    for y in 0..source.height.min(target.height.saturating_sub(origin_y)) {
        for x in 0..source.width.min(target.width.saturating_sub(origin_x)) {
            let source_index = ((y as usize) * (source.width as usize) + (x as usize)) * 4;
            let target_index = (((origin_y + y) as usize) * (target.width as usize)
                + ((origin_x + x) as usize))
                * 4;
            target.rgba8[target_index..target_index + 4]
                .copy_from_slice(&source.rgba8[source_index..source_index + 4]);
        }
    }
}

#[cfg(test)]
mod tests {
    use glam::Vec3;
    use shape_core::Aabb;

    use super::*;

    #[test]
    fn surface_preview_rejects_missing_texture() {
        let request = request_with_texture(Vec::new());
        let report = validate_surface_preview_request(&request);

        assert!(!report.valid);
        assert!(
            report
                .issue_codes
                .contains(&"missing_base_color_texture".to_owned())
        );
    }

    #[test]
    fn surface_preview_renders_textured_pixels_deterministically() {
        let request = request_with_texture(checker_texture("paint"));

        let first = render_surface_preview(&request).expect("first preview");
        let second = render_surface_preview(&request).expect("second preview");

        assert!(first.report.valid, "{:?}", first.report);
        assert!(first.report.visible_surface_pixel_count > 0);
        assert_eq!(first.image.rgba8, second.image.rgba8);
        assert_eq!(first.report.sampling, TextureSampling::Bilinear);
    }

    #[test]
    fn nearest_and_bilinear_sampling_are_distinct() {
        let mut nearest = request_with_texture(checker_texture("paint"));
        nearest.sampling = TextureSampling::Nearest;
        let mut bilinear = nearest.clone();
        bilinear.sampling = TextureSampling::Bilinear;

        assert_ne!(
            sample_texture(
                &nearest.textures[0],
                Vec2::new(0.51, 0.51),
                nearest.sampling
            ),
            sample_texture(
                &bilinear.textures[0],
                Vec2::new(0.51, 0.51),
                bilinear.sampling
            )
        );
    }

    #[test]
    fn material_slot_overlay_uses_slot_color_payloads() {
        let request = request_with_texture(checker_texture("paint"));

        let overlay = render_material_slot_overlay(&request).expect("overlay");

        assert!(overlay.report.visible_surface_pixel_count > 0);
    }

    fn request_with_texture(textures: Vec<SurfacePreviewTexture>) -> SurfacePreviewRequest {
        let mesh = TriangleMesh {
            positions: vec![[-0.8, -0.8, 0.0], [0.8, -0.8, 0.0], [0.0, 0.8, 0.0]],
            normals: vec![[0.0, 0.0, 1.0]; 3],
            indices: vec![0, 1, 2],
            bounds: Aabb {
                min: Vec3::new(-0.8, -0.8, 0.0),
                max: Vec3::new(0.8, 0.8, 0.0),
            },
        };
        SurfacePreviewRequest {
            mesh,
            texcoord0: vec![[0.0, 0.0], [1.0, 0.0], [0.5, 1.0]],
            material_bindings: vec![SurfacePreviewMaterialBinding {
                slot_id: "body".to_owned(),
                display_name: "Body".to_owned(),
                material_id: "paint".to_owned(),
            }],
            triangle_bindings: vec![SurfacePreviewTriangleBinding {
                triangle_index: 0,
                material_slot_id: "body".to_owned(),
            }],
            textures,
            camera: OrbitCamera {
                target: Vec3::ZERO,
                yaw_degrees: 0.0,
                pitch_degrees: 0.0,
                distance: 3.0,
                vertical_fov_degrees: 45.0,
            },
            render_settings: RenderSettings {
                width: 64,
                height: 64,
                background: [1, 2, 3, 255],
                ambient: 1.0,
                light_direction: Vec3::new(0.0, -1.0, -1.0).normalize(),
                wireframe: false,
            },
            sampling: TextureSampling::Bilinear,
        }
    }

    fn checker_texture(material_id: &str) -> Vec<SurfacePreviewTexture> {
        vec![SurfacePreviewTexture {
            material_id: material_id.to_owned(),
            channel: SurfacePreviewTextureChannel::BaseColor,
            width: 2,
            height: 2,
            rgba8: vec![
                255, 0, 0, 255, 0, 255, 0, 255, 0, 0, 255, 255, 255, 255, 0, 255,
            ],
        }]
    }
}
