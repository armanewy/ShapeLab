//! CPU-rendered whole-model previews for foundry comparison surfaces.

use std::cmp::Ordering;
use std::collections::{BTreeMap, VecDeque};
use std::thread;

use glam::{Vec3, Vec4};
use serde::{Deserialize, Serialize};
use shape_core::Aabb;
use shape_foundry::{
    CandidateLegibilityClass, FoundryPreviewDisplayMode, SemanticClayRoleAssignment,
    VariationChannel, VariationScope,
};
use shape_mesh::TriangleMesh;
use thiserror::Error;

use crate::{
    OrbitCamera, RenderError, RenderSettings, RenderedImage, clay_readability_render_settings,
    fit_camera_to_bounds_with_aspect, render_mesh,
};

const OVERLAY_HASH_OFFSET: u64 = 14_695_981_039_346_656_037;
const OVERLAY_HASH_PRIME: u64 = 1_099_511_628_211;
const MIN_NORMAL_LENGTH_SQUARED: f32 = 1.0e-12;
const RENDER_DUPLICATE_AVERAGE_DELTA: f32 = 0.018;
const RENDER_DUPLICATE_MAX_DELTA: f32 = 0.035;
const RENDER_CLEAR_AVERAGE_DELTA: f32 = 0.075;
const RENDER_CLEAR_MAX_DELTA: f32 = 0.115;
const RENDER_STRONG_AVERAGE_DELTA: f32 = 0.16;
const RENDER_STRONG_MAX_DELTA: f32 = 0.24;

/// Default bounded capacity for the in-memory Foundry preview cache.
pub const FOUNDRY_DEFAULT_PREVIEW_CACHE_CAPACITY: usize = 64;

/// Supported square foundry preview sizes.
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum FoundryPreviewResolution {
    /// 64x64 preview.
    Px64,
    /// 96x96 preview.
    Px96,
    /// 128x128 preview.
    Px128,
    /// 256x256 preview.
    Px256,
    /// 512x512 preview.
    Px512,
    /// 1024x1024 preview.
    Px1024,
}

impl FoundryPreviewResolution {
    /// Return the square pixel dimension for this preview size.
    #[must_use]
    pub const fn pixels(self) -> u32 {
        match self {
            Self::Px64 => 64,
            Self::Px96 => 96,
            Self::Px128 => 128,
            Self::Px256 => 256,
            Self::Px512 => 512,
            Self::Px1024 => 1024,
        }
    }

    /// Convert a pixel size into a supported preview resolution.
    #[must_use]
    pub const fn from_pixels(pixels: u32) -> Option<Self> {
        match pixels {
            64 => Some(Self::Px64),
            96 => Some(Self::Px96),
            128 => Some(Self::Px128),
            256 => Some(Self::Px256),
            512 => Some(Self::Px512),
            1024 => Some(Self::Px1024),
            _ => None,
        }
    }
}

/// Canonical sampled value for a foundry preview key.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum FoundryPreviewControlValue {
    /// Floating-point scalar.
    Scalar(f32),
    /// Integer value.
    Integer(i64),
    /// Boolean value.
    Toggle(bool),
    /// Symbolic choice.
    Choice(String),
    /// Provider ID.
    Provider(String),
}

impl PartialEq for FoundryPreviewControlValue {
    fn eq(&self, other: &Self) -> bool {
        self.cmp(other) == Ordering::Equal
    }
}

impl Eq for FoundryPreviewControlValue {}

impl PartialOrd for FoundryPreviewControlValue {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for FoundryPreviewControlValue {
    fn cmp(&self, other: &Self) -> Ordering {
        control_value_rank(self)
            .cmp(&control_value_rank(other))
            .then_with(|| match (self, other) {
                (Self::Scalar(left), Self::Scalar(right)) => {
                    canonical_f32_bits(*left).cmp(&canonical_f32_bits(*right))
                }
                (Self::Integer(left), Self::Integer(right)) => left.cmp(right),
                (Self::Toggle(left), Self::Toggle(right)) => left.cmp(right),
                (Self::Choice(left), Self::Choice(right))
                | (Self::Provider(left), Self::Provider(right)) => left.cmp(right),
                _ => Ordering::Equal,
            })
    }
}

/// Normalized camera fields used by foundry preview cache keys.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct FoundryPreviewCameraKey {
    /// Clamped camera target components as canonical f32 bits.
    pub target: [u32; 3],
    /// Clamped yaw as canonical f32 bits.
    pub yaw_degrees: u32,
    /// Clamped pitch as canonical f32 bits.
    pub pitch_degrees: u32,
    /// Clamped distance as canonical f32 bits.
    pub distance: u32,
    /// Clamped vertical field of view as canonical f32 bits.
    pub vertical_fov_degrees: u32,
}

impl FoundryPreviewCameraKey {
    /// Build a cache-key camera payload from a render camera.
    #[must_use]
    pub fn from_camera(camera: &OrbitCamera) -> Self {
        let camera = camera.clamped();
        Self {
            target: vec3_key(camera.target),
            yaw_degrees: canonical_f32_bits(camera.yaw_degrees),
            pitch_degrees: canonical_f32_bits(camera.pitch_degrees),
            distance: canonical_f32_bits(camera.distance),
            vertical_fov_degrees: canonical_f32_bits(camera.vertical_fov_degrees),
        }
    }
}

/// Normalized render settings used by foundry preview cache keys.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct FoundryPreviewRenderSettingsKey {
    /// Output width in pixels.
    pub width: u32,
    /// Output height in pixels.
    pub height: u32,
    /// Background RGBA.
    pub background: [u8; 4],
    /// Ambient light as canonical f32 bits.
    pub ambient: u32,
    /// Render-equivalent light direction as canonical f32 bits.
    pub light_direction: [u32; 3],
    /// Whether wireframe overlay is enabled.
    pub wireframe: bool,
    /// Whether the display-only edge outline is enabled.
    pub edge_outline: bool,
}

impl FoundryPreviewRenderSettingsKey {
    /// Build a cache-key settings payload from render settings.
    #[must_use]
    pub fn from_settings(settings: &RenderSettings) -> Self {
        Self {
            width: settings.width,
            height: settings.height,
            background: settings.background,
            ambient: canonical_f32_bits(settings.ambient.clamp(0.0, 1.0)),
            light_direction: vec3_key(normalized_light_direction(settings.light_direction)),
            wireframe: settings.wireframe,
            edge_outline: settings.edge_outline,
        }
    }
}

/// Complete key for a foundry preview cache entry.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct FoundryPreviewCacheKey {
    /// Fingerprint for the document geometry that produced this complete model.
    pub document_geometry_fingerprint: String,
    /// Sampled whole-model control state.
    pub sampled_control_state: BTreeMap<String, FoundryPreviewControlValue>,
    /// Effective provider choices keyed by family role.
    pub provider_choices: BTreeMap<String, String>,
    /// Camera used for this comparison set.
    pub camera: FoundryPreviewCameraKey,
    /// Render settings after preview resolution is applied.
    pub render_settings: FoundryPreviewRenderSettingsKey,
    /// Requested preview resolution.
    pub resolution: FoundryPreviewResolution,
    /// Preview display mode.
    pub display_mode: FoundryPreviewDisplayMode,
    /// Semantic Clay assignments used by the display mode.
    pub semantic_clay_assignments: Vec<FoundrySemanticClayAssignmentKey>,
}

impl FoundryPreviewCacheKey {
    /// Build a cache key from all render-relevant foundry preview inputs.
    #[must_use]
    fn from_item(item: &FoundryPreviewRequest, render_context: &PreviewRenderContext<'_>) -> Self {
        Self {
            document_geometry_fingerprint: item.document_geometry_fingerprint.clone(),
            sampled_control_state: item.sampled_control_state.clone(),
            provider_choices: item.provider_choices.clone(),
            camera: FoundryPreviewCameraKey::from_camera(render_context.camera),
            render_settings: FoundryPreviewRenderSettingsKey::from_settings(
                render_context.render_settings,
            ),
            resolution: render_context.resolution,
            display_mode: item.display_mode,
            semantic_clay_assignments: item
                .semantic_clay_assignments
                .iter()
                .map(FoundrySemanticClayAssignmentKey::from_assignment)
                .collect(),
        }
    }
}

/// Cache-key-safe Semantic Clay assignment.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct FoundrySemanticClayAssignmentKey {
    /// Family role or semantic part-group ID.
    pub role_or_part_group: String,
    /// Product-safe display label.
    pub display_label: String,
    /// Canonical neutral gray bits.
    pub neutral_gray_value: u32,
    /// Higher priority wins when assignments overlap.
    pub priority: u8,
    /// Whether the assignment applies to generated candidates.
    pub applies_to_candidates: bool,
}

impl FoundrySemanticClayAssignmentKey {
    fn from_assignment(assignment: &SemanticClayRoleAssignment) -> Self {
        Self {
            role_or_part_group: assignment.role_or_part_group.clone(),
            display_label: assignment.display_label.clone(),
            neutral_gray_value: canonical_f32_bits(assignment.neutral_gray_value.clamp(0.0, 1.0)),
            priority: assignment.priority,
            applies_to_candidates: assignment.applies_to_candidates,
        }
    }
}

/// Foundry surface requesting a whole-model preview.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum FoundryPreviewKind {
    /// Candidate card in a whole-model candidate comparison.
    CandidateCard {
        /// Candidate ID.
        candidate_id: String,
    },
    /// Continuous slider filmstrip sample.
    SliderFilmstrip {
        /// Control ID.
        control_id: String,
        /// Stable sample index in the filmstrip.
        sample_index: u32,
    },
    /// Integer, toggle, or symbolic strip sample.
    DiscreteStrip {
        /// Control ID.
        control_id: String,
        /// Stable value index in the strip.
        value_index: u32,
    },
    /// Provider-gallery option preview.
    ProviderGallery {
        /// Family role.
        role: String,
        /// Provider ID.
        provider_id: String,
        /// Stable option index in the gallery.
        option_index: u32,
    },
    /// Preview carrying changed-role overlay metadata.
    ChangedRoleOverlay {
        /// Family role being annotated.
        role: String,
    },
}

/// Metadata for a changed role drawn over a complete-model preview.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FoundryChangedRoleOverlay {
    /// Family role.
    pub role: String,
    /// Provider before the change, when known.
    pub previous_provider: Option<String>,
    /// Provider after the change, when known.
    pub current_provider: Option<String>,
    /// Controls responsible for the role change.
    pub changed_controls: Vec<String>,
}

/// Product-safe variation metadata carried with a preview for future overlays.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FoundryPreviewVariationMetadata {
    /// Scope represented by this preview.
    #[serde(default)]
    pub scope: VariationScope,
    /// Channels represented by this preview.
    #[serde(default)]
    pub channels: Vec<VariationChannel>,
    /// Selected part-group ID, when a Focus Part preview is requested.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub selected_part_group: Option<String>,
    /// Material slot ID, when a future surface preview is requested.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub material_slot_id: Option<String>,
    /// Product legibility class assigned before/after preview comparison.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub legibility_class: Option<CandidateLegibilityClass>,
}

impl Default for FoundryPreviewVariationMetadata {
    fn default() -> Self {
        Self {
            scope: VariationScope::WholeAsset,
            channels: vec![VariationChannel::CompleteLook],
            selected_part_group: None,
            material_slot_id: None,
            legibility_class: None,
        }
    }
}

/// Pixel-space visual delta between two rendered foundry previews.
#[derive(Debug, Copy, Clone, PartialEq)]
pub struct FoundryRenderedVisibleDelta {
    /// Mean visible pixel difference in `0..1`.
    pub mean_pixel_delta: f32,
    /// Ratio of visibly changed foreground pixels in `0..1`.
    pub changed_pixel_ratio: f32,
    /// Ratio of silhouette foreground/background changes in `0..1`.
    pub silhouette_delta: f32,
    /// Weighted aggregate score in `0..1`.
    pub score: f32,
    /// Plain reason when the previews could not be compared.
    pub unavailable_reason: Option<&'static str>,
}

/// Strict multi-camera report derived from rendered preview comparisons.
#[derive(Debug, Clone, PartialEq)]
pub struct FoundryRenderedPerceptualReport {
    /// Stable candidate ID.
    pub candidate_id: String,
    /// Per-camera aggregate deltas in `0..1`.
    pub render_delta_by_camera: Vec<f32>,
    /// Maximum camera delta in `0..1`.
    pub max_delta: f32,
    /// Average camera delta in `0..1`.
    pub average_delta: f32,
    /// Maximum silhouette delta in `0..1`.
    pub silhouette_delta: f32,
    /// Product-visible classification.
    pub legibility_class: CandidateLegibilityClass,
    /// Rejection reason when the comparison is not selectable.
    pub reject_reason: Option<String>,
    /// Human-facing diagnostic summary.
    pub human_summary: String,
}

impl FoundryRenderedVisibleDelta {
    /// Return true when this comparison has usable finite evidence.
    #[must_use]
    pub fn available(self) -> bool {
        self.unavailable_reason.is_none()
    }

    fn unavailable(reason: &'static str) -> Self {
        Self {
            mean_pixel_delta: 0.0,
            changed_pixel_ratio: 0.0,
            silhouette_delta: 0.0,
            score: 0.0,
            unavailable_reason: Some(reason),
        }
    }
}

/// Classify rendered parent/candidate comparisons from one or more fixed cameras.
#[must_use]
pub fn classify_foundry_rendered_perceptual_report(
    candidate_id: impl Into<String>,
    camera_pairs: &[(&RenderedImage, &RenderedImage)],
    background: [u8; 4],
) -> FoundryRenderedPerceptualReport {
    let candidate_id = candidate_id.into();
    let deltas = camera_pairs
        .iter()
        .map(|(parent, candidate)| {
            compare_foundry_rendered_visible_delta(parent, candidate, background)
        })
        .collect::<Vec<_>>();
    let render_delta_by_camera = deltas.iter().map(|delta| delta.score).collect::<Vec<_>>();
    let max_delta = render_delta_by_camera.iter().copied().fold(0.0, f32::max);
    let average_delta = if render_delta_by_camera.is_empty() {
        0.0
    } else {
        render_delta_by_camera.iter().sum::<f32>() / render_delta_by_camera.len() as f32
    };
    let silhouette_delta = deltas
        .iter()
        .map(|delta| delta.silhouette_delta)
        .fold(0.0, f32::max);
    let unavailable = deltas.iter().find_map(|delta| delta.unavailable_reason);
    let (legibility_class, reject_reason) = if camera_pairs.len() < 2 {
        (
            CandidateLegibilityClass::Unsupported,
            Some("Multi-camera preview evidence needs at least two fixed views.".to_owned()),
        )
    } else if let Some(reason) = unavailable {
        (
            CandidateLegibilityClass::Unsupported,
            Some(reason.to_owned()),
        )
    } else if average_delta < RENDER_DUPLICATE_AVERAGE_DELTA
        && max_delta < RENDER_DUPLICATE_MAX_DELTA
    {
        (
            CandidateLegibilityClass::DuplicateLooking,
            Some("Candidate looks identical to the parent at preview size.".to_owned()),
        )
    } else if average_delta >= RENDER_STRONG_AVERAGE_DELTA || max_delta >= RENDER_STRONG_MAX_DELTA {
        (CandidateLegibilityClass::Strong, None)
    } else if average_delta >= RENDER_CLEAR_AVERAGE_DELTA || max_delta >= RENDER_CLEAR_MAX_DELTA {
        (CandidateLegibilityClass::Clear, None)
    } else {
        (
            CandidateLegibilityClass::TooSubtle,
            Some("Visible change is too small for a normal direction card.".to_owned()),
        )
    };
    let human_summary = if legibility_class.selectable() {
        format!(
            "{} rendered as {} with {:.0}% average preview delta.",
            candidate_id,
            legibility_class.display_label(),
            average_delta * 100.0
        )
    } else {
        format!(
            "{} rejected: {}",
            candidate_id,
            reject_reason
                .as_deref()
                .unwrap_or("preview comparison is unavailable")
        )
    };
    FoundryRenderedPerceptualReport {
        candidate_id,
        render_delta_by_camera,
        max_delta,
        average_delta,
        silhouette_delta,
        legibility_class,
        reject_reason,
        human_summary,
    }
}

/// Compare two rendered previews while ignoring shared background pixels.
///
/// The comparison is deterministic, clamps every score to `0..1`, rejects
/// mismatched or malformed image buffers, and treats the supplied background as
/// non-evidence unless foreground appears on either side.
#[must_use]
pub fn compare_foundry_rendered_visible_delta(
    parent: &RenderedImage,
    candidate: &RenderedImage,
    background: [u8; 4],
) -> FoundryRenderedVisibleDelta {
    if parent.width != candidate.width || parent.height != candidate.height {
        return FoundryRenderedVisibleDelta::unavailable(
            "Preview sizes do not match for visual comparison.",
        );
    }
    if parent.width == 0 || parent.height == 0 {
        return FoundryRenderedVisibleDelta::unavailable(
            "Preview images are empty and cannot be compared.",
        );
    }
    let expected_len = (parent.width as usize)
        .checked_mul(parent.height as usize)
        .and_then(|pixels| pixels.checked_mul(4));
    if expected_len != Some(parent.rgba8.len()) || expected_len != Some(candidate.rgba8.len()) {
        return FoundryRenderedVisibleDelta::unavailable(
            "Preview pixels are incomplete and cannot be compared.",
        );
    }

    let mut total_delta = 0.0_f32;
    let mut changed_pixels = 0_usize;
    let mut silhouette_pixels = 0_usize;
    let mut evidence_pixels = 0_usize;
    for (left, right) in parent
        .rgba8
        .chunks_exact(4)
        .zip(candidate.rgba8.chunks_exact(4))
    {
        let left_foreground = pixel_is_foreground(left, background);
        let right_foreground = pixel_is_foreground(right, background);
        if !left_foreground && !right_foreground {
            continue;
        }
        evidence_pixels += 1;
        let left_alpha = left[3] as f32 / 255.0;
        let right_alpha = right[3] as f32 / 255.0;
        let alpha_delta = (left_alpha - right_alpha).abs();
        let visible_weight = left_alpha.max(right_alpha);
        let color_delta = ((left[0].abs_diff(right[0]) as f32
            + left[1].abs_diff(right[1]) as f32
            + left[2].abs_diff(right[2]) as f32)
            / (3.0 * 255.0))
            * visible_weight;
        let pixel_delta = color_delta.max(alpha_delta).clamp(0.0, 1.0);
        total_delta += pixel_delta;
        if pixel_delta >= 0.08 {
            changed_pixels += 1;
        }
        if left_foreground != right_foreground {
            silhouette_pixels += 1;
        }
    }
    if evidence_pixels == 0 {
        return FoundryRenderedVisibleDelta::unavailable(
            "Preview comparison has no visible foreground pixels.",
        );
    }
    let evidence_pixels = evidence_pixels as f32;
    let mean_pixel_delta = clamp_visible_score(total_delta / evidence_pixels);
    let changed_pixel_ratio = clamp_visible_score(changed_pixels as f32 / evidence_pixels);
    let silhouette_delta = clamp_visible_score(silhouette_pixels as f32 / evidence_pixels);
    let score = clamp_visible_score(
        mean_pixel_delta * 0.55 + changed_pixel_ratio * 0.35 + silhouette_delta * 0.75,
    );
    FoundryRenderedVisibleDelta {
        mean_pixel_delta,
        changed_pixel_ratio,
        silhouette_delta,
        score,
        unavailable_reason: None,
    }
}

fn pixel_is_foreground(pixel: &[u8], background: [u8; 4]) -> bool {
    pixel[3] > 5 && pixel != background
}

fn clamp_visible_score(value: f32) -> f32 {
    if value.is_finite() {
        value.clamp(0.0, 1.0)
    } else {
        0.0
    }
}

/// One complete-model preview request.
#[derive(Debug, Clone, PartialEq)]
pub struct FoundryPreviewRequest {
    /// Stable preview ID from the calling foundry surface.
    pub preview_id: String,
    /// Surface requesting the preview.
    pub kind: FoundryPreviewKind,
    /// Geometry fingerprint for the complete model.
    pub document_geometry_fingerprint: String,
    /// Complete model mesh to render.
    pub mesh: TriangleMesh,
    /// Sampled control state represented by this preview.
    pub sampled_control_state: BTreeMap<String, FoundryPreviewControlValue>,
    /// Effective provider choices represented by this preview.
    pub provider_choices: BTreeMap<String, String>,
    /// Changed-role overlays to draw above this preview.
    pub changed_role_overlays: Vec<FoundryChangedRoleOverlay>,
    /// Display mode used for this untextured clay preview.
    pub display_mode: FoundryPreviewDisplayMode,
    /// Preview-only neutral gray assignments for Semantic Clay.
    pub semantic_clay_assignments: Vec<SemanticClayRoleAssignment>,
    /// Product-safe variation metadata for future overlays.
    pub variation_metadata: FoundryPreviewVariationMetadata,
}

impl FoundryPreviewRequest {
    /// Create a complete-model preview request with empty state metadata.
    #[must_use]
    pub fn new(
        preview_id: impl Into<String>,
        kind: FoundryPreviewKind,
        document_geometry_fingerprint: impl Into<String>,
        mesh: TriangleMesh,
    ) -> Self {
        Self {
            preview_id: preview_id.into(),
            kind,
            document_geometry_fingerprint: document_geometry_fingerprint.into(),
            mesh,
            sampled_control_state: BTreeMap::new(),
            provider_choices: BTreeMap::new(),
            changed_role_overlays: Vec::new(),
            display_mode: FoundryPreviewDisplayMode::PureClay,
            semantic_clay_assignments: Vec::new(),
            variation_metadata: FoundryPreviewVariationMetadata::default(),
        }
    }

    /// Default to Semantic Clay when assignments exist, otherwise Pure Clay.
    pub fn use_novice_default_display_mode(&mut self) {
        self.display_mode =
            FoundryPreviewDisplayMode::novice_default(&self.semantic_clay_assignments);
    }
}

/// Batch of foundry previews that must share camera framing.
#[derive(Debug, Clone, PartialEq)]
pub struct FoundryPreviewBatchRequest {
    /// Preview comparison-set ID used for caller bookkeeping.
    pub comparison_set_id: String,
    /// Complete-model preview requests in deterministic output order.
    pub items: Vec<FoundryPreviewRequest>,
    /// Optional explicit camera. When absent, one camera is fit to every item.
    pub camera: Option<OrbitCamera>,
    /// Base render settings. Width and height are replaced by `resolution`.
    pub render_settings: RenderSettings,
    /// Square preview resolution.
    pub resolution: FoundryPreviewResolution,
    /// Maximum number of render jobs to run at once. Zero is treated as one.
    pub max_parallel_jobs: usize,
}

impl FoundryPreviewBatchRequest {
    /// Create a preview batch using default render settings.
    #[must_use]
    pub fn new(
        comparison_set_id: impl Into<String>,
        items: Vec<FoundryPreviewRequest>,
        resolution: FoundryPreviewResolution,
    ) -> Self {
        Self {
            comparison_set_id: comparison_set_id.into(),
            items,
            camera: None,
            render_settings: clay_readability_render_settings(
                resolution.pixels(),
                resolution.pixels(),
            ),
            resolution,
            max_parallel_jobs: 1,
        }
    }
}

/// Whether a preview image came from cache or a fresh CPU render.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum FoundryPreviewCacheStatus {
    /// Image was reused from the in-memory preview cache.
    Hit,
    /// Image was freshly CPU-rendered.
    Miss,
}

/// Rendered foundry whole-model preview with metadata for the requesting surface.
#[derive(Debug, Clone, PartialEq)]
pub struct FoundryRenderedPreview {
    /// Stable preview ID from the request.
    pub preview_id: String,
    /// Surface that requested the preview.
    pub kind: FoundryPreviewKind,
    /// Cache key used for lookup and storage.
    pub key: FoundryPreviewCacheKey,
    /// Rendered RGBA8 image.
    pub image: RenderedImage,
    /// Cache status for this preview.
    pub cache_status: FoundryPreviewCacheStatus,
    /// Camera shared by this comparison set.
    pub camera: OrbitCamera,
    /// Render settings with preview resolution applied.
    pub render_settings: RenderSettings,
    /// Requested preview resolution.
    pub resolution: FoundryPreviewResolution,
    /// Bounds for this complete model mesh.
    pub whole_model_bounds: Aabb,
    /// Union bounds used to fit the shared comparison camera.
    pub comparison_bounds: Aabb,
    /// Changed-role overlay metadata to paint over this preview.
    pub changed_role_overlays: Vec<FoundryChangedRoleOverlay>,
    /// Product-safe variation metadata copied from the request.
    pub variation_metadata: FoundryPreviewVariationMetadata,
}

/// Rendered output for a foundry preview comparison set.
#[derive(Debug, Clone, PartialEq)]
pub struct FoundryPreviewBatchOutput {
    /// Preview comparison-set ID copied from the request.
    pub comparison_set_id: String,
    /// Camera shared by all previews in the comparison set.
    pub camera: OrbitCamera,
    /// Render settings with preview resolution applied.
    pub render_settings: RenderSettings,
    /// Union bounds used to fit the camera.
    pub comparison_bounds: Aabb,
    /// Rendered previews in the same order as the input requests.
    pub previews: Vec<FoundryRenderedPreview>,
}

/// Cache counters and capacity information.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct FoundryPreviewCacheStats {
    /// Current number of cached images.
    pub len: usize,
    /// Maximum number of cached images.
    pub capacity: usize,
    /// Cache hits observed through batch rendering.
    pub hits: u64,
    /// Cache misses observed through batch rendering.
    pub misses: u64,
    /// Entries evicted by LRU pressure.
    pub evictions: u64,
    /// Duplicate cache misses coalesced into an already pending render in the
    /// same batch.
    pub coalesced_misses: u64,
}

/// Bounded in-memory LRU cache for whole-model foundry previews.
#[derive(Debug, Clone)]
pub struct FoundryPreviewCache {
    capacity: usize,
    entries: BTreeMap<FoundryPreviewCacheKey, RenderedImage>,
    lru_order: VecDeque<FoundryPreviewCacheKey>,
    hits: u64,
    misses: u64,
    evictions: u64,
    coalesced_misses: u64,
}

impl FoundryPreviewCache {
    /// Create an empty preview cache with the given maximum entry count.
    #[must_use]
    pub fn new(capacity: usize) -> Self {
        Self {
            capacity,
            entries: BTreeMap::new(),
            lru_order: VecDeque::new(),
            hits: 0,
            misses: 0,
            evictions: 0,
            coalesced_misses: 0,
        }
    }

    /// Return the number of cached images.
    #[must_use]
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Return true when no images are cached.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Return true when the key is currently cached.
    #[must_use]
    pub fn contains_key(&self, key: &FoundryPreviewCacheKey) -> bool {
        self.entries.contains_key(key)
    }

    /// Return cache counters and capacity information.
    #[must_use]
    pub fn stats(&self) -> FoundryPreviewCacheStats {
        FoundryPreviewCacheStats {
            len: self.len(),
            capacity: self.capacity,
            hits: self.hits,
            misses: self.misses,
            evictions: self.evictions,
            coalesced_misses: self.coalesced_misses,
        }
    }

    /// Remove all cached images and reset LRU order. Counters are retained.
    pub fn clear(&mut self) {
        self.entries.clear();
        self.lru_order.clear();
    }

    /// Render a complete-model preview comparison set.
    pub fn render_batch(
        &mut self,
        request: FoundryPreviewBatchRequest,
    ) -> Result<FoundryPreviewBatchOutput, FoundryPreviewError> {
        if request.items.is_empty() {
            return Err(FoundryPreviewError::EmptyBatch);
        }

        validate_preview_ids(&request.items)?;

        let render_settings = settings_for_resolution(&request.render_settings, request.resolution);
        let comparison_bounds = comparison_bounds(&request.items);
        let aspect_ratio = render_settings.width as f32 / render_settings.height as f32;
        let first_preview_id = request.items[0].preview_id.as_str();
        validate_render_settings(&render_settings, first_preview_id)?;
        validate_preview_meshes(&request.items)?;
        let camera = comparison_camera(
            request.camera.as_ref(),
            comparison_bounds,
            aspect_ratio,
            first_preview_id,
        )?;

        let render_context = PreviewRenderContext {
            camera: &camera,
            render_settings: &render_settings,
            resolution: request.resolution,
            comparison_bounds,
        };

        let mut previews = vec![None; request.items.len()];
        let mut render_jobs_by_key = BTreeMap::<FoundryPreviewCacheKey, RenderJob>::new();
        for (index, item) in request.items.iter().enumerate() {
            let key = FoundryPreviewCacheKey::from_item(item, &render_context);
            if let Some(job) = render_jobs_by_key.get_mut(&key) {
                job.indices.push(index);
                self.coalesced_misses = self.coalesced_misses.saturating_add(1);
                continue;
            }

            if let Some(image) = self.lookup(&key) {
                previews[index] = Some(preview_from_image(
                    item,
                    key,
                    image,
                    FoundryPreviewCacheStatus::Hit,
                    &render_context,
                ));
            } else {
                render_jobs_by_key.insert(
                    key.clone(),
                    RenderJob {
                        indices: vec![index],
                        preview_id: item.preview_id.clone(),
                        key,
                        mesh: item.mesh.clone(),
                        display_mode: item.display_mode,
                        semantic_clay_assignments: item.semantic_clay_assignments.clone(),
                    },
                );
            }
        }

        let rendered_jobs = render_missing_jobs(
            render_jobs_by_key.into_values().collect(),
            &camera,
            &render_settings,
            request.max_parallel_jobs,
        )?;
        for rendered in rendered_jobs {
            self.insert(rendered.key.clone(), rendered.image.clone());
            for index in rendered.indices {
                let item = &request.items[index];
                previews[index] = Some(preview_from_image(
                    item,
                    rendered.key.clone(),
                    rendered.image.clone(),
                    FoundryPreviewCacheStatus::Miss,
                    &render_context,
                ));
            }
        }

        Ok(FoundryPreviewBatchOutput {
            comparison_set_id: request.comparison_set_id,
            camera,
            render_settings,
            comparison_bounds,
            previews: previews
                .into_iter()
                .map(|preview| preview.expect("every preview should be rendered or cached"))
                .collect(),
        })
    }

    fn lookup(&mut self, key: &FoundryPreviewCacheKey) -> Option<RenderedImage> {
        let image = self.entries.get(key).cloned();
        if image.is_some() {
            self.hits = self.hits.saturating_add(1);
            self.touch(key);
        } else {
            self.misses = self.misses.saturating_add(1);
        }
        image
    }

    fn insert(&mut self, key: FoundryPreviewCacheKey, image: RenderedImage) {
        if self.capacity == 0 {
            return;
        }

        self.entries.insert(key.clone(), image);
        self.touch(&key);

        while self.entries.len() > self.capacity {
            let Some(evicted_key) = self.lru_order.pop_front() else {
                break;
            };
            if self.entries.remove(&evicted_key).is_some() {
                self.evictions = self.evictions.saturating_add(1);
            }
        }
    }

    fn touch(&mut self, key: &FoundryPreviewCacheKey) {
        if let Some(position) = self
            .lru_order
            .iter()
            .position(|ordered_key| ordered_key == key)
        {
            self.lru_order.remove(position);
        }
        self.lru_order.push_back(key.clone());
    }
}

impl Default for FoundryPreviewCache {
    fn default() -> Self {
        Self::new(FOUNDRY_DEFAULT_PREVIEW_CACHE_CAPACITY)
    }
}

/// Render a complete-model foundry comparison set through a preview cache.
pub fn render_foundry_previews(
    cache: &mut FoundryPreviewCache,
    request: FoundryPreviewBatchRequest,
) -> Result<FoundryPreviewBatchOutput, FoundryPreviewError> {
    cache.render_batch(request)
}

/// Foundry preview rendering errors.
#[derive(Debug, Clone, Error, PartialEq, Eq)]
pub enum FoundryPreviewError {
    /// Batch had no items.
    #[error("foundry preview batch cannot be empty")]
    EmptyBatch,
    /// A preview had an empty preview ID.
    #[error("foundry preview request at index {index} has an empty preview ID")]
    EmptyPreviewId {
        /// Request index.
        index: usize,
    },
    /// A preview had an empty geometry fingerprint.
    #[error("foundry preview request `{preview_id}` has an empty geometry fingerprint")]
    EmptyGeometryFingerprint {
        /// Preview ID.
        preview_id: String,
    },
    /// A worker thread panicked while rendering.
    #[error("foundry preview render worker panicked")]
    RenderWorkerPanicked,
    /// Rendering one preview failed.
    #[error("rendering foundry preview `{preview_id}` failed: {source}")]
    Render {
        /// Preview ID.
        preview_id: String,
        /// Render failure.
        #[source]
        source: RenderError,
    },
}

#[derive(Debug)]
struct RenderJob {
    indices: Vec<usize>,
    preview_id: String,
    key: FoundryPreviewCacheKey,
    mesh: TriangleMesh,
    display_mode: FoundryPreviewDisplayMode,
    semantic_clay_assignments: Vec<SemanticClayRoleAssignment>,
}

#[derive(Debug)]
struct RenderJobOutput {
    indices: Vec<usize>,
    key: FoundryPreviewCacheKey,
    image: RenderedImage,
}

#[derive(Debug, Copy, Clone)]
struct PreviewRenderContext<'a> {
    camera: &'a OrbitCamera,
    render_settings: &'a RenderSettings,
    resolution: FoundryPreviewResolution,
    comparison_bounds: Aabb,
}

fn render_missing_jobs(
    jobs: Vec<RenderJob>,
    camera: &OrbitCamera,
    render_settings: &RenderSettings,
    max_parallel_jobs: usize,
) -> Result<Vec<RenderJobOutput>, FoundryPreviewError> {
    let parallelism = max_parallel_jobs.max(1);
    let mut outputs = Vec::with_capacity(jobs.len());
    for chunk in jobs.chunks(parallelism) {
        let mut chunk_outputs = thread::scope(|scope| {
            let handles = chunk
                .iter()
                .map(|job| scope.spawn(move || render_mesh(&job.mesh, camera, render_settings)))
                .collect::<Vec<_>>();

            let mut rendered = Vec::with_capacity(handles.len());
            for (job, handle) in chunk.iter().zip(handles) {
                let mut image = handle
                    .join()
                    .map_err(|_| FoundryPreviewError::RenderWorkerPanicked)?
                    .map_err(|source| FoundryPreviewError::Render {
                        preview_id: job.preview_id.clone(),
                        source,
                    })?;
                apply_display_mode(
                    &mut image,
                    job.display_mode,
                    &job.semantic_clay_assignments,
                    render_settings.background,
                );
                rendered.push(RenderJobOutput {
                    indices: job.indices.clone(),
                    key: job.key.clone(),
                    image,
                });
            }
            Ok(rendered)
        })?;
        outputs.append(&mut chunk_outputs);
    }
    outputs.sort_by_key(|output| output.indices[0]);
    Ok(outputs)
}

fn apply_display_mode(
    image: &mut RenderedImage,
    mode: FoundryPreviewDisplayMode,
    assignments: &[SemanticClayRoleAssignment],
    background: [u8; 4],
) {
    match mode {
        FoundryPreviewDisplayMode::PureClay => {
            apply_gray_bands(image, &[0.68], background);
        }
        FoundryPreviewDisplayMode::SemanticClay => {
            if assignments.is_empty() {
                apply_gray_bands(image, &[0.68], background);
            } else {
                let grays = assignments
                    .iter()
                    .map(|assignment| assignment.neutral_gray_value.clamp(0.0, 1.0))
                    .collect::<Vec<_>>();
                apply_gray_bands(image, &grays, background);
            }
        }
        FoundryPreviewDisplayMode::DiagnosticPartColor => {
            apply_diagnostic_bands(image, background);
        }
    }
}

fn apply_gray_bands(image: &mut RenderedImage, grays: &[f32], background: [u8; 4]) {
    if image.width == 0 || image.height == 0 || grays.is_empty() {
        return;
    }
    let width = image.width as usize;
    let band_count = grays.len().max(1);
    for y in 0..image.height as usize {
        for x in 0..width {
            let byte_index = (y * width + x) * 4;
            let Some(pixel) = image.rgba8.get_mut(byte_index..byte_index + 4) else {
                continue;
            };
            if !pixel_is_foreground(pixel, background) {
                continue;
            }
            let band_index = (x * band_count / width).min(band_count - 1);
            let gray = (grays[band_index].clamp(0.0, 1.0) * 255.0).round() as u8;
            pixel[0] = gray;
            pixel[1] = gray;
            pixel[2] = gray;
        }
    }
}

fn apply_diagnostic_bands(image: &mut RenderedImage, background: [u8; 4]) {
    const COLORS: [[u8; 3]; 6] = [
        [240, 64, 64],
        [64, 200, 80],
        [64, 128, 240],
        [240, 200, 64],
        [200, 64, 240],
        [64, 220, 220],
    ];
    if image.width == 0 || image.height == 0 {
        return;
    }
    let width = image.width as usize;
    for y in 0..image.height as usize {
        for x in 0..width {
            let byte_index = (y * width + x) * 4;
            let Some(pixel) = image.rgba8.get_mut(byte_index..byte_index + 4) else {
                continue;
            };
            if !pixel_is_foreground(pixel, background) {
                continue;
            }
            let color = COLORS[(x * COLORS.len() / width).min(COLORS.len() - 1)];
            pixel[0] = color[0];
            pixel[1] = color[1];
            pixel[2] = color[2];
        }
    }
}

fn preview_from_image(
    item: &FoundryPreviewRequest,
    key: FoundryPreviewCacheKey,
    mut image: RenderedImage,
    cache_status: FoundryPreviewCacheStatus,
    context: &PreviewRenderContext<'_>,
) -> FoundryRenderedPreview {
    apply_changed_role_overlays(&mut image, &item.changed_role_overlays);
    FoundryRenderedPreview {
        preview_id: item.preview_id.clone(),
        kind: item.kind.clone(),
        key,
        image,
        cache_status,
        camera: context.camera.clone(),
        render_settings: context.render_settings.clone(),
        resolution: context.resolution,
        whole_model_bounds: item.mesh.bounds,
        comparison_bounds: context.comparison_bounds,
        changed_role_overlays: item.changed_role_overlays.clone(),
        variation_metadata: item.variation_metadata.clone(),
    }
}

fn apply_changed_role_overlays(image: &mut RenderedImage, overlays: &[FoundryChangedRoleOverlay]) {
    if overlays.is_empty() || image.width == 0 || image.height == 0 {
        return;
    }

    let width = image.width as usize;
    let height = image.height as usize;
    let thickness = (image.width.min(image.height) / 24).clamp(2, 6) as usize;
    let color = overlay_color(overlays);

    for y in 0..height {
        for x in 0..width {
            if x < thickness
                || y < thickness
                || x >= width.saturating_sub(thickness)
                || y >= height.saturating_sub(thickness)
            {
                set_pixel(image, width, x, y, color);
            }
        }
    }

    let segment_width = (width / overlays.len().max(1)).max(thickness);
    for (index, overlay) in overlays.iter().enumerate() {
        let tick_color = overlay_color(std::slice::from_ref(overlay));
        let start = index * segment_width;
        let end = (start + segment_width).min(width);
        for y in thickness..(thickness * 3).min(height) {
            for x in start..end {
                set_pixel(image, width, x, y, tick_color);
            }
        }
    }
}

fn set_pixel(image: &mut RenderedImage, width: usize, x: usize, y: usize, color: [u8; 4]) {
    let Some(byte_index) = y
        .checked_mul(width)
        .and_then(|pixel| pixel.checked_add(x))
        .and_then(|pixel| pixel.checked_mul(4))
    else {
        return;
    };
    let Some(end) = byte_index.checked_add(4) else {
        return;
    };
    let Some(pixel) = image.rgba8.get_mut(byte_index..end) else {
        return;
    };
    pixel.copy_from_slice(&color);
}

fn overlay_color(overlays: &[FoundryChangedRoleOverlay]) -> [u8; 4] {
    let mut hasher = OverlayHasher::new();
    hasher.write_u64(overlays.len() as u64);
    for overlay in overlays {
        hasher.write_str(&overlay.role);
        hasher.write_optional_str(overlay.previous_provider.as_deref());
        hasher.write_optional_str(overlay.current_provider.as_deref());
        hasher.write_u64(overlay.changed_controls.len() as u64);
        for control in &overlay.changed_controls {
            hasher.write_str(control);
        }
    }
    let hash = hasher.finish();
    [
        96 + (hash & 0x7f) as u8,
        96 + ((hash >> 8) & 0x7f) as u8,
        96 + ((hash >> 16) & 0x7f) as u8,
        255,
    ]
}

fn validate_preview_ids(items: &[FoundryPreviewRequest]) -> Result<(), FoundryPreviewError> {
    for (index, item) in items.iter().enumerate() {
        if item.preview_id.trim().is_empty() {
            return Err(FoundryPreviewError::EmptyPreviewId { index });
        }
        if item.document_geometry_fingerprint.trim().is_empty() {
            return Err(FoundryPreviewError::EmptyGeometryFingerprint {
                preview_id: item.preview_id.clone(),
            });
        }
    }
    Ok(())
}

fn validate_render_settings(
    settings: &RenderSettings,
    preview_id: &str,
) -> Result<(), FoundryPreviewError> {
    if !settings.ambient.is_finite() {
        return Err(FoundryPreviewError::Render {
            preview_id: preview_id.to_owned(),
            source: RenderError::InvalidSettings("ambient must be finite"),
        });
    }
    if !is_finite_vec3(settings.light_direction)
        || settings.light_direction.length_squared() <= MIN_NORMAL_LENGTH_SQUARED
    {
        return Err(FoundryPreviewError::Render {
            preview_id: preview_id.to_owned(),
            source: RenderError::InvalidSettings("light direction must be finite and non-zero"),
        });
    }
    Ok(())
}

fn validate_preview_meshes(items: &[FoundryPreviewRequest]) -> Result<(), FoundryPreviewError> {
    for item in items {
        validate_preview_mesh(&item.preview_id, &item.mesh)?;
    }
    Ok(())
}

fn validate_preview_mesh(preview_id: &str, mesh: &TriangleMesh) -> Result<(), FoundryPreviewError> {
    if !mesh.indices.len().is_multiple_of(3) {
        return Err(FoundryPreviewError::Render {
            preview_id: preview_id.to_owned(),
            source: RenderError::InvalidMesh("triangle index count must be divisible by three"),
        });
    }
    if mesh.positions.iter().any(|position| {
        !position[0].is_finite() || !position[1].is_finite() || !position[2].is_finite()
    }) {
        return Err(FoundryPreviewError::Render {
            preview_id: preview_id.to_owned(),
            source: RenderError::InvalidMesh("positions must be finite"),
        });
    }
    let vertex_count = mesh.positions.len();
    if mesh
        .indices
        .iter()
        .any(|index| (*index as usize) >= vertex_count)
    {
        return Err(FoundryPreviewError::Render {
            preview_id: preview_id.to_owned(),
            source: RenderError::InvalidMesh("triangle index out of bounds"),
        });
    }
    Ok(())
}

fn comparison_camera(
    requested: Option<&OrbitCamera>,
    bounds: Aabb,
    aspect_ratio: f32,
    preview_id: &str,
) -> Result<OrbitCamera, FoundryPreviewError> {
    let Some(camera) = requested else {
        return Ok(fit_camera_to_bounds_with_aspect(bounds, aspect_ratio));
    };
    validate_explicit_camera(camera, preview_id)?;
    let mut fitted = camera.clamped();
    if bounds.is_empty() || !is_finite_vec3(bounds.min) || !is_finite_vec3(bounds.max) {
        return Ok(fitted);
    }
    let center = bounds.center();
    if is_finite_vec3(center) {
        fitted.target = center;
    }
    while !camera_contains_bounds(&fitted, bounds, aspect_ratio) && fitted.distance < 1_000_000.0 {
        fitted.distance = (fitted.distance * 1.25).min(1_000_000.0);
    }
    if !camera_contains_bounds(&fitted, bounds, aspect_ratio) {
        let mut fallback = fit_camera_to_bounds_with_aspect(bounds, aspect_ratio);
        fallback.yaw_degrees = fitted.yaw_degrees;
        fallback.pitch_degrees = fitted.pitch_degrees;
        fallback.vertical_fov_degrees = fitted.vertical_fov_degrees;
        fallback.distance = fitted.distance.max(fallback.distance);
        fitted = fallback.clamped();
    }
    Ok(fitted)
}

fn validate_explicit_camera(
    camera: &OrbitCamera,
    preview_id: &str,
) -> Result<(), FoundryPreviewError> {
    if !is_finite_vec3(camera.target) {
        return Err(FoundryPreviewError::Render {
            preview_id: preview_id.to_owned(),
            source: RenderError::InvalidCamera("target must be finite"),
        });
    }
    if !camera.yaw_degrees.is_finite()
        || !camera.pitch_degrees.is_finite()
        || !camera.distance.is_finite()
        || !camera.vertical_fov_degrees.is_finite()
    {
        return Err(FoundryPreviewError::Render {
            preview_id: preview_id.to_owned(),
            source: RenderError::InvalidCamera("orbit values must be finite"),
        });
    }
    Ok(())
}

fn settings_for_resolution(
    settings: &RenderSettings,
    resolution: FoundryPreviewResolution,
) -> RenderSettings {
    let mut render_settings = settings.clone();
    let pixels = resolution.pixels();
    render_settings.width = pixels;
    render_settings.height = pixels;
    render_settings
}

fn comparison_bounds(items: &[FoundryPreviewRequest]) -> Aabb {
    items.iter().fold(Aabb::empty(), |bounds, item| {
        bounds.union(&item.mesh.bounds)
    })
}

fn camera_contains_bounds(camera: &OrbitCamera, bounds: Aabb, aspect_ratio: f32) -> bool {
    let view_projection = camera.view_projection_matrix(aspect_ratio);
    bounds_corners(bounds).into_iter().all(|corner| {
        let clip = view_projection * Vec4::new(corner.x, corner.y, corner.z, 1.0);
        if !clip.w.is_finite() || clip.w <= 0.0 {
            return false;
        }
        let ndc = clip.truncate() / clip.w;
        ndc.x.is_finite()
            && ndc.y.is_finite()
            && ndc.z.is_finite()
            && ndc.x.abs() <= 1.0
            && ndc.y.abs() <= 1.0
            && ndc.z >= -1.0
            && ndc.z <= 1.0
    })
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

fn control_value_rank(value: &FoundryPreviewControlValue) -> u8 {
    match value {
        FoundryPreviewControlValue::Scalar(_) => 0,
        FoundryPreviewControlValue::Integer(_) => 1,
        FoundryPreviewControlValue::Toggle(_) => 2,
        FoundryPreviewControlValue::Choice(_) => 3,
        FoundryPreviewControlValue::Provider(_) => 4,
    }
}

fn normalized_light_direction(light_direction: Vec3) -> Vec3 {
    if is_finite_vec3(light_direction) && light_direction.length_squared() > 0.0 {
        light_direction.normalize()
    } else {
        light_direction
    }
}

fn vec3_key(value: Vec3) -> [u32; 3] {
    [
        canonical_f32_bits(value.x),
        canonical_f32_bits(value.y),
        canonical_f32_bits(value.z),
    ]
}

fn canonical_f32_bits(value: f32) -> u32 {
    if value.is_nan() {
        f32::NAN.to_bits()
    } else if value.to_bits() == (-0.0_f32).to_bits() {
        0.0_f32.to_bits()
    } else {
        value.to_bits()
    }
}

fn is_finite_vec3(value: Vec3) -> bool {
    value.x.is_finite() && value.y.is_finite() && value.z.is_finite()
}

#[derive(Debug, Copy, Clone)]
struct OverlayHasher {
    value: u64,
}

impl OverlayHasher {
    fn new() -> Self {
        Self {
            value: OVERLAY_HASH_OFFSET,
        }
    }

    fn write_optional_str(&mut self, value: Option<&str>) {
        match value {
            Some(value) => {
                self.write_u8(1);
                self.write_str(value);
            }
            None => self.write_u8(0),
        }
    }

    fn write_str(&mut self, value: &str) {
        self.write_u64(value.len() as u64);
        self.write_bytes(value.as_bytes());
    }

    fn write_u8(&mut self, value: u8) {
        self.write_bytes(&[value]);
    }

    fn write_u64(&mut self, value: u64) {
        self.write_bytes(&value.to_le_bytes());
    }

    fn write_bytes(&mut self, bytes: &[u8]) {
        for byte in bytes {
            self.value ^= u64::from(*byte);
            self.value = self.value.wrapping_mul(OVERLAY_HASH_PRIME);
        }
    }

    fn finish(self) -> u64 {
        self.value
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn image(pixels: &[[u8; 4]]) -> RenderedImage {
        RenderedImage {
            width: 2,
            height: 2,
            rgba8: pixels.iter().flatten().copied().collect(),
        }
    }

    #[test]
    fn rendered_perceptual_report_rejects_identical_preview() {
        let background = [0, 0, 0, 255];
        let parent = image(&[
            background,
            [200, 200, 200, 255],
            background,
            [180, 180, 180, 255],
        ]);
        let report = classify_foundry_rendered_perceptual_report(
            "same",
            &[(&parent, &parent), (&parent, &parent)],
            background,
        );

        assert_eq!(
            report.legibility_class,
            CandidateLegibilityClass::DuplicateLooking
        );
        assert!(report.reject_reason.is_some());
    }

    #[test]
    fn rendered_perceptual_report_requires_multi_camera_evidence() {
        let background = [0, 0, 0, 255];
        let parent = image(&[
            background,
            [200, 200, 200, 255],
            background,
            [180, 180, 180, 255],
        ]);
        let candidate = image(&[
            [200, 200, 200, 255],
            background,
            [220, 220, 220, 255],
            background,
        ]);

        let report = classify_foundry_rendered_perceptual_report(
            "single",
            &[(&parent, &candidate)],
            background,
        );

        assert_eq!(
            report.legibility_class,
            CandidateLegibilityClass::Unsupported
        );
        assert!(report.reject_reason.is_some());
    }

    #[test]
    fn rendered_perceptual_report_accepts_obvious_delta() {
        let background = [0, 0, 0, 255];
        let parent = image(&[
            background,
            [200, 200, 200, 255],
            background,
            [180, 180, 180, 255],
        ]);
        let candidate = image(&[
            [200, 200, 200, 255],
            background,
            [220, 220, 220, 255],
            background,
        ]);
        let report = classify_foundry_rendered_perceptual_report(
            "changed",
            &[(&parent, &candidate), (&parent, &candidate)],
            background,
        );

        assert!(matches!(
            report.legibility_class,
            CandidateLegibilityClass::Clear | CandidateLegibilityClass::Strong
        ));
        assert_eq!(report.render_delta_by_camera.len(), 2);
        assert!(report.silhouette_delta > 0.0);
    }

    #[test]
    fn rendered_perceptual_report_is_deterministic() {
        let background = [0, 0, 0, 255];
        let parent = image(&[
            background,
            [200, 200, 200, 255],
            background,
            [180, 180, 180, 255],
        ]);
        let candidate = image(&[
            [200, 200, 200, 255],
            background,
            [220, 220, 220, 255],
            background,
        ]);

        let first = classify_foundry_rendered_perceptual_report(
            "changed",
            &[(&parent, &candidate), (&parent, &candidate)],
            background,
        );
        let second = classify_foundry_rendered_perceptual_report(
            "changed",
            &[(&parent, &candidate), (&parent, &candidate)],
            background,
        );

        assert_eq!(first, second);
    }
}
