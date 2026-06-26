#![forbid(unsafe_code)]

//! Surface Lab contracts for static prop UV/material/texture readiness.
//!
//! The metadata package remains useful for policy/status sidecars. Surface Lab
//! v1 adds a concrete static-prop surface artifact with deterministic UVs,
//! material-slot coverage, generated texture payload references, and evidence
//! files while still avoiding novice UI coupling or game-ready overclaims.

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Component, Path};

use serde::{Deserialize, Serialize};

use crate::GameAssetValidationIssue;
use crate::export::StaticPropFeatureStatus;

/// Surface Lab package schema version.
pub const SURFACE_LAB_PACKAGE_SCHEMA_VERSION: u32 = 1;

/// Surface Lab validation report schema version.
pub const SURFACE_LAB_VALIDATION_REPORT_SCHEMA_VERSION: u32 = 1;

/// Surface artifact schema version.
pub const SURFACE_ARTIFACT_SCHEMA_VERSION: u32 = 1;

/// Surface capability sidecar schema version.
pub const SURFACE_CAPABILITIES_SCHEMA_VERSION: u32 = 1;

/// Surface visual delta report schema version.
pub const SURFACE_VISUAL_DELTA_REPORT_SCHEMA_VERSION: u32 = 1;

/// Sci-Fi Crate material variant candidate schema version.
pub const SURFACE_MATERIAL_VARIANT_CANDIDATES_SCHEMA_VERSION: u32 = 1;

/// Surface readiness status.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SurfaceLabStatus {
    /// Automated checks found no blocker.
    Ready,
    /// One or more blockers prevent a surface-ready claim.
    Blocked,
}

/// One Surface Lab metadata package.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SurfaceLabPackage {
    /// Package schema version.
    pub schema_version: u32,
    /// Source profile slug.
    pub profile_id: String,
    /// Human-facing asset name.
    pub display_name: String,
    /// Human-facing asset family.
    pub asset_family: String,
    /// UV policy and blocker state.
    pub uv_layout: SurfaceUvLayoutPolicy,
    /// Procedural/material metadata pack.
    pub material_pack: MaterialStylePack,
    /// Texture channel requirements for a future full claim.
    pub texture_requirements: TextureRequirementSet,
    /// Unsupported texture/material outputs recorded truthfully.
    pub unsupported_outputs: UnsupportedSurfaceOutputReport,
    /// Swatch/contact-sheet evidence references.
    pub evidence: SurfaceEvidence,
}

/// UV policy for a static prop.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SurfaceUvLayoutPolicy {
    /// UV readiness state.
    pub status: StaticPropFeatureStatus,
    /// Whether UV layout is required before any texture-ready claim.
    pub required_for_texture_ready: bool,
    /// Strategy name.
    pub policy: String,
    /// Stable blocker code when UVs are unavailable.
    pub blocker_code: Option<String>,
    /// Plain-language explanation.
    pub explanation: String,
}

/// A procedural material style pack.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MaterialStylePack {
    /// Stable material pack ID.
    pub id: String,
    /// Human-facing material pack name.
    pub display_name: String,
    /// Texture packing policy.
    pub texture_packing_policy: String,
    /// Authored material recipes.
    pub recipes: Vec<MaterialRecipe>,
    /// Slot-to-recipe bindings.
    pub slot_bindings: Vec<MaterialSlotBinding>,
}

/// One procedural material recipe.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MaterialRecipe {
    /// Stable material recipe ID.
    pub material_id: String,
    /// Human-facing material name.
    pub display_name: String,
    /// PBR base color in sRGB.
    pub base_color_srgb: [u8; 3],
    /// Metallic factor.
    pub metallic: f32,
    /// Roughness factor.
    pub roughness: f32,
    /// Wear/damage policy.
    pub wear_policy: String,
    /// Whether final texture payloads are included.
    pub texture_payload_ready: bool,
}

/// Binding from semantic material slot to material recipe.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MaterialSlotBinding {
    /// Stable material slot ID.
    pub slot_id: String,
    /// Human-facing material slot label.
    pub display_name: String,
    /// Referenced material recipe ID.
    pub material_id: String,
    /// Semantic roles covered by this binding.
    pub semantic_roles: Vec<String>,
}

/// Texture requirements for future runtime/DCC packages.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TextureRequirementSet {
    /// Stable texture requirement set ID.
    pub id: String,
    /// Required texture channels.
    pub channels: Vec<TextureRequirement>,
}

/// One texture channel requirement.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TextureRequirement {
    /// Texture channel key.
    pub channel: TextureChannel,
    /// Recommended square resolution.
    pub resolution_px: u32,
    /// Whether this channel is required before a texture-ready claim.
    pub required_for_texture_ready: bool,
    /// Current readiness state.
    pub status: StaticPropFeatureStatus,
    /// Stable blocker code if not ready.
    pub blocker_code: Option<String>,
    /// Plain-language requirement.
    pub explanation: String,
}

/// Texture channel vocabulary.
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TextureChannel {
    /// Base color/albedo texture.
    BaseColor,
    /// Normal texture.
    Normal,
    /// Metallic-roughness packed texture.
    MetallicRoughness,
    /// Ambient occlusion texture.
    Occlusion,
    /// Emissive texture.
    Emissive,
}

const REQUIRED_READY_TEXTURE_CHANNELS: [TextureChannel; 4] = [
    TextureChannel::BaseColor,
    TextureChannel::MetallicRoughness,
    TextureChannel::Normal,
    TextureChannel::Occlusion,
];

/// Unsupported surface output report.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UnsupportedSurfaceOutputReport {
    /// Unsupported outputs.
    pub outputs: Vec<UnsupportedSurfaceOutput>,
}

/// One unsupported output.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UnsupportedSurfaceOutput {
    /// Stable output key.
    pub output: String,
    /// Stable blocker code.
    pub blocker_code: String,
    /// Plain-language explanation.
    pub explanation: String,
}

/// Surface evidence references.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SurfaceEvidence {
    /// Swatch-sheet PNG.
    pub swatch_sheet: String,
    /// Surface validation report JSON.
    pub validation_report: String,
}

/// Concrete static-prop surface artifact schema.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SurfaceArtifact {
    /// Artifact schema version.
    pub schema_version: u32,
    /// Source profile slug.
    pub profile_id: String,
    /// Human-facing asset name.
    pub display_name: String,
    /// Source artifact fingerprint.
    pub source_artifact_fingerprint: String,
    /// Stable source recipe hash.
    pub source_recipe_hash: u64,
    /// Package-relative frozen mesh reference.
    pub frozen_mesh_ref: String,
    /// Emitted UV sets.
    pub uv_sets: Vec<SurfaceUvSet>,
    /// Material slots with coverage.
    pub material_slots: Vec<SurfaceMaterialSlot>,
    /// Texture payload sets.
    pub texture_sets: Vec<SurfaceTextureSet>,
    /// One material/UV binding per triangle.
    pub triangle_bindings: Vec<SurfaceTriangleBinding>,
    /// Surface evidence references.
    pub evidence: SurfaceArtifactEvidence,
    /// Package-relative validation report reference.
    pub validation_report_ref: String,
    /// Manual review status for the surface artifact.
    pub manual_review: SurfaceReviewStatus,
}

/// One UV coordinate set.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SurfaceUvSet {
    /// Stable UV set key.
    pub id: String,
    /// Human-facing UV set name.
    pub display_name: String,
    /// Zero-based channel index.
    pub channel_index: u32,
    /// Number of UV coordinates.
    pub coordinate_count: u32,
    /// UV coordinates in exported vertex order.
    pub coordinates: Vec<[f32; 2]>,
    /// UV generation policy.
    pub source_policy: String,
    /// Whether the set passed automated readiness checks.
    pub readiness_status: StaticPropFeatureStatus,
    /// Whether coordinates outside 0..1 are intentional tiling.
    #[serde(default)]
    pub tiling_allowed: bool,
}

/// One triangle to material/UV binding.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SurfaceTriangleBinding {
    /// Zero-based triangle index.
    pub triangle_index: u32,
    /// Referenced material slot ID.
    pub material_slot_id: String,
    /// Referenced UV set ID.
    pub uv_set_id: String,
    /// Optional source part label.
    pub source_part: Option<String>,
    /// Optional source region label.
    pub source_region: Option<String>,
    /// Optional source operation label.
    pub source_operation: Option<String>,
}

/// Surface material slot coverage.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SurfaceMaterialSlot {
    /// Stable slot ID.
    pub slot_id: String,
    /// Human-facing display name.
    pub display_name: String,
    /// Human-facing semantic roles covered by this slot.
    pub semantic_roles: Vec<String>,
    /// Referenced material recipe ID.
    pub recipe_id: String,
    /// Number of covered triangles.
    pub coverage_triangle_count: u32,
    /// Covered triangle fraction from 0 to 1.
    pub coverage_fraction: f32,
}

/// One generated texture payload set.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SurfaceTextureSet {
    /// Stable texture set ID.
    pub id: String,
    /// Human-facing texture set name.
    pub display_name: String,
    /// Referenced material recipe ID.
    pub material_recipe_id: String,
    /// Texture files in deterministic channel order.
    pub files: Vec<SurfaceTextureFile>,
    /// Deterministic procedural source description.
    pub procedural_source: String,
    /// Whether files are present and validated.
    pub payload_ready: bool,
}

/// One generated texture file.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SurfaceTextureFile {
    /// Texture channel.
    pub channel: TextureChannel,
    /// Package-relative texture path.
    pub path: String,
    /// Declared width.
    pub width: u32,
    /// Declared height.
    pub height: u32,
    /// Color space label, for example `sRGB` or `linear`.
    pub color_space: String,
    /// Whether this file is required before a texture-ready claim.
    pub required_for_texture_ready: bool,
}

/// Surface artifact evidence references.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SurfaceArtifactEvidence {
    /// UV layout PNG.
    pub uv_layout: String,
    /// Material swatch PNG.
    pub material_swatch_sheet: String,
    /// Texture contact sheet PNG.
    pub texture_contact_sheet: String,
    /// Triangle slot coverage JSON.
    pub triangle_slot_coverage: String,
}

/// Manual/art review status for surface artifacts.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SurfaceReviewStatus {
    /// No manual review happened yet.
    NotReviewed,
    /// Automated checks passed, but manual review remains required.
    AutomatedReady,
    /// Manual review is required before final game-ready use.
    ManualRequired,
    /// Manual review approved the artifact.
    Approved,
    /// Manual review rejected the artifact.
    Rejected,
}

/// Future UI capability sidecar for headless surface payloads.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SurfaceCapabilities {
    /// Sidecar schema version.
    pub schema_version: u32,
    /// Source profile slug.
    pub profile_id: String,
    /// Whether a concrete surface payload exists.
    pub surface_payload_ready: bool,
    /// Whether UV evidence exists.
    pub uv_ready: bool,
    /// Supported material slot IDs.
    pub material_slots: Vec<String>,
    /// Supported texture channels.
    pub texture_channels: Vec<TextureChannel>,
    /// Supported variation channels.
    pub variation_channels_supported: SurfaceVariationChannels,
    /// Whether headless textured previews/contact sheets exist as visual evidence.
    #[serde(default)]
    pub surface_visual_evidence_ready: bool,
    /// Whether part-specific surface editing is ready.
    pub focus_part_surface_ready: bool,
    /// Human-facing label.
    pub human_label: String,
    /// Plain-language unavailable reasons.
    pub unavailable_reasons: Vec<String>,
}

/// Result class for material/surface-only visual deltas.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SurfaceVisualDeltaResultClass {
    /// Strong material/surface difference with preview evidence.
    Strong,
    /// Clear material/surface difference with preview evidence.
    Clear,
    /// Subtle but non-duplicate material/surface difference.
    Subtle,
    /// Looks too similar to the base preview.
    DuplicateLooking,
    /// Unsupported because evidence is missing or shape changed.
    Unsupported,
}

/// Evidence values used to classify a surface-only visual delta.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SurfaceVisualDeltaEvidence {
    /// Frozen mesh fingerprint for the base artifact.
    pub base_frozen_mesh_fingerprint: String,
    /// Frozen mesh fingerprint for the candidate artifact.
    pub candidate_frozen_mesh_fingerprint: String,
    /// Base textured preview reference.
    #[serde(default)]
    pub base_textured_preview_ref: Option<String>,
    /// Candidate textured preview reference.
    #[serde(default)]
    pub candidate_textured_preview_ref: Option<String>,
    /// Optional normalized visible pixel delta from textured previews.
    #[serde(default)]
    pub visible_surface_pixel_delta: Option<f32>,
}

/// Surface-only visual delta report.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SurfaceVisualDeltaReport {
    /// Report schema version.
    pub schema_version: u32,
    /// Base profile slug.
    pub profile_id: String,
    /// Candidate ID, when this report describes a variant.
    pub candidate_id: String,
    /// Normalized base-color recipe delta.
    pub base_color_delta: f32,
    /// Normalized slot-to-recipe binding delta.
    pub material_slot_delta: f32,
    /// Normalized texture-channel metadata delta.
    pub texture_channel_delta: f32,
    /// Optional normalized wear-policy delta.
    #[serde(default)]
    pub wear_delta: Option<f32>,
    /// Normalized visible surface pixel delta from textured previews.
    pub visible_surface_pixel_delta: f32,
    /// True when frozen mesh/topology changed and the candidate leaked shape delta.
    pub shape_delta_leak_detected: bool,
    /// Classified result.
    pub result_class: SurfaceVisualDeltaResultClass,
    /// Stable blocker/diagnostic codes.
    pub diagnostics: Vec<String>,
}

impl SurfaceVisualDeltaReport {
    /// Return true only when this is a supported surface-only delta with
    /// preview evidence.
    #[must_use]
    pub fn can_pass_surface_delta(&self) -> bool {
        !self.shape_delta_leak_detected
            && !matches!(
                self.result_class,
                SurfaceVisualDeltaResultClass::Unsupported
                    | SurfaceVisualDeltaResultClass::DuplicateLooking
            )
            && self
                .diagnostics
                .iter()
                .all(|code| code != "missing_textured_preview_evidence")
    }
}

/// One material-only surface variant candidate.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SurfaceMaterialVariantCandidate {
    /// Stable candidate ID.
    pub candidate_id: String,
    /// Product-facing candidate title.
    pub title: String,
    /// Product-facing variant name.
    pub display_name: String,
    /// Short product-facing summary of the material-only change.
    pub summary: String,
    /// Package-relative candidate directory.
    pub variant_dir: String,
    /// Material slots whose recipe payload changed from the base artifact.
    pub changed_material_slots: Vec<String>,
    /// Package-relative surface artifact override.
    pub surface_artifact_ref: String,
    /// Package-relative material pack override.
    pub material_pack_ref: String,
    /// Package-relative textured preview PNG.
    pub textured_preview_ref: String,
    /// Package-relative preview PNG, duplicated for future UI metadata readers.
    pub preview_path: String,
    /// Package-relative surface delta report.
    pub surface_delta_ref: String,
    /// Embedded surface-only delta report.
    pub surface_delta: SurfaceVisualDeltaReport,
    /// Package-relative texture files emitted for this candidate.
    pub texture_files: Vec<String>,
    /// Frozen mesh fingerprint copied from the base artifact.
    pub frozen_mesh_fingerprint: String,
    /// True when geometry, UVs, and triangle/material slot bindings are preserved.
    pub preserves_frozen_geometry: bool,
    /// Full game-ready status remains blocked until manual/engine proof exists.
    pub full_ready_status: SurfaceLabStatus,
    /// True when this candidate must not be represented as fully game-ready.
    pub blocked_full_ready: bool,
    /// Candidate result class copied from the delta report.
    pub result_class: SurfaceVisualDeltaResultClass,
    /// Candidate diagnostics.
    pub diagnostics: Vec<String>,
}

/// Candidate list emitted by the headless material-only variant generator.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SurfaceMaterialVariantCandidateSet {
    /// Candidate-set schema version.
    pub schema_version: u32,
    /// Source profile slug.
    pub profile_id: String,
    /// Base surface artifact.
    pub base_surface_artifact_ref: String,
    /// Base textured preview.
    pub base_textured_preview_ref: String,
    /// Candidates in deterministic order.
    pub candidates: Vec<SurfaceMaterialVariantCandidate>,
}

/// Surface variation channel availability.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SurfaceVariationChannels {
    /// Whole-surface variation.
    pub surface: bool,
    /// Wear/damage variation.
    pub wear: bool,
}

/// One validation check result.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SurfaceLabValidationCheck {
    /// Stable check code.
    pub code: String,
    /// Whether this check passed.
    pub passed: bool,
    /// Plain-language message.
    pub message: String,
}

/// Surface Lab validation report.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SurfaceLabValidationReport {
    /// Report schema version.
    pub schema_version: u32,
    /// Profile ID copied from the package.
    pub profile_id: String,
    /// Overall status.
    pub status: SurfaceLabStatus,
    /// Validation checks.
    pub checks: Vec<SurfaceLabValidationCheck>,
    /// Blocking issues.
    pub blockers: Vec<GameAssetValidationIssue>,
    /// Non-blocking warnings.
    pub warnings: Vec<GameAssetValidationIssue>,
}

impl SurfaceLabValidationReport {
    /// Return true when the package can make a surface-ready claim.
    #[must_use]
    pub fn is_ready(&self) -> bool {
        self.status == SurfaceLabStatus::Ready && self.blockers.is_empty()
    }
}

/// Validate a concrete Surface Lab artifact without checking files on disk.
#[must_use]
pub fn validate_surface_artifact(artifact: &SurfaceArtifact) -> SurfaceLabValidationReport {
    validate_surface_artifact_inner(artifact, SurfaceArtifactValidation::Unverified)
}

/// Validate a concrete Surface Lab artifact and prove referenced files exist
/// below `package_root`.
#[must_use]
pub fn validate_surface_artifact_with_root(
    artifact: &SurfaceArtifact,
    package_root: impl AsRef<Path>,
) -> SurfaceLabValidationReport {
    validate_surface_artifact_inner(
        artifact,
        SurfaceArtifactValidation::Filesystem(package_root.as_ref()),
    )
}

fn validate_surface_artifact_inner(
    artifact: &SurfaceArtifact,
    artifacts: SurfaceArtifactValidation<'_>,
) -> SurfaceLabValidationReport {
    let mut builder = SurfaceLabValidationReportBuilder::new(&artifact.profile_id);
    builder.require(
        "surface_artifact_schema_version_supported",
        artifact.schema_version == SURFACE_ARTIFACT_SCHEMA_VERSION,
        Some("schema_version"),
        "unsupported_surface_artifact_schema",
        "Surface artifact schema is supported.",
        "Surface artifact schema is not supported.",
    );
    builder.require_non_empty("profile_id", &artifact.profile_id, "empty_profile_id");
    builder.require_non_empty("display_name", &artifact.display_name, "empty_display_name");
    builder.require_non_empty(
        "source_artifact_fingerprint",
        &artifact.source_artifact_fingerprint,
        "missing_source_artifact_fingerprint",
    );
    builder.require(
        "source_recipe_hash_recorded",
        artifact.source_recipe_hash != 0,
        Some("source_recipe_hash"),
        "missing_source_recipe_hash",
        "Source recipe hash is recorded.",
        "Source recipe hash must be recorded.",
    );
    builder.require_non_empty(
        "frozen_mesh_ref",
        &artifact.frozen_mesh_ref,
        "missing_frozen_mesh_ref",
    );
    validate_evidence_path_only(
        &mut builder,
        "validation_report_ref",
        &artifact.validation_report_ref,
    );
    validate_surface_uv_sets(&mut builder, artifact);
    validate_surface_material_slots(&mut builder, artifact);
    validate_surface_triangle_bindings(&mut builder, artifact);
    validate_surface_texture_sets(&mut builder, artifact, artifacts);
    validate_surface_artifact_evidence(&mut builder, artifact, artifacts);
    validate_surface_manual_review(&mut builder, artifact.manual_review);
    builder.finish()
}

/// Validate a Surface Lab package without checking files on disk.
#[must_use]
pub fn validate_surface_lab_package(package: &SurfaceLabPackage) -> SurfaceLabValidationReport {
    validate_surface_lab_package_inner(package, SurfaceArtifactValidation::Unverified)
}

/// Validate a Surface Lab package and prove referenced files exist below root.
#[must_use]
pub fn validate_surface_lab_package_with_root(
    package: &SurfaceLabPackage,
    root: impl AsRef<Path>,
) -> SurfaceLabValidationReport {
    validate_surface_lab_package_inner(
        package,
        SurfaceArtifactValidation::Filesystem(root.as_ref()),
    )
}

/// Build a surface-only visual delta report for two artifacts. The report is
/// unsupported when textured preview evidence is missing or frozen geometry
/// fingerprints differ.
#[must_use]
pub fn surface_visual_delta_report(
    base: &SurfaceArtifact,
    candidate: &SurfaceArtifact,
    candidate_id: impl Into<String>,
    evidence: SurfaceVisualDeltaEvidence,
) -> SurfaceVisualDeltaReport {
    let mut diagnostics = Vec::new();
    let shape_delta_leak_detected = evidence.base_frozen_mesh_fingerprint
        != evidence.candidate_frozen_mesh_fingerprint
        || base.source_artifact_fingerprint != candidate.source_artifact_fingerprint
        || base.frozen_mesh_ref != candidate.frozen_mesh_ref
        || base.uv_sets != candidate.uv_sets
        || base.triangle_bindings != candidate.triangle_bindings;
    if shape_delta_leak_detected {
        diagnostics.push("shape_delta_leak_detected".to_owned());
    }
    if evidence
        .base_textured_preview_ref
        .as_deref()
        .is_none_or(|path| path.trim().is_empty())
        || evidence
            .candidate_textured_preview_ref
            .as_deref()
            .is_none_or(|path| path.trim().is_empty())
        || evidence.visible_surface_pixel_delta.is_none()
    {
        diagnostics.push("missing_textured_preview_evidence".to_owned());
    }

    let base_color_delta = base_color_delta(base, candidate);
    let material_slot_delta = material_slot_delta(base, candidate);
    let texture_channel_delta = texture_channel_delta(base, candidate);
    let wear_delta_value = wear_delta(base, candidate);
    let wear_delta = Some(wear_delta_value);
    let visible_surface_pixel_delta = evidence.visible_surface_pixel_delta.unwrap_or(0.0);

    let result_class = if diagnostics.iter().any(|code| {
        code == "missing_textured_preview_evidence" || code == "shape_delta_leak_detected"
    }) {
        SurfaceVisualDeltaResultClass::Unsupported
    } else {
        classify_surface_delta(
            base_color_delta,
            material_slot_delta,
            texture_channel_delta,
            wear_delta_value,
            visible_surface_pixel_delta,
        )
    };
    if result_class == SurfaceVisualDeltaResultClass::DuplicateLooking {
        diagnostics.push("duplicate_looking_surface_variant".to_owned());
    }

    SurfaceVisualDeltaReport {
        schema_version: SURFACE_VISUAL_DELTA_REPORT_SCHEMA_VERSION,
        profile_id: base.profile_id.clone(),
        candidate_id: candidate_id.into(),
        base_color_delta,
        material_slot_delta,
        texture_channel_delta,
        wear_delta,
        visible_surface_pixel_delta,
        shape_delta_leak_detected,
        result_class,
        diagnostics,
    }
}

/// Validate the candidate set emitted by the material-only variant generator.
#[must_use]
pub fn validate_surface_material_variant_candidate_set(
    set: &SurfaceMaterialVariantCandidateSet,
) -> SurfaceLabValidationReport {
    let mut builder = SurfaceLabValidationReportBuilder::new(&set.profile_id);
    builder.require(
        "surface_variant_candidate_schema_version_supported",
        set.schema_version == SURFACE_MATERIAL_VARIANT_CANDIDATES_SCHEMA_VERSION,
        Some("schema_version"),
        "unsupported_surface_variant_candidate_schema",
        "Surface variant candidate schema is supported.",
        "Surface variant candidate schema is not supported.",
    );
    builder.require_non_empty("profile_id", &set.profile_id, "empty_profile_id");
    builder.require_non_empty(
        "base_surface_artifact_ref",
        &set.base_surface_artifact_ref,
        "missing_base_surface_artifact_ref",
    );
    builder.require_non_empty(
        "base_textured_preview_ref",
        &set.base_textured_preview_ref,
        "missing_base_textured_preview_ref",
    );
    builder.require(
        "surface_variant_candidate_count_supported",
        (6..=12).contains(&set.candidates.len()),
        Some("candidates"),
        "surface_variant_candidate_count_out_of_range",
        "Surface variant generator emitted 6 to 12 candidates.",
        "Surface variant generator must emit 6 to 12 candidates.",
    );
    let mut ids = BTreeSet::new();
    for candidate in &set.candidates {
        builder.require_non_empty(
            format!("candidates.{}.candidate_id", candidate.candidate_id),
            &candidate.candidate_id,
            "empty_surface_variant_candidate_id",
        );
        if !ids.insert(candidate.candidate_id.as_str()) {
            builder.block(
                Some(format!(
                    "candidates.{}.candidate_id",
                    candidate.candidate_id
                )),
                "duplicate_surface_variant_candidate_id",
                "Surface variant candidate IDs must be unique.",
            );
        }
        builder.require_non_empty(
            format!("candidates.{}.title", candidate.candidate_id),
            &candidate.title,
            "empty_surface_variant_title",
        );
        builder.require_non_empty(
            format!("candidates.{}.display_name", candidate.candidate_id),
            &candidate.display_name,
            "empty_surface_variant_display_name",
        );
        builder.require_non_empty(
            format!("candidates.{}.summary", candidate.candidate_id),
            &candidate.summary,
            "empty_surface_variant_summary",
        );
        builder.require(
            format!(
                "candidate_{}_changed_slots_recorded",
                candidate.candidate_id
            ),
            !candidate.changed_material_slots.is_empty(),
            Some(format!(
                "candidates.{}.changed_material_slots",
                candidate.candidate_id
            )),
            "surface_variant_changed_slots_missing",
            "Surface variant records changed material slots.",
            "Surface variant candidates must record changed material slots.",
        );
        builder.require_non_empty(
            format!("candidates.{}.preview_path", candidate.candidate_id),
            &candidate.preview_path,
            "missing_surface_variant_preview_path",
        );
        builder.require(
            format!(
                "candidate_{}_preview_path_matches_ref",
                candidate.candidate_id
            ),
            candidate.preview_path == candidate.textured_preview_ref,
            Some(format!(
                "candidates.{}.preview_path",
                candidate.candidate_id
            )),
            "surface_variant_preview_path_mismatch",
            "Surface variant preview metadata points at the textured preview.",
            "Surface variant preview_path must match textured_preview_ref.",
        );
        builder.require(
            format!(
                "candidate_{}_texture_files_recorded",
                candidate.candidate_id
            ),
            !candidate.texture_files.is_empty(),
            Some(format!(
                "candidates.{}.texture_files",
                candidate.candidate_id
            )),
            "surface_variant_texture_files_missing",
            "Surface variant records generated texture files.",
            "Surface variant candidates must record generated texture files.",
        );
        builder.require(
            format!("candidate_{}_surface_delta_matches", candidate.candidate_id),
            candidate.surface_delta.candidate_id == candidate.candidate_id
                && candidate.surface_delta.result_class == candidate.result_class,
            Some(format!(
                "candidates.{}.surface_delta",
                candidate.candidate_id
            )),
            "surface_variant_delta_metadata_mismatch",
            "Surface variant embeds matching surface delta metadata.",
            "Surface variant embedded surface delta must match candidate metadata.",
        );
        builder.require(
            format!(
                "candidate_{}_preserves_frozen_geometry",
                candidate.candidate_id
            ),
            candidate.preserves_frozen_geometry,
            Some(format!(
                "candidates.{}.preserves_frozen_geometry",
                candidate.candidate_id
            )),
            "surface_variant_changed_geometry",
            "Material-only surface variant preserves frozen geometry.",
            "Material-only surface variants must preserve geometry, UVs, and triangle/material slot bindings.",
        );
        builder.require(
            format!("candidate_{}_full_ready_blocked", candidate.candidate_id),
            candidate.full_ready_status == SurfaceLabStatus::Blocked
                && candidate.blocked_full_ready,
            Some(format!(
                "candidates.{}.full_ready_status",
                candidate.candidate_id
            )),
            "surface_variant_full_ready_overclaim",
            "Surface variant remains blocked from full game-ready status.",
            "Surface variants must stay blocked without manual review and engine proof.",
        );
        if candidate.result_class == SurfaceVisualDeltaResultClass::DuplicateLooking
            || candidate
                .diagnostics
                .iter()
                .any(|code| code == "duplicate_looking_surface_variant")
        {
            builder.block(
                Some(format!(
                    "candidates.{}.result_class",
                    candidate.candidate_id
                )),
                "duplicate_looking_surface_variant",
                "Duplicate-looking material variants must be rejected.",
            );
        }
        if candidate.result_class == SurfaceVisualDeltaResultClass::Unsupported {
            builder.warn(
                Some(format!(
                    "candidates.{}.result_class",
                    candidate.candidate_id
                )),
                "surface_variant_unsupported",
                "Surface variant is present as diagnostic evidence but cannot pass.",
            );
        }
    }
    builder.finish()
}

fn validate_surface_lab_package_inner(
    package: &SurfaceLabPackage,
    artifacts: SurfaceArtifactValidation<'_>,
) -> SurfaceLabValidationReport {
    let mut builder = SurfaceLabValidationReportBuilder::new(&package.profile_id);
    builder.require(
        "schema_version_supported",
        package.schema_version == SURFACE_LAB_PACKAGE_SCHEMA_VERSION,
        Some("schema_version"),
        "unsupported_surface_lab_schema",
        "Surface Lab package schema is supported.",
        "Surface Lab package schema is not supported.",
    );
    builder.require_non_empty("profile_id", &package.profile_id, "empty_profile_id");
    builder.require_non_empty("display_name", &package.display_name, "empty_display_name");
    builder.require_non_empty("asset_family", &package.asset_family, "empty_asset_family");
    validate_uv_layout(&mut builder, &package.uv_layout);
    validate_material_pack(&mut builder, &package.material_pack);
    validate_texture_requirements(&mut builder, &package.texture_requirements);
    validate_unsupported_outputs(&mut builder, &package.unsupported_outputs);
    validate_evidence(&mut builder, &package.evidence, artifacts);
    builder.finish()
}

fn base_color_delta(base: &SurfaceArtifact, candidate: &SurfaceArtifact) -> f32 {
    let base_colors = recipe_base_colors(base);
    let candidate_colors = recipe_base_colors(candidate);
    let mut compared = 0_u32;
    let mut total = 0.0_f32;
    for (recipe_id, base_color) in base_colors {
        if let Some(candidate_color) = candidate_colors.get(recipe_id) {
            compared = compared.saturating_add(1);
            total += color_distance(base_color, *candidate_color);
        }
    }
    if compared == 0 {
        0.0
    } else {
        (total / compared as f32).clamp(0.0, 1.0)
    }
}

fn recipe_base_colors(artifact: &SurfaceArtifact) -> BTreeMap<&str, [u8; 3]> {
    artifact
        .texture_sets
        .iter()
        .filter_map(|set| {
            let color = set
                .files
                .iter()
                .find(|file| file.channel == TextureChannel::BaseColor)
                .and_then(|file| base_color_hint_from_path(&file.path));
            color.map(|color| (set.material_recipe_id.as_str(), color))
        })
        .collect()
}

fn base_color_hint_from_path(path: &str) -> Option<[u8; 3]> {
    let lower = path.to_ascii_lowercase();
    let color = if lower.contains("clean-lab-white") {
        [218, 224, 218]
    } else if lower.contains("worn-hazard-yellow") {
        [204, 166, 42]
    } else if lower.contains("dark-industrial-metal") {
        [42, 47, 50]
    } else if lower.contains("field-blue-utility") {
        [55, 94, 140]
    } else if lower.contains("graphite-cargo") {
        [70, 76, 76]
    } else if lower.contains("orange-warning-trim") {
        [216, 102, 36]
    } else {
        return None;
    };
    Some(color)
}

fn color_distance(left: [u8; 3], right: [u8; 3]) -> f32 {
    let dr = f32::from(left[0]) - f32::from(right[0]);
    let dg = f32::from(left[1]) - f32::from(right[1]);
    let db = f32::from(left[2]) - f32::from(right[2]);
    ((dr * dr + dg * dg + db * db).sqrt() / (255.0 * 3.0_f32.sqrt())).clamp(0.0, 1.0)
}

fn material_slot_delta(base: &SurfaceArtifact, candidate: &SurfaceArtifact) -> f32 {
    let base_slots = base
        .material_slots
        .iter()
        .map(|slot| (slot.slot_id.as_str(), slot.recipe_id.as_str()))
        .collect::<BTreeMap<_, _>>();
    let candidate_slots = candidate
        .material_slots
        .iter()
        .map(|slot| (slot.slot_id.as_str(), slot.recipe_id.as_str()))
        .collect::<BTreeMap<_, _>>();
    if base_slots.is_empty() {
        return 0.0;
    }
    let changed = base_slots
        .iter()
        .filter(|(slot_id, recipe_id)| candidate_slots.get(*slot_id).copied() != Some(**recipe_id))
        .count();
    (changed as f32 / base_slots.len() as f32).clamp(0.0, 1.0)
}

fn texture_channel_delta(base: &SurfaceArtifact, candidate: &SurfaceArtifact) -> f32 {
    let base_channels = texture_channel_keys(base);
    let candidate_channels = texture_channel_keys(candidate);
    if base_channels.is_empty() {
        return 0.0;
    }
    let changed = base_channels
        .iter()
        .filter(|key| !candidate_channels.contains(*key))
        .count()
        + candidate_channels
            .iter()
            .filter(|key| !base_channels.contains(*key))
            .count();
    (changed as f32 / (base_channels.len().max(candidate_channels.len()) as f32)).clamp(0.0, 1.0)
}

fn texture_channel_keys(artifact: &SurfaceArtifact) -> BTreeSet<(String, TextureChannel, String)> {
    artifact
        .texture_sets
        .iter()
        .flat_map(|set| {
            set.files.iter().map(|file| {
                (
                    set.material_recipe_id.clone(),
                    file.channel,
                    file.path.clone(),
                )
            })
        })
        .collect()
}

fn wear_delta(base: &SurfaceArtifact, candidate: &SurfaceArtifact) -> f32 {
    if base.texture_sets == candidate.texture_sets {
        0.0
    } else {
        0.25
    }
}

fn classify_surface_delta(
    base_color_delta: f32,
    material_slot_delta: f32,
    texture_channel_delta: f32,
    wear_delta: f32,
    visible_surface_pixel_delta: f32,
) -> SurfaceVisualDeltaResultClass {
    let combined = (base_color_delta * 0.35)
        + (material_slot_delta * 0.15)
        + (texture_channel_delta * 0.15)
        + (wear_delta * 0.1)
        + (visible_surface_pixel_delta * 0.25);
    if visible_surface_pixel_delta < 0.015 && combined < 0.035 {
        SurfaceVisualDeltaResultClass::DuplicateLooking
    } else if combined >= 0.32 || visible_surface_pixel_delta >= 0.28 {
        SurfaceVisualDeltaResultClass::Strong
    } else if combined >= 0.14 || visible_surface_pixel_delta >= 0.12 {
        SurfaceVisualDeltaResultClass::Clear
    } else {
        SurfaceVisualDeltaResultClass::Subtle
    }
}

fn validate_surface_uv_sets(
    builder: &mut SurfaceLabValidationReportBuilder,
    artifact: &SurfaceArtifact,
) {
    builder.require(
        "surface_uv_sets_present",
        !artifact.uv_sets.is_empty(),
        Some("uv_sets"),
        "missing_uv_sets",
        "Surface artifact includes UV sets.",
        "Surface artifact must include at least one UV set.",
    );
    let mut ids = BTreeSet::new();
    let mut channel_indices = BTreeSet::new();
    for uv_set in &artifact.uv_sets {
        builder.require_non_empty(
            format!("uv_sets.{}.id", uv_set.id),
            &uv_set.id,
            "empty_uv_set_id",
        );
        if !ids.insert(uv_set.id.as_str()) {
            builder.block(
                Some(format!("uv_sets.{}.id", uv_set.id)),
                "duplicate_uv_set_id",
                "UV set IDs must be unique.",
            );
        }
        if !channel_indices.insert(uv_set.channel_index) {
            builder.block(
                Some(format!("uv_sets.{}.channel_index", uv_set.id)),
                "duplicate_uv_channel_index",
                "UV channel indices must be unique.",
            );
        }
        builder.require_non_empty(
            format!("uv_sets.{}.display_name", uv_set.id),
            &uv_set.display_name,
            "empty_uv_set_display_name",
        );
        builder.require_non_empty(
            format!("uv_sets.{}.source_policy", uv_set.id),
            &uv_set.source_policy,
            "empty_uv_source_policy",
        );
        builder.require(
            format!("uv_set_{}_coordinate_count_matches", uv_set.id),
            uv_set.coordinate_count as usize == uv_set.coordinates.len()
                && uv_set.coordinate_count > 0,
            Some(format!("uv_sets.{}.coordinate_count", uv_set.id)),
            "uv_coordinate_count_mismatch",
            "UV coordinate count matches the payload.",
            "UV coordinate count must match a non-empty coordinate payload.",
        );
        if uv_set.readiness_status == StaticPropFeatureStatus::Ready
            && uv_set.coordinates.is_empty()
        {
            builder.block(
                Some(format!("uv_sets.{}.readiness_status", uv_set.id)),
                "ready_uv_set_has_no_coordinates",
                "Ready UV sets must include coordinates.",
            );
        }
        for (index, coordinate) in uv_set.coordinates.iter().enumerate() {
            let finite = coordinate[0].is_finite() && coordinate[1].is_finite();
            if !finite {
                builder.block(
                    Some(format!("uv_sets.{}.coordinates.{index}", uv_set.id)),
                    "non_finite_uv_coordinate",
                    "UV coordinates must be finite.",
                );
                continue;
            }
            if !uv_set.tiling_allowed
                && (!(0.0..=1.0).contains(&coordinate[0]) || !(0.0..=1.0).contains(&coordinate[1]))
            {
                builder.block(
                    Some(format!("uv_sets.{}.coordinates.{index}", uv_set.id)),
                    "uv_coordinate_outside_normalized_atlas",
                    "UV coordinates outside 0..1 require tiling_allowed=true.",
                );
            }
        }
    }
}

fn validate_surface_material_slots(
    builder: &mut SurfaceLabValidationReportBuilder,
    artifact: &SurfaceArtifact,
) {
    builder.require(
        "surface_material_slots_present",
        !artifact.material_slots.is_empty(),
        Some("material_slots"),
        "missing_surface_material_slots",
        "Surface artifact includes material slots.",
        "Surface artifact must include at least one material slot.",
    );
    let triangle_count = triangle_count_from_bindings(&artifact.triangle_bindings);
    let mut ids = BTreeSet::new();
    let mut coverage_sum = 0_u32;
    for slot in &artifact.material_slots {
        builder.require_non_empty(
            format!("material_slots.{}.slot_id", slot.slot_id),
            &slot.slot_id,
            "empty_surface_material_slot_id",
        );
        if !ids.insert(slot.slot_id.as_str()) {
            builder.block(
                Some(format!("material_slots.{}.slot_id", slot.slot_id)),
                "duplicate_surface_material_slot_id",
                "Surface material slot IDs must be unique.",
            );
        }
        builder.require_non_empty(
            format!("material_slots.{}.display_name", slot.slot_id),
            &slot.display_name,
            "empty_surface_material_slot_display_name",
        );
        builder.require(
            format!("surface_material_slot_{}_semantic_roles", slot.slot_id),
            !slot.semantic_roles.is_empty(),
            Some(format!("material_slots.{}.semantic_roles", slot.slot_id)),
            "surface_material_slot_missing_roles",
            "Surface material slot records semantic roles.",
            "Surface material slot must record semantic roles.",
        );
        builder.require_non_empty(
            format!("material_slots.{}.recipe_id", slot.slot_id),
            &slot.recipe_id,
            "empty_surface_material_recipe_id",
        );
        builder.require(
            format!("surface_material_slot_{}_coverage_fraction", slot.slot_id),
            slot.coverage_fraction.is_finite() && (0.0..=1.0).contains(&slot.coverage_fraction),
            Some(format!("material_slots.{}.coverage_fraction", slot.slot_id)),
            "invalid_surface_material_coverage_fraction",
            "Surface material coverage fraction is finite and normalized.",
            "Surface material coverage fraction must be finite and in 0..1.",
        );
        coverage_sum = coverage_sum.saturating_add(slot.coverage_triangle_count);
    }
    builder.require(
        "surface_material_coverage_sums_to_triangle_count",
        triangle_count == 0 || coverage_sum == triangle_count,
        Some("material_slots"),
        "surface_material_coverage_mismatch",
        "Surface material coverage sums to triangle count.",
        "Surface material coverage must sum to triangle count.",
    );
}

fn validate_surface_triangle_bindings(
    builder: &mut SurfaceLabValidationReportBuilder,
    artifact: &SurfaceArtifact,
) {
    builder.require(
        "surface_triangle_bindings_present",
        !artifact.triangle_bindings.is_empty(),
        Some("triangle_bindings"),
        "missing_surface_triangle_bindings",
        "Surface artifact includes triangle bindings.",
        "Surface artifact must include one binding per triangle.",
    );
    let uv_ids = artifact
        .uv_sets
        .iter()
        .map(|set| set.id.as_str())
        .collect::<BTreeSet<_>>();
    let slot_ids = artifact
        .material_slots
        .iter()
        .map(|slot| slot.slot_id.as_str())
        .collect::<BTreeSet<_>>();
    let mut seen = BTreeSet::new();
    let mut counts_by_slot = BTreeMap::<&str, u32>::new();
    for binding in &artifact.triangle_bindings {
        if !seen.insert(binding.triangle_index) {
            builder.block(
                Some(format!("triangle_bindings.{}", binding.triangle_index)),
                "duplicate_surface_triangle_binding",
                "Every triangle must have exactly one material slot binding.",
            );
        }
        if !uv_ids.contains(binding.uv_set_id.as_str()) {
            builder.block(
                Some(format!(
                    "triangle_bindings.{}.uv_set_id",
                    binding.triangle_index
                )),
                "unknown_surface_uv_set",
                "Triangle binding references an unknown UV set.",
            );
        }
        if !slot_ids.contains(binding.material_slot_id.as_str()) {
            builder.block(
                Some(format!(
                    "triangle_bindings.{}.material_slot_id",
                    binding.triangle_index
                )),
                "unknown_surface_material_slot",
                "Triangle binding references an unknown material slot.",
            );
        }
        *counts_by_slot
            .entry(binding.material_slot_id.as_str())
            .or_default() += 1;
    }
    let triangle_count = triangle_count_from_bindings(&artifact.triangle_bindings);
    if triangle_count > 0 {
        let expected = (0..triangle_count).collect::<BTreeSet<_>>();
        builder.require(
            "surface_triangle_bindings_are_dense",
            seen == expected,
            Some("triangle_bindings"),
            "surface_triangle_binding_gap",
            "Triangle bindings cover a dense zero-based triangle range.",
            "Triangle bindings must cover every triangle exactly once.",
        );
    }
    for slot in &artifact.material_slots {
        let observed = counts_by_slot
            .get(slot.slot_id.as_str())
            .copied()
            .unwrap_or_default();
        if observed != slot.coverage_triangle_count {
            builder.block(
                Some(format!(
                    "material_slots.{}.coverage_triangle_count",
                    slot.slot_id
                )),
                "surface_material_slot_coverage_count_mismatch",
                "Material slot coverage count must match triangle bindings.",
            );
        }
    }
}

fn validate_surface_texture_sets(
    builder: &mut SurfaceLabValidationReportBuilder,
    artifact: &SurfaceArtifact,
    artifacts: SurfaceArtifactValidation<'_>,
) {
    builder.require(
        "surface_texture_sets_present",
        !artifact.texture_sets.is_empty(),
        Some("texture_sets"),
        "missing_surface_texture_sets",
        "Surface artifact includes texture sets.",
        "Surface artifact must include generated texture sets.",
    );
    let recipe_ids = artifact
        .material_slots
        .iter()
        .map(|slot| slot.recipe_id.as_str())
        .collect::<BTreeSet<_>>();
    let mut texture_recipe_ids = BTreeSet::new();
    let mut ids = BTreeSet::new();
    for set in &artifact.texture_sets {
        builder.require_non_empty(
            format!("texture_sets.{}.id", set.id),
            &set.id,
            "empty_surface_texture_set_id",
        );
        if !ids.insert(set.id.as_str()) {
            builder.block(
                Some(format!("texture_sets.{}.id", set.id)),
                "duplicate_surface_texture_set_id",
                "Surface texture set IDs must be unique.",
            );
        }
        builder.require_non_empty(
            format!("texture_sets.{}.display_name", set.id),
            &set.display_name,
            "empty_surface_texture_set_display_name",
        );
        builder.require_non_empty(
            format!("texture_sets.{}.procedural_source", set.id),
            &set.procedural_source,
            "empty_surface_texture_procedural_source",
        );
        if !recipe_ids.contains(set.material_recipe_id.as_str()) {
            builder.block(
                Some(format!("texture_sets.{}.material_recipe_id", set.id)),
                "surface_texture_unknown_recipe",
                "Texture sets must reference a material recipe used by a surface slot.",
            );
        } else {
            texture_recipe_ids.insert(set.material_recipe_id.as_str());
        }
        if !set.payload_ready {
            builder.block(
                Some(format!("texture_sets.{}.payload_ready", set.id)),
                "surface_texture_payload_not_ready",
                "Texture set is not ready.",
            );
        } else if set.files.is_empty() {
            builder.block(
                Some(format!("texture_sets.{}.files", set.id)),
                "missing_surface_texture_files",
                "Ready texture sets must include texture files.",
            );
        }
        let mut channels = BTreeSet::new();
        for file in &set.files {
            if !channels.insert(file.channel) {
                builder.block(
                    Some(format!("texture_sets.{}.files", set.id)),
                    "duplicate_surface_texture_channel",
                    "Texture channels must be listed once per set.",
                );
            }
            validate_texture_file_metadata(builder, &set.id, file, artifacts);
        }
        if set.payload_ready {
            for required_channel in REQUIRED_READY_TEXTURE_CHANNELS {
                if !channels.contains(&required_channel) {
                    builder.block(
                        Some(format!("texture_sets.{}.files", set.id)),
                        "missing_required_surface_texture_channel",
                        format!(
                            "Ready texture sets must include the {:?} channel.",
                            required_channel
                        ),
                    );
                }
            }
        }
    }
    for recipe_id in recipe_ids {
        if !texture_recipe_ids.contains(recipe_id) {
            builder.block(
                Some("texture_sets"),
                "missing_surface_texture_set_for_recipe",
                format!("Material recipe {recipe_id} must have a ready texture set."),
            );
        }
    }
}

fn validate_texture_file_metadata(
    builder: &mut SurfaceLabValidationReportBuilder,
    texture_set_id: &str,
    file: &SurfaceTextureFile,
    artifacts: SurfaceArtifactValidation<'_>,
) {
    builder.require_non_empty(
        format!(
            "texture_sets.{texture_set_id}.files.{:?}.path",
            file.channel
        ),
        &file.path,
        "empty_surface_texture_path",
    );
    builder.require(
        format!(
            "texture_file_{texture_set_id}_{:?}_dimensions",
            file.channel
        ),
        file.width > 0 && file.height > 0,
        Some(format!(
            "texture_sets.{texture_set_id}.files.{:?}",
            file.channel
        )),
        "invalid_surface_texture_dimensions",
        "Texture dimensions are positive.",
        "Texture dimensions must be positive.",
    );
    builder.require(
        format!(
            "texture_file_{texture_set_id}_{:?}_color_space",
            file.channel
        ),
        matches!(file.color_space.as_str(), "sRGB" | "linear"),
        Some(format!(
            "texture_sets.{texture_set_id}.files.{:?}.color_space",
            file.channel
        )),
        "invalid_surface_texture_color_space",
        "Texture color space is supported.",
        "Texture color space must be sRGB or linear.",
    );
    let SurfaceArtifactValidation::Filesystem(root) = artifacts else {
        builder.block(
            Some(format!(
                "texture_sets.{texture_set_id}.files.{:?}",
                file.channel
            )),
            "surface_texture_files_not_verified",
            "Surface texture files were not checked against a package directory.",
        );
        return;
    };
    validate_png_file_with_dimensions(
        builder,
        root,
        &format!("texture_sets.{texture_set_id}.files.{:?}", file.channel),
        &file.path,
        Some((file.width, file.height)),
    );
}

fn validate_surface_artifact_evidence(
    builder: &mut SurfaceLabValidationReportBuilder,
    artifact: &SurfaceArtifact,
    artifacts: SurfaceArtifactValidation<'_>,
) {
    validate_evidence_path_only(builder, "frozen_mesh_ref", &artifact.frozen_mesh_ref);
    let SurfaceArtifactValidation::Filesystem(root) = artifacts else {
        builder.block(
            Some("surface_artifact.evidence"),
            "surface_artifact_files_not_verified",
            "Surface artifact files were not checked against a package directory.",
        );
        return;
    };
    validate_path_exists_any(builder, root, "frozen_mesh_ref", &artifact.frozen_mesh_ref);
    validate_png_file_with_dimensions(
        builder,
        root,
        "evidence.uv_layout",
        &artifact.evidence.uv_layout,
        None,
    );
    validate_png_file_with_dimensions(
        builder,
        root,
        "evidence.material_swatch_sheet",
        &artifact.evidence.material_swatch_sheet,
        None,
    );
    validate_png_file_with_dimensions(
        builder,
        root,
        "evidence.texture_contact_sheet",
        &artifact.evidence.texture_contact_sheet,
        None,
    );
    validate_json_file(
        builder,
        root,
        "evidence.triangle_slot_coverage",
        &artifact.evidence.triangle_slot_coverage,
    );
}

fn validate_surface_manual_review(
    builder: &mut SurfaceLabValidationReportBuilder,
    status: SurfaceReviewStatus,
) {
    match status {
        SurfaceReviewStatus::Approved => builder.checks.push(SurfaceLabValidationCheck {
            code: "surface_manual_review_approved".to_owned(),
            passed: true,
            message: "Surface artifact has manual approval.".to_owned(),
        }),
        SurfaceReviewStatus::Rejected => builder.block(
            Some("manual_review"),
            "surface_manual_review_rejected",
            "Surface artifact was rejected by manual review.",
        ),
        SurfaceReviewStatus::NotReviewed
        | SurfaceReviewStatus::AutomatedReady
        | SurfaceReviewStatus::ManualRequired => builder.block(
            Some("manual_review"),
            "surface_manual_review_required",
            "Manual surface review is required before final game-ready claims.",
        ),
    }
}

fn triangle_count_from_bindings(bindings: &[SurfaceTriangleBinding]) -> u32 {
    bindings
        .iter()
        .map(|binding| binding.triangle_index)
        .max()
        .map_or(0, |max| max.saturating_add(1))
}

fn validate_uv_layout(
    builder: &mut SurfaceLabValidationReportBuilder,
    policy: &SurfaceUvLayoutPolicy,
) {
    builder.require_non_empty("uv_layout.policy", &policy.policy, "empty_uv_policy");
    builder.require_non_empty(
        "uv_layout.explanation",
        &policy.explanation,
        "empty_uv_explanation",
    );
    builder.require(
        "uv_required_for_texture_ready",
        policy.required_for_texture_ready,
        Some("uv_layout.required_for_texture_ready"),
        "uv_not_required_for_texture_ready",
        "UV layout is required before texture-ready claims.",
        "UV layout must be required before texture-ready claims.",
    );
    if policy.status != StaticPropFeatureStatus::Ready {
        builder.block(
            Some("uv_layout.status"),
            policy
                .blocker_code
                .as_deref()
                .unwrap_or("uv_layout_not_ready"),
            "UV layout is not ready.",
        );
    }
}

fn validate_material_pack(
    builder: &mut SurfaceLabValidationReportBuilder,
    pack: &MaterialStylePack,
) {
    builder.require_non_empty("material_pack.id", &pack.id, "empty_material_pack_id");
    builder.require_non_empty(
        "material_pack.display_name",
        &pack.display_name,
        "empty_material_pack_display_name",
    );
    builder.require_non_empty(
        "material_pack.texture_packing_policy",
        &pack.texture_packing_policy,
        "empty_texture_packing_policy",
    );
    builder.require(
        "material_recipes_present",
        !pack.recipes.is_empty(),
        Some("material_pack.recipes"),
        "missing_material_recipes",
        "Material recipes are present.",
        "At least one material recipe is required.",
    );
    let mut material_ids = BTreeSet::new();
    for recipe in &pack.recipes {
        builder.require_non_empty(
            format!("material_pack.recipes.{}", recipe.material_id),
            &recipe.material_id,
            "empty_material_id",
        );
        if !material_ids.insert(recipe.material_id.as_str()) {
            builder.block(
                Some(format!("material_pack.recipes.{}", recipe.material_id)),
                "duplicate_material_id",
                "Material recipe IDs must be unique.",
            );
        }
        builder.require_non_empty(
            format!("material_pack.recipes.{}.display_name", recipe.material_id),
            &recipe.display_name,
            "empty_material_display_name",
        );
        validate_unit_scalar(builder, &recipe.material_id, "metallic", recipe.metallic);
        validate_unit_scalar(builder, &recipe.material_id, "roughness", recipe.roughness);
        builder.require_non_empty(
            format!("material_pack.recipes.{}.wear_policy", recipe.material_id),
            &recipe.wear_policy,
            "empty_wear_policy",
        );
        if !recipe.texture_payload_ready {
            builder.block(
                Some(format!(
                    "material_pack.recipes.{}.texture_payload_ready",
                    recipe.material_id
                )),
                "texture_payload_policy_only",
                "Material recipe is procedural metadata only; no texture payload is included.",
            );
        }
    }

    builder.require(
        "material_slot_bindings_present",
        !pack.slot_bindings.is_empty(),
        Some("material_pack.slot_bindings"),
        "missing_material_slot_bindings",
        "Material slot bindings are present.",
        "At least one material slot binding is required.",
    );
    let mut slot_ids = BTreeSet::new();
    for binding in &pack.slot_bindings {
        builder.require_non_empty(
            format!("material_pack.slot_bindings.{}", binding.slot_id),
            &binding.slot_id,
            "empty_material_slot_id",
        );
        if !slot_ids.insert(binding.slot_id.as_str()) {
            builder.block(
                Some(format!("material_pack.slot_bindings.{}", binding.slot_id)),
                "duplicate_material_slot_id",
                "Material slot IDs must be unique.",
            );
        }
        if !material_ids.contains(binding.material_id.as_str()) {
            builder.block(
                Some(format!(
                    "material_pack.slot_bindings.{}.material_id",
                    binding.slot_id
                )),
                "unknown_material_reference",
                "Material slot bindings must reference an authored material recipe.",
            );
        }
        builder.require(
            format!("material_slot_{}_semantic_roles", binding.slot_id),
            !binding.semantic_roles.is_empty(),
            Some(format!(
                "material_pack.slot_bindings.{}.semantic_roles",
                binding.slot_id
            )),
            "material_slot_missing_semantic_roles",
            "Material slot binding records semantic roles.",
            "Material slot bindings must record semantic roles.",
        );
    }
}

fn validate_texture_requirements(
    builder: &mut SurfaceLabValidationReportBuilder,
    requirements: &TextureRequirementSet,
) {
    builder.require_non_empty(
        "texture_requirements.id",
        &requirements.id,
        "empty_texture_requirements_id",
    );
    builder.require(
        "texture_channels_present",
        !requirements.channels.is_empty(),
        Some("texture_requirements.channels"),
        "missing_texture_requirements",
        "Texture requirements are present.",
        "At least one texture channel requirement is required.",
    );
    let mut channels = BTreeSet::new();
    for requirement in &requirements.channels {
        if !channels.insert(requirement.channel) {
            builder.block(
                Some("texture_requirements.channels"),
                "duplicate_texture_channel",
                "Texture channels must be listed once.",
            );
        }
        builder.require(
            format!("texture_channel_{:?}_resolution", requirement.channel),
            requirement.resolution_px > 0,
            Some("texture_requirements.channels.resolution_px"),
            "invalid_texture_resolution",
            "Texture requirement resolution is positive.",
            "Texture requirement resolution must be positive.",
        );
        builder.require_non_empty(
            format!("texture_channel_{:?}_explanation", requirement.channel),
            &requirement.explanation,
            "empty_texture_requirement_explanation",
        );
        if requirement.required_for_texture_ready
            && requirement.status != StaticPropFeatureStatus::Ready
        {
            builder.block(
                Some(format!(
                    "texture_requirements.channels.{:?}",
                    requirement.channel
                )),
                requirement
                    .blocker_code
                    .as_deref()
                    .unwrap_or("texture_channel_not_ready"),
                "Required texture channel is not ready.",
            );
        }
    }
}

fn validate_unsupported_outputs(
    builder: &mut SurfaceLabValidationReportBuilder,
    report: &UnsupportedSurfaceOutputReport,
) {
    let mut outputs = BTreeSet::new();
    for output in &report.outputs {
        builder.require_non_empty(
            format!("unsupported_outputs.{}", output.output),
            &output.output,
            "empty_unsupported_output",
        );
        if !outputs.insert(output.output.as_str()) {
            builder.block(
                Some(format!("unsupported_outputs.{}", output.output)),
                "duplicate_unsupported_output",
                "Unsupported outputs must be listed once.",
            );
        }
        builder.require_non_empty(
            format!("unsupported_outputs.{}.blocker_code", output.output),
            &output.blocker_code,
            "empty_unsupported_output_blocker",
        );
        builder.require_non_empty(
            format!("unsupported_outputs.{}.explanation", output.output),
            &output.explanation,
            "empty_unsupported_output_explanation",
        );
        builder.block(
            Some(format!("unsupported_outputs.{}", output.output)),
            output.blocker_code.clone(),
            "Unsupported surface outputs prevent a surface-ready claim.",
        );
    }
}

fn validate_evidence(
    builder: &mut SurfaceLabValidationReportBuilder,
    evidence: &SurfaceEvidence,
    artifacts: SurfaceArtifactValidation<'_>,
) {
    builder.require_non_empty(
        "evidence.swatch_sheet",
        &evidence.swatch_sheet,
        "missing_swatch_sheet",
    );
    builder.require_non_empty(
        "evidence.validation_report",
        &evidence.validation_report,
        "missing_surface_validation_report",
    );
    validate_evidence_path_only(
        builder,
        "evidence.validation_report",
        &evidence.validation_report,
    );
    let SurfaceArtifactValidation::Filesystem(root) = artifacts else {
        builder.block(
            Some("evidence"),
            "surface_artifacts_not_verified",
            "Surface evidence paths were not checked against a package directory.",
        );
        return;
    };
    validate_evidence_file(
        builder,
        root,
        "evidence.swatch_sheet",
        &evidence.swatch_sheet,
        true,
    );
}

fn validate_evidence_path_only(
    builder: &mut SurfaceLabValidationReportBuilder,
    subject: &str,
    relative_path: &str,
) {
    if !is_portable_relative_path(Path::new(relative_path)) {
        builder.block(
            Some(subject.to_owned()),
            "surface_artifact_path_not_portable",
            "Surface evidence paths must be portable package-relative paths.",
        );
    }
}

fn validate_evidence_file(
    builder: &mut SurfaceLabValidationReportBuilder,
    root: &Path,
    subject: &str,
    relative_path: &str,
    png: bool,
) {
    let path = Path::new(relative_path);
    if !is_portable_relative_path(path) {
        builder.block(
            Some(subject.to_owned()),
            "surface_artifact_path_not_portable",
            "Surface evidence paths must be portable package-relative paths.",
        );
        return;
    }
    let resolved = root.join(path);
    builder.require(
        format!("{}_exists", subject.replace('.', "_")),
        resolved.is_file(),
        Some(subject.to_owned()),
        "surface_artifact_missing",
        "Surface evidence file exists.",
        "Surface evidence file is missing.",
    );
    if let Ok(bytes) = fs::read(&resolved) {
        let content_valid = if png {
            bytes.starts_with(b"\x89PNG\r\n\x1a\n")
        } else {
            serde_json::from_slice::<serde_json::Value>(&bytes).is_ok()
        };
        builder.require(
            format!("{}_content_valid", subject.replace('.', "_")),
            content_valid,
            Some(subject.to_owned()),
            "surface_artifact_content_invalid",
            "Surface evidence file content has the expected lightweight signature.",
            "Surface evidence file content is not valid.",
        );
    }
}

fn validate_path_exists_any(
    builder: &mut SurfaceLabValidationReportBuilder,
    root: &Path,
    subject: &str,
    relative_path: &str,
) {
    let path = Path::new(relative_path);
    if !is_portable_relative_path(path) {
        builder.block(
            Some(subject.to_owned()),
            "surface_artifact_path_not_portable",
            "Surface artifact paths must be portable package-relative paths.",
        );
        return;
    }
    let resolved = root.join(path);
    builder.require(
        format!("{}_exists", subject.replace('.', "_")),
        resolved.exists(),
        Some(subject.to_owned()),
        "surface_artifact_missing",
        "Surface artifact path exists.",
        "Surface artifact path is missing.",
    );
}

fn validate_json_file(
    builder: &mut SurfaceLabValidationReportBuilder,
    root: &Path,
    subject: &str,
    relative_path: &str,
) {
    let path = Path::new(relative_path);
    if !is_portable_relative_path(path) {
        builder.block(
            Some(subject.to_owned()),
            "surface_artifact_path_not_portable",
            "Surface artifact paths must be portable package-relative paths.",
        );
        return;
    }
    let resolved = root.join(path);
    builder.require(
        format!("{}_exists", subject.replace('.', "_")),
        resolved.is_file(),
        Some(subject.to_owned()),
        "surface_artifact_missing",
        "Surface evidence file exists.",
        "Surface evidence file is missing.",
    );
    if let Ok(bytes) = fs::read(&resolved) {
        builder.require(
            format!("{}_content_valid", subject.replace('.', "_")),
            serde_json::from_slice::<serde_json::Value>(&bytes).is_ok(),
            Some(subject.to_owned()),
            "surface_artifact_content_invalid",
            "Surface evidence JSON parses.",
            "Surface evidence JSON is invalid.",
        );
    }
}

fn validate_png_file_with_dimensions(
    builder: &mut SurfaceLabValidationReportBuilder,
    root: &Path,
    subject: &str,
    relative_path: &str,
    expected_dimensions: Option<(u32, u32)>,
) {
    let path = Path::new(relative_path);
    if !is_portable_relative_path(path) {
        builder.block(
            Some(subject.to_owned()),
            "surface_artifact_path_not_portable",
            "Surface artifact paths must be portable package-relative paths.",
        );
        return;
    }
    let resolved = root.join(path);
    builder.require(
        format!("{}_exists", subject.replace('.', "_")),
        resolved.is_file(),
        Some(subject.to_owned()),
        "surface_artifact_missing",
        "Surface PNG file exists.",
        "Surface PNG file is missing.",
    );
    let Ok(bytes) = fs::read(&resolved) else {
        return;
    };
    let dimensions = png_dimensions(&bytes);
    builder.require(
        format!("{}_png_valid", subject.replace('.', "_")),
        dimensions.is_some(),
        Some(subject.to_owned()),
        "surface_artifact_content_invalid",
        "Surface PNG has a valid signature and IHDR.",
        "Surface PNG content is invalid.",
    );
    if let (Some(actual), Some(expected)) = (dimensions, expected_dimensions) {
        builder.require(
            format!("{}_dimensions_match", subject.replace('.', "_")),
            actual == expected,
            Some(subject.to_owned()),
            "surface_texture_dimension_mismatch",
            "Surface texture dimensions match metadata.",
            "Surface texture dimensions do not match metadata.",
        );
    }
}

fn png_dimensions(bytes: &[u8]) -> Option<(u32, u32)> {
    if bytes.len() < 24 || !bytes.starts_with(b"\x89PNG\r\n\x1a\n") {
        return None;
    }
    if bytes.get(12..16) != Some(b"IHDR".as_slice()) {
        return None;
    }
    let width = u32::from_be_bytes(bytes.get(16..20)?.try_into().ok()?);
    let height = u32::from_be_bytes(bytes.get(20..24)?.try_into().ok()?);
    if width == 0 || height == 0 {
        None
    } else {
        Some((width, height))
    }
}

fn validate_unit_scalar(
    builder: &mut SurfaceLabValidationReportBuilder,
    material_id: &str,
    field: &str,
    value: f32,
) {
    builder.require(
        format!("material_{material_id}_{field}_unit"),
        value.is_finite() && (0.0..=1.0).contains(&value),
        Some(format!("material_pack.recipes.{material_id}.{field}")),
        format!("invalid_material_{field}"),
        format!("Material {field} is a finite unit value."),
        format!("Material {field} must be a finite unit value."),
    );
}

fn is_portable_relative_path(path: &Path) -> bool {
    !path.as_os_str().is_empty()
        && path
            .components()
            .all(|component| matches!(component, Component::Normal(_) | Component::CurDir))
}

#[derive(Debug, Copy, Clone)]
enum SurfaceArtifactValidation<'a> {
    Unverified,
    Filesystem(&'a Path),
}

struct SurfaceLabValidationReportBuilder {
    profile_id: String,
    checks: Vec<SurfaceLabValidationCheck>,
    blockers: Vec<GameAssetValidationIssue>,
    warnings: Vec<GameAssetValidationIssue>,
}

impl SurfaceLabValidationReportBuilder {
    fn new(profile_id: &str) -> Self {
        Self {
            profile_id: profile_id.to_owned(),
            checks: Vec::new(),
            blockers: Vec::new(),
            warnings: Vec::new(),
        }
    }

    fn finish(self) -> SurfaceLabValidationReport {
        let status = if self.blockers.is_empty() {
            SurfaceLabStatus::Ready
        } else {
            SurfaceLabStatus::Blocked
        };
        SurfaceLabValidationReport {
            schema_version: SURFACE_LAB_VALIDATION_REPORT_SCHEMA_VERSION,
            profile_id: self.profile_id,
            status,
            checks: self.checks,
            blockers: self.blockers,
            warnings: self.warnings,
        }
    }

    fn require(
        &mut self,
        code: impl Into<String>,
        condition: bool,
        subject: Option<impl Into<String>>,
        issue_code: impl Into<String>,
        pass_message: impl Into<String>,
        fail_message: impl Into<String>,
    ) {
        let code = code.into();
        if condition {
            self.checks.push(SurfaceLabValidationCheck {
                code,
                passed: true,
                message: pass_message.into(),
            });
        } else {
            let message = fail_message.into();
            self.checks.push(SurfaceLabValidationCheck {
                code,
                passed: false,
                message: message.clone(),
            });
            self.blockers.push(GameAssetValidationIssue {
                subject: subject.map(Into::into),
                code: issue_code.into(),
                message,
            });
        }
    }

    fn require_non_empty(
        &mut self,
        subject: impl Into<String>,
        value: &str,
        issue_code: impl Into<String>,
    ) {
        let subject = subject.into();
        self.require(
            format!("{}_present", subject.replace('.', "_")),
            !value.trim().is_empty(),
            Some(subject.clone()),
            issue_code,
            format!("{subject} is present."),
            format!("{subject} must be present."),
        );
    }

    fn block(
        &mut self,
        subject: Option<impl Into<String>>,
        code: impl Into<String>,
        message: impl Into<String>,
    ) {
        let code = code.into();
        let message = message.into();
        self.checks.push(SurfaceLabValidationCheck {
            code: code.clone(),
            passed: false,
            message: message.clone(),
        });
        self.blockers.push(GameAssetValidationIssue {
            subject: subject.map(Into::into),
            code,
            message,
        });
    }

    fn warn(
        &mut self,
        subject: Option<impl Into<String>>,
        code: impl Into<String>,
        message: impl Into<String>,
    ) {
        self.warnings.push(GameAssetValidationIssue {
            subject: subject.map(Into::into),
            code: code.into(),
            message: message.into(),
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn surface_lab_blocks_policy_only_uv_and_required_textures() {
        let package = valid_surface_package();

        let report = validate_surface_lab_package(&package);
        let blocker_codes = report
            .blockers
            .iter()
            .map(|issue| issue.code.as_str())
            .collect::<Vec<_>>();

        assert_eq!(report.status, SurfaceLabStatus::Blocked);
        assert!(blocker_codes.contains(&"uv_layout_not_implemented"));
        assert!(blocker_codes.contains(&"base_color_texture_not_authored"));
        assert!(blocker_codes.contains(&"texture_payload_policy_only"));
        assert!(blocker_codes.contains(&"surface_artifacts_not_verified"));
    }

    #[test]
    fn surface_lab_validates_evidence_files() {
        let package = valid_surface_package();
        let temp = tempfile::tempdir().expect("tempdir");
        fs::write(
            temp.path().join("swatches.png"),
            b"\x89PNG\r\n\x1a\nfixture",
        )
        .expect("swatch");
        fs::write(
            temp.path().join("surface-validation-report.json"),
            br#"{"schema_version":1}"#,
        )
        .expect("report");

        let report = validate_surface_lab_package_with_root(&package, temp.path());

        assert!(
            !report
                .blockers
                .iter()
                .any(|issue| issue.code == "surface_artifact_missing"),
            "{report:#?}"
        );
    }

    #[test]
    fn surface_artifact_round_trips_and_serializes_deterministically() {
        let artifact = valid_surface_artifact();

        let first = serde_json::to_string_pretty(&artifact).expect("first serialize");
        let second = serde_json::to_string_pretty(&artifact).expect("second serialize");
        let decoded: SurfaceArtifact = serde_json::from_str(&first).expect("decode");

        assert_eq!(first, second);
        assert_eq!(decoded, artifact);
    }

    #[test]
    fn surface_artifact_rejects_non_finite_uvs() {
        let mut artifact = valid_surface_artifact();
        artifact.uv_sets[0].coordinates[0][0] = f32::NAN;

        let report = validate_surface_artifact(&artifact);

        assert!(
            report
                .blockers
                .iter()
                .any(|issue| issue.code == "non_finite_uv_coordinate"),
            "{report:#?}"
        );
    }

    #[test]
    fn surface_artifact_rejects_missing_ready_uv_set() {
        let mut artifact = valid_surface_artifact();
        artifact.uv_sets.clear();

        let report = validate_surface_artifact(&artifact);

        assert!(
            report
                .blockers
                .iter()
                .any(|issue| issue.code == "missing_uv_sets"),
            "{report:#?}"
        );
    }

    #[test]
    fn surface_artifact_rejects_missing_texture_file() {
        let artifact = valid_surface_artifact();
        let temp = tempfile::tempdir().expect("tempdir");
        write_surface_artifact_evidence(temp.path(), false);

        let report = validate_surface_artifact_with_root(&artifact, temp.path());

        assert!(
            report
                .blockers
                .iter()
                .any(|issue| issue.code == "surface_artifact_missing"),
            "{report:#?}"
        );
    }

    #[test]
    fn surface_artifact_rejects_missing_required_ready_texture_channel() {
        let mut artifact = valid_surface_artifact();
        artifact.texture_sets[0]
            .files
            .retain(|file| file.channel != TextureChannel::Normal);

        let report = validate_surface_artifact(&artifact);

        assert!(
            report
                .blockers
                .iter()
                .any(|issue| issue.code == "missing_required_surface_texture_channel"),
            "{report:#?}"
        );
    }

    #[test]
    fn surface_artifact_rejects_missing_texture_set_for_used_recipe() {
        let mut artifact = valid_surface_artifact();
        artifact.texture_sets[0].material_recipe_id = "unused-recipe".to_owned();

        let report = validate_surface_artifact(&artifact);

        assert!(
            report
                .blockers
                .iter()
                .any(|issue| issue.code == "missing_surface_texture_set_for_recipe"),
            "{report:#?}"
        );
    }

    #[test]
    fn surface_artifact_rejects_material_coverage_mismatch() {
        let mut artifact = valid_surface_artifact();
        artifact.material_slots[0].coverage_triangle_count = 99;

        let report = validate_surface_artifact(&artifact);

        assert!(
            report.blockers.iter().any(|issue| {
                issue.code == "surface_material_coverage_mismatch"
                    || issue.code == "surface_material_slot_coverage_count_mismatch"
            }),
            "{report:#?}"
        );
    }

    #[test]
    fn unsupported_surface_output_cannot_pass() {
        let package = valid_surface_package();

        let report = validate_surface_lab_package(&package);

        assert!(
            report
                .blockers
                .iter()
                .any(|issue| issue.code == "texture_baking_not_implemented"),
            "{report:#?}"
        );
    }

    #[test]
    fn manual_surface_review_is_required_for_final_claim() {
        let mut artifact = valid_surface_artifact();
        artifact.manual_review = SurfaceReviewStatus::AutomatedReady;
        let temp = tempfile::tempdir().expect("tempdir");
        write_surface_artifact_evidence(temp.path(), true);

        let report = validate_surface_artifact_with_root(&artifact, temp.path());

        assert!(
            report
                .blockers
                .iter()
                .any(|issue| issue.code == "surface_manual_review_required"),
            "{report:#?}"
        );
    }

    #[test]
    fn surface_delta_rejects_shape_delta_leak() {
        let base = valid_surface_artifact();
        let mut candidate = base.clone();
        candidate.source_artifact_fingerprint = "artifact:changed".to_owned();

        let report = surface_visual_delta_report(
            &base,
            &candidate,
            "changed-shape",
            SurfaceVisualDeltaEvidence {
                base_frozen_mesh_fingerprint: "mesh:a".to_owned(),
                candidate_frozen_mesh_fingerprint: "mesh:b".to_owned(),
                base_textured_preview_ref: Some("surface/textured-preview.png".to_owned()),
                candidate_textured_preview_ref: Some(
                    "surface/variants/changed/textured-preview.png".to_owned(),
                ),
                visible_surface_pixel_delta: Some(0.4),
            },
        );

        assert!(report.shape_delta_leak_detected);
        assert_eq!(
            report.result_class,
            SurfaceVisualDeltaResultClass::Unsupported
        );
        assert!(!report.can_pass_surface_delta());
    }

    #[test]
    fn surface_delta_requires_textured_preview_evidence() {
        let base = valid_surface_artifact();
        let mut candidate = base.clone();
        candidate.texture_sets[0].files[0].path =
            "surface/variants/clean-lab-white/textures/worn-painted-sci-fi-metal-base_color.png"
                .to_owned();

        let report = surface_visual_delta_report(
            &base,
            &candidate,
            "clean-lab-white",
            SurfaceVisualDeltaEvidence {
                base_frozen_mesh_fingerprint: "mesh:a".to_owned(),
                candidate_frozen_mesh_fingerprint: "mesh:a".to_owned(),
                base_textured_preview_ref: None,
                candidate_textured_preview_ref: Some(
                    "surface/variants/clean-lab-white/textured-preview.png".to_owned(),
                ),
                visible_surface_pixel_delta: None,
            },
        );

        assert_eq!(
            report.result_class,
            SurfaceVisualDeltaResultClass::Unsupported
        );
        assert!(
            report
                .diagnostics
                .contains(&"missing_textured_preview_evidence".to_owned())
        );
        assert!(!report.can_pass_surface_delta());
    }

    #[test]
    fn surface_delta_classifies_material_only_variant() {
        let base = valid_surface_artifact();
        let mut candidate = base.clone();
        candidate.texture_sets[0].files[0].path =
            "surface/variants/worn-hazard-yellow/textures/worn-painted-sci-fi-metal-base_color.png"
                .to_owned();

        let report = surface_visual_delta_report(
            &base,
            &candidate,
            "worn-hazard-yellow",
            SurfaceVisualDeltaEvidence {
                base_frozen_mesh_fingerprint: "mesh:a".to_owned(),
                candidate_frozen_mesh_fingerprint: "mesh:a".to_owned(),
                base_textured_preview_ref: Some("surface/textured-preview.png".to_owned()),
                candidate_textured_preview_ref: Some(
                    "surface/variants/worn-hazard-yellow/textured-preview.png".to_owned(),
                ),
                visible_surface_pixel_delta: Some(0.22),
            },
        );

        assert!(!report.shape_delta_leak_detected);
        assert!(matches!(
            report.result_class,
            SurfaceVisualDeltaResultClass::Clear | SurfaceVisualDeltaResultClass::Strong
        ));
        assert!(report.can_pass_surface_delta(), "{report:#?}");
    }

    #[test]
    fn material_variant_candidate_set_validates_count_and_geometry_preservation() {
        let candidates = (0..6)
            .map(|index| SurfaceMaterialVariantCandidate {
                candidate_id: format!("variant-{index}"),
                title: format!("Variant {index}"),
                display_name: format!("Variant {index}"),
                summary: "Material-only variant preserving frozen geometry.".to_owned(),
                variant_dir: format!("surface/variants/variant-{index}"),
                changed_material_slots: vec!["painted_metal_body".to_owned()],
                surface_artifact_ref: format!(
                    "surface/variants/variant-{index}/surface-artifact.json"
                ),
                material_pack_ref: format!("surface/variants/variant-{index}/material-pack.json"),
                textured_preview_ref: format!(
                    "surface/variants/variant-{index}/textured-preview.png"
                ),
                preview_path: format!("surface/variants/variant-{index}/textured-preview.png"),
                surface_delta_ref: format!("surface/variants/variant-{index}/surface-delta.json"),
                surface_delta: SurfaceVisualDeltaReport {
                    schema_version: SURFACE_VISUAL_DELTA_REPORT_SCHEMA_VERSION,
                    profile_id: "sci-fi-crate".to_owned(),
                    candidate_id: format!("variant-{index}"),
                    base_color_delta: 0.2,
                    material_slot_delta: 0.0,
                    texture_channel_delta: 0.2,
                    wear_delta: Some(0.25),
                    visible_surface_pixel_delta: 0.2,
                    shape_delta_leak_detected: false,
                    result_class: SurfaceVisualDeltaResultClass::Clear,
                    diagnostics: Vec::new(),
                },
                texture_files: vec![format!(
                    "surface/variants/variant-{index}/textures/body-base_color.png"
                )],
                frozen_mesh_fingerprint: "mesh:a".to_owned(),
                preserves_frozen_geometry: true,
                full_ready_status: SurfaceLabStatus::Blocked,
                blocked_full_ready: true,
                result_class: SurfaceVisualDeltaResultClass::Clear,
                diagnostics: Vec::new(),
            })
            .collect::<Vec<_>>();
        let set = SurfaceMaterialVariantCandidateSet {
            schema_version: SURFACE_MATERIAL_VARIANT_CANDIDATES_SCHEMA_VERSION,
            profile_id: "sci-fi-crate".to_owned(),
            base_surface_artifact_ref: "surface/surface-artifact.json".to_owned(),
            base_textured_preview_ref: "surface/textured-preview.png".to_owned(),
            candidates,
        };

        assert!(validate_surface_material_variant_candidate_set(&set).is_ready());
    }

    #[test]
    fn material_variant_candidate_set_rejects_duplicate_looking_variants() {
        let mut set = SurfaceMaterialVariantCandidateSet {
            schema_version: SURFACE_MATERIAL_VARIANT_CANDIDATES_SCHEMA_VERSION,
            profile_id: "sci-fi-crate".to_owned(),
            base_surface_artifact_ref: "surface/surface-artifact.json".to_owned(),
            base_textured_preview_ref: "surface/textured-preview.png".to_owned(),
            candidates: (0..6)
                .map(|index| SurfaceMaterialVariantCandidate {
                    candidate_id: format!("variant-{index}"),
                    title: format!("Variant {index}"),
                    display_name: format!("Variant {index}"),
                    summary: "Material-only variant preserving frozen geometry.".to_owned(),
                    variant_dir: format!("surface/variants/variant-{index}"),
                    changed_material_slots: vec!["painted_metal_body".to_owned()],
                    surface_artifact_ref: format!(
                        "surface/variants/variant-{index}/surface-artifact.json"
                    ),
                    material_pack_ref: format!(
                        "surface/variants/variant-{index}/material-pack.json"
                    ),
                    textured_preview_ref: format!(
                        "surface/variants/variant-{index}/textured-preview.png"
                    ),
                    preview_path: format!("surface/variants/variant-{index}/textured-preview.png"),
                    surface_delta_ref: format!(
                        "surface/variants/variant-{index}/surface-delta.json"
                    ),
                    surface_delta: SurfaceVisualDeltaReport {
                        schema_version: SURFACE_VISUAL_DELTA_REPORT_SCHEMA_VERSION,
                        profile_id: "sci-fi-crate".to_owned(),
                        candidate_id: format!("variant-{index}"),
                        base_color_delta: 0.2,
                        material_slot_delta: 0.0,
                        texture_channel_delta: 0.2,
                        wear_delta: Some(0.25),
                        visible_surface_pixel_delta: 0.2,
                        shape_delta_leak_detected: false,
                        result_class: SurfaceVisualDeltaResultClass::Clear,
                        diagnostics: Vec::new(),
                    },
                    texture_files: vec![format!(
                        "surface/variants/variant-{index}/textures/body-base_color.png"
                    )],
                    frozen_mesh_fingerprint: "mesh:a".to_owned(),
                    preserves_frozen_geometry: true,
                    full_ready_status: SurfaceLabStatus::Blocked,
                    blocked_full_ready: true,
                    result_class: SurfaceVisualDeltaResultClass::Clear,
                    diagnostics: Vec::new(),
                })
                .collect(),
        };
        set.candidates[0].result_class = SurfaceVisualDeltaResultClass::DuplicateLooking;
        set.candidates[0].surface_delta.result_class =
            SurfaceVisualDeltaResultClass::DuplicateLooking;
        set.candidates[0]
            .diagnostics
            .push("duplicate_looking_surface_variant".to_owned());

        let report = validate_surface_material_variant_candidate_set(&set);

        assert!(
            report
                .blockers
                .iter()
                .any(|issue| issue.code == "duplicate_looking_surface_variant"),
            "{report:#?}"
        );
    }

    fn valid_surface_package() -> SurfaceLabPackage {
        SurfaceLabPackage {
            schema_version: SURFACE_LAB_PACKAGE_SCHEMA_VERSION,
            profile_id: "sci-fi-crate".to_owned(),
            display_name: "Sci-Fi Crate".to_owned(),
            asset_family: "Static Prop".to_owned(),
            uv_layout: SurfaceUvLayoutPolicy {
                status: StaticPropFeatureStatus::NotImplemented,
                required_for_texture_ready: true,
                policy: "box/projected regions only".to_owned(),
                blocker_code: Some("uv_layout_not_implemented".to_owned()),
                explanation: "UV layout is not implemented.".to_owned(),
            },
            material_pack: MaterialStylePack {
                id: "sci-fi-crate-industrial-surfaces-v0".to_owned(),
                display_name: "Industrial Sci-Fi Surface Recipes".to_owned(),
                texture_packing_policy: "Future ORM packing: occlusion, roughness, metallic."
                    .to_owned(),
                recipes: vec![MaterialRecipe {
                    material_id: "worn-painted-metal".to_owned(),
                    display_name: "Worn Painted Metal".to_owned(),
                    base_color_srgb: [116, 124, 112],
                    metallic: 0.0,
                    roughness: 0.72,
                    wear_policy: "Edge wear mask policy only.".to_owned(),
                    texture_payload_ready: false,
                }],
                slot_bindings: vec![MaterialSlotBinding {
                    slot_id: "painted_metal_body".to_owned(),
                    display_name: "Painted Metal Body".to_owned(),
                    material_id: "worn-painted-metal".to_owned(),
                    semantic_roles: vec!["body".to_owned()],
                }],
            },
            texture_requirements: TextureRequirementSet {
                id: "sci-fi-crate-texture-requirements-v0".to_owned(),
                channels: vec![TextureRequirement {
                    channel: TextureChannel::BaseColor,
                    resolution_px: 1024,
                    required_for_texture_ready: true,
                    status: StaticPropFeatureStatus::NotImplemented,
                    blocker_code: Some("base_color_texture_not_authored".to_owned()),
                    explanation: "Base color texture is required later.".to_owned(),
                }],
            },
            unsupported_outputs: UnsupportedSurfaceOutputReport {
                outputs: vec![UnsupportedSurfaceOutput {
                    output: "baked-texture-set".to_owned(),
                    blocker_code: "texture_baking_not_implemented".to_owned(),
                    explanation: "Texture baking is not implemented.".to_owned(),
                }],
            },
            evidence: SurfaceEvidence {
                swatch_sheet: "swatches.png".to_owned(),
                validation_report: "surface-validation-report.json".to_owned(),
            },
        }
    }

    fn valid_surface_artifact() -> SurfaceArtifact {
        SurfaceArtifact {
            schema_version: SURFACE_ARTIFACT_SCHEMA_VERSION,
            profile_id: "sci-fi-crate".to_owned(),
            display_name: "Sci-Fi Crate".to_owned(),
            source_artifact_fingerprint: "artifact:abc".to_owned(),
            source_recipe_hash: 42,
            frozen_mesh_ref: "model-package".to_owned(),
            uv_sets: vec![SurfaceUvSet {
                id: "uv0".to_owned(),
                display_name: "UV0".to_owned(),
                channel_index: 0,
                coordinate_count: 3,
                coordinates: vec![[0.0, 0.0], [1.0, 0.0], [0.0, 1.0]],
                source_policy: "test projection".to_owned(),
                readiness_status: StaticPropFeatureStatus::Ready,
                tiling_allowed: false,
            }],
            material_slots: vec![SurfaceMaterialSlot {
                slot_id: "painted_metal_body".to_owned(),
                display_name: "Painted Metal Body".to_owned(),
                semantic_roles: vec!["body".to_owned()],
                recipe_id: "worn-painted-sci-fi-metal".to_owned(),
                coverage_triangle_count: 1,
                coverage_fraction: 1.0,
            }],
            texture_sets: vec![SurfaceTextureSet {
                id: "worn-painted-sci-fi-metal-texture-set-v1".to_owned(),
                display_name: "Worn Painted Metal Texture Set".to_owned(),
                material_recipe_id: "worn-painted-sci-fi-metal".to_owned(),
                files: vec![
                    SurfaceTextureFile {
                        channel: TextureChannel::BaseColor,
                        path: "surface/textures/worn-painted-sci-fi-metal-base_color.png"
                            .to_owned(),
                        width: 2,
                        height: 2,
                        color_space: "sRGB".to_owned(),
                        required_for_texture_ready: true,
                    },
                    SurfaceTextureFile {
                        channel: TextureChannel::MetallicRoughness,
                        path: "surface/textures/worn-painted-sci-fi-metal-metallic_roughness.png"
                            .to_owned(),
                        width: 2,
                        height: 2,
                        color_space: "linear".to_owned(),
                        required_for_texture_ready: true,
                    },
                    SurfaceTextureFile {
                        channel: TextureChannel::Normal,
                        path: "surface/textures/worn-painted-sci-fi-metal-normal.png".to_owned(),
                        width: 2,
                        height: 2,
                        color_space: "linear".to_owned(),
                        required_for_texture_ready: true,
                    },
                    SurfaceTextureFile {
                        channel: TextureChannel::Occlusion,
                        path: "surface/textures/worn-painted-sci-fi-metal-occlusion.png".to_owned(),
                        width: 2,
                        height: 2,
                        color_space: "linear".to_owned(),
                        required_for_texture_ready: true,
                    },
                ],
                procedural_source: "test procedural source".to_owned(),
                payload_ready: true,
            }],
            triangle_bindings: vec![SurfaceTriangleBinding {
                triangle_index: 0,
                material_slot_id: "painted_metal_body".to_owned(),
                uv_set_id: "uv0".to_owned(),
                source_part: Some("compiled_part_001".to_owned()),
                source_region: None,
                source_operation: None,
            }],
            evidence: SurfaceArtifactEvidence {
                uv_layout: "surface/uv-layout.png".to_owned(),
                material_swatch_sheet: "surface/material-swatch-sheet.png".to_owned(),
                texture_contact_sheet: "surface/texture-contact-sheet.png".to_owned(),
                triangle_slot_coverage: "surface/triangle-slot-coverage.json".to_owned(),
            },
            validation_report_ref: "surface/surface-validation-report.json".to_owned(),
            manual_review: SurfaceReviewStatus::Approved,
        }
    }

    fn write_surface_artifact_evidence(root: &Path, include_textures: bool) {
        fs::create_dir_all(root.join("model-package")).expect("model dir");
        fs::create_dir_all(root.join("surface/textures")).expect("texture dir");
        for path in [
            "surface/uv-layout.png",
            "surface/material-swatch-sheet.png",
            "surface/texture-contact-sheet.png",
        ] {
            fs::write(root.join(path), png_fixture(8, 8)).expect("png");
        }
        fs::write(
            root.join("surface/triangle-slot-coverage.json"),
            br#"{"triangle_count":1}"#,
        )
        .expect("coverage");
        if include_textures {
            for path in [
                "surface/textures/worn-painted-sci-fi-metal-base_color.png",
                "surface/textures/worn-painted-sci-fi-metal-metallic_roughness.png",
                "surface/textures/worn-painted-sci-fi-metal-normal.png",
                "surface/textures/worn-painted-sci-fi-metal-occlusion.png",
            ] {
                fs::write(root.join(path), png_fixture(2, 2)).expect("texture");
            }
        }
    }

    fn png_fixture(width: u32, height: u32) -> Vec<u8> {
        let mut bytes = Vec::new();
        bytes.extend_from_slice(b"\x89PNG\r\n\x1a\n");
        bytes.extend_from_slice(&13_u32.to_be_bytes());
        bytes.extend_from_slice(b"IHDR");
        bytes.extend_from_slice(&width.to_be_bytes());
        bytes.extend_from_slice(&height.to_be_bytes());
        bytes.extend_from_slice(&[8, 6, 0, 0, 0]);
        bytes.extend_from_slice(&[0, 0, 0, 0]);
        bytes
    }
}
