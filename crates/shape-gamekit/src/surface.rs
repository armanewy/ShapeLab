#![forbid(unsafe_code)]

//! Surface Lab v0 contracts for static prop UV/material/texture readiness.
//!
//! These contracts are metadata-first. They let backend tools emit material
//! slots, procedural material recipes, texture requirements, unsupported-output
//! blockers, and swatch evidence without integrating unfinished controls into
//! the novice product UI.

use std::collections::BTreeSet;
use std::fs;
use std::path::{Component, Path};

use serde::{Deserialize, Serialize};

use crate::GameAssetValidationIssue;
use crate::export::StaticPropFeatureStatus;

/// Surface Lab package schema version.
pub const SURFACE_LAB_PACKAGE_SCHEMA_VERSION: u32 = 1;

/// Surface Lab validation report schema version.
pub const SURFACE_LAB_VALIDATION_REPORT_SCHEMA_VERSION: u32 = 1;

/// Surface readiness status.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SurfaceLabStatus {
    /// Automated checks found no blocker.
    Ready,
    /// One or more blockers prevent a surface-ready claim.
    Blocked,
}

/// One Surface Lab v0 package.
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
}
