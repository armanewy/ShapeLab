//! Geometry-only export contracts.
//!
//! V0 export is a narrow bridge from reviewed draft geometry to a GLB file.
//! It intentionally excludes material looks, UV claims, collision, rigging,
//! animation, public publishing, and game-ready status.

use serde::{Deserialize, Serialize};
use shape_compile::export::RelationshipRealizationSummary;

use crate::{MaterializationStatus, MaterializedObjectDraft, PrimitiveKind};

/// Request to export a supported source as a geometry-only asset package.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GeometryExportRequest {
    /// Source kind to export.
    pub source_kind: GeometryExportSourceKind,
    /// Stable source reference. CLI tools may resolve this to a file or draft ID.
    pub source_ref: String,
    /// Requested export format.
    pub export_format: GeometryExportFormat,
    /// V0 safety policy.
    pub export_policy: GeometryExportPolicy,
    /// Product-safe output stem.
    pub output_name: String,
    /// Output directory reference.
    pub output_dir: String,
}

/// Supported source kinds for geometry export.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub enum GeometryExportSourceKind {
    /// The primitive currently being edited.
    CurrentPrimitive,
    /// A validated ObjectPlan.
    ObjectPlan,
    /// A materialized ObjectPlan draft.
    MaterializedObjectDraft,
}

/// Supported export formats.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub enum GeometryExportFormat {
    /// Binary glTF 2.0 package.
    Glb,
    /// Directory-form glTF. Reserved for a later proof.
    GltfDirectory,
}

/// Safety policy for geometry-only export.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GeometryExportPolicy {
    /// V0 exports geometry only.
    pub geometry_only: bool,
    /// Source materialization must have passed before export.
    pub require_valid_materialization: bool,
    /// A single neutral material may be used only so geometry is visible.
    pub allow_placeholder_neutral_material: bool,
    /// Textures are forbidden.
    pub forbid_textures: bool,
    /// UV support claims are forbidden.
    pub forbid_uv_claims: bool,
    /// Rigging is forbidden.
    pub forbid_rigging: bool,
    /// Animation is forbidden.
    pub forbid_animation: bool,
    /// Collision/gameplay metadata claims are forbidden.
    pub forbid_collision_claims: bool,
    /// Game-ready claims are forbidden.
    pub forbid_game_ready_claims: bool,
}

impl Default for GeometryExportPolicy {
    fn default() -> Self {
        Self {
            geometry_only: true,
            require_valid_materialization: true,
            allow_placeholder_neutral_material: true,
            forbid_textures: true,
            forbid_uv_claims: true,
            forbid_rigging: true,
            forbid_animation: true,
            forbid_collision_claims: true,
            forbid_game_ready_claims: true,
        }
    }
}

/// Geometry export status.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub enum GeometryExportStatus {
    /// Export passed for the requested geometry-only scope.
    Passed,
    /// Export was blocked by an unsupported source or requested capability.
    Blocked,
    /// Export failed unexpectedly.
    Failed,
}

/// Structured report for geometry-only export.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GeometryExportReport {
    /// Export status.
    pub status: GeometryExportStatus,
    /// Relative output file references.
    pub output_files: Vec<String>,
    /// Optional source ObjectPlan ID.
    pub source_plan_id: Option<String>,
    /// Source primitive count.
    pub primitive_count: usize,
    /// Exported mesh count.
    pub mesh_count: usize,
    /// Exported triangle count.
    pub triangle_count: usize,
    /// Warning count.
    pub warning_count: usize,
    /// Blockers that prevented export.
    pub blockers: Vec<String>,
    /// How authored relationships were realized in this geometry export.
    pub relationship_realizations: Vec<RelationshipRealizationSummary>,
    /// Whether UV data is included.
    pub includes_uvs: bool,
    /// Whether textures are included.
    pub includes_textures: bool,
    /// Whether material looks are included.
    pub includes_material_looks: bool,
    /// Whether collision data is included.
    pub includes_collision: bool,
    /// Whether rig data is included.
    pub includes_rig: bool,
    /// Whether animation data is included.
    pub includes_animation: bool,
    /// V0 geometry export is never game-ready.
    pub game_ready: bool,
    /// Human review remains required.
    pub human_review_required: bool,
}

/// Product-facing summary for geometry-only export.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GeometryExportUserSummary {
    /// Short headline.
    pub title: String,
    /// Product-safe lines.
    pub lines: Vec<String>,
}

/// One geometry export validation issue.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GeometryExportValidationIssue {
    /// Stable subject path.
    pub subject: String,
    /// Stable issue code.
    pub code: String,
    /// Developer-facing message.
    pub message: String,
}

/// Validation report for geometry export contracts.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GeometryExportValidationReport {
    /// Issues discovered during validation.
    pub issues: Vec<GeometryExportValidationIssue>,
}

impl GeometryExportValidationReport {
    /// Return true when no issues were discovered.
    #[must_use]
    pub fn is_valid(&self) -> bool {
        self.issues.is_empty()
    }

    fn push(
        &mut self,
        subject: impl Into<String>,
        code: impl Into<String>,
        message: impl Into<String>,
    ) {
        self.issues.push(GeometryExportValidationIssue {
            subject: subject.into(),
            code: code.into(),
            message: message.into(),
        });
    }

    fn extend(&mut self, nested: GeometryExportValidationReport) {
        self.issues.extend(nested.issues);
    }
}

/// Validate a geometry export request.
#[must_use]
pub fn validate_geometry_export_request(
    request: &GeometryExportRequest,
) -> GeometryExportValidationReport {
    let mut report = GeometryExportValidationReport::default();

    if request.source_ref.trim().is_empty() {
        report.push(
            "source_ref",
            "missing_geometry_export_source_ref",
            "Geometry export requires a source reference.",
        );
    }
    if request.output_name.trim().is_empty() {
        report.push(
            "output_name",
            "missing_geometry_export_output_name",
            "Geometry export requires an output name.",
        );
    }
    if request.output_dir.trim().is_empty() {
        report.push(
            "output_dir",
            "missing_geometry_export_output_dir",
            "Geometry export requires an output directory.",
        );
    }
    if request.export_format == GeometryExportFormat::GltfDirectory {
        report.push(
            "export_format",
            "gltf_directory_export_not_supported_v0",
            "Directory glTF export is reserved for a later proof.",
        );
    }
    report.extend(validate_geometry_export_policy(&request.export_policy));

    report
}

/// Validate a V0 geometry export policy.
#[must_use]
pub fn validate_geometry_export_policy(
    policy: &GeometryExportPolicy,
) -> GeometryExportValidationReport {
    let mut report = GeometryExportValidationReport::default();

    if !policy.geometry_only {
        report.push(
            "export_policy.geometry_only",
            "geometry_export_must_be_geometry_only",
            "V0 export only supports geometry-only packages.",
        );
    }
    if !policy.require_valid_materialization {
        report.push(
            "export_policy.require_valid_materialization",
            "geometry_export_requires_valid_materialization",
            "V0 export requires a passed materialization.",
        );
    }
    if !policy.forbid_textures {
        report.push(
            "export_policy.forbid_textures",
            "geometry_export_textures_forbidden",
            "V0 export cannot include textures.",
        );
    }
    if !policy.forbid_uv_claims {
        report.push(
            "export_policy.forbid_uv_claims",
            "geometry_export_uv_claims_forbidden",
            "V0 export cannot claim UV support.",
        );
    }
    if !policy.forbid_rigging {
        report.push(
            "export_policy.forbid_rigging",
            "geometry_export_rigging_forbidden",
            "V0 export cannot include rigging.",
        );
    }
    if !policy.forbid_animation {
        report.push(
            "export_policy.forbid_animation",
            "geometry_export_animation_forbidden",
            "V0 export cannot include animation.",
        );
    }
    if !policy.forbid_collision_claims {
        report.push(
            "export_policy.forbid_collision_claims",
            "geometry_export_collision_claims_forbidden",
            "V0 export cannot include collision or gameplay metadata.",
        );
    }
    if !policy.forbid_game_ready_claims {
        report.push(
            "export_policy.forbid_game_ready_claims",
            "geometry_export_game_ready_claims_forbidden",
            "V0 export cannot claim game-ready status.",
        );
    }

    report
}

/// Validate a V0 geometry export report.
#[must_use]
pub fn validate_geometry_export_report(
    export_report: &GeometryExportReport,
) -> GeometryExportValidationReport {
    let mut report = GeometryExportValidationReport::default();

    if export_report.status == GeometryExportStatus::Passed && export_report.output_files.is_empty()
    {
        report.push(
            "output_files",
            "geometry_export_passed_without_output",
            "A passed geometry export report must list output files.",
        );
    }
    if export_report.status != GeometryExportStatus::Passed && export_report.blockers.is_empty() {
        report.push(
            "blockers",
            "geometry_export_blocked_without_reason",
            "Blocked or failed geometry exports must report blockers.",
        );
    }
    for (index, realization) in export_report.relationship_realizations.iter().enumerate() {
        if realization.baked {
            report.push(
                format!("relationship_realizations.{index}.baked"),
                "geometry_export_relationship_bake_unproven",
                "V0 geometry export cannot claim a baked relationship realization.",
            );
        }
    }
    if export_report.includes_uvs {
        report.push(
            "includes_uvs",
            "geometry_export_uv_claims_forbidden",
            "V0 export cannot claim UV support.",
        );
    }
    if export_report.includes_textures {
        report.push(
            "includes_textures",
            "geometry_export_textures_forbidden",
            "V0 export cannot include textures.",
        );
    }
    if export_report.includes_material_looks {
        report.push(
            "includes_material_looks",
            "geometry_export_material_looks_forbidden",
            "V0 export cannot include material looks.",
        );
    }
    if export_report.includes_collision {
        report.push(
            "includes_collision",
            "geometry_export_collision_claims_forbidden",
            "V0 export cannot include collision or gameplay metadata.",
        );
    }
    if export_report.includes_rig {
        report.push(
            "includes_rig",
            "geometry_export_rigging_forbidden",
            "V0 export cannot include rigging.",
        );
    }
    if export_report.includes_animation {
        report.push(
            "includes_animation",
            "geometry_export_animation_forbidden",
            "V0 export cannot include animation.",
        );
    }
    if export_report.game_ready {
        report.push(
            "game_ready",
            "geometry_export_game_ready_claims_forbidden",
            "V0 export cannot claim game-ready status.",
        );
    }
    if !export_report.human_review_required {
        report.push(
            "human_review_required",
            "geometry_export_review_required",
            "Geometry export still requires human review.",
        );
    }

    report
}

/// Return true when a primitive is inside the V0 geometry export scope.
#[must_use]
pub fn geometry_export_supports_primitive(primitive_kind: PrimitiveKind) -> bool {
    matches!(
        primitive_kind,
        PrimitiveKind::BoxPrimitive
            | PrimitiveKind::FlatPanelPrimitive
            | PrimitiveKind::SpherePrimitive
    )
}

/// Return export blockers for a materialized draft.
#[must_use]
pub fn geometry_export_blockers_for_materialized_draft(
    draft: &MaterializedObjectDraft,
) -> Vec<String> {
    let mut blockers = Vec::new();

    if draft.status != MaterializationStatus::Passed {
        blockers.push("Source materialization did not pass.".to_owned());
    }
    if !draft.unresolved_nodes.is_empty() {
        blockers.push("Source draft has unresolved primitive nodes.".to_owned());
    }
    if !draft.unresolved_attachments.is_empty() {
        blockers.push("Source draft has unresolved attachments.".to_owned());
    }
    for instance in &draft.primitive_instances {
        if !geometry_export_supports_primitive(instance.primitive_kind) {
            blockers.push(format!(
                "Primitive kind {:?} is not supported by geometry export V0.",
                instance.primitive_kind
            ));
        }
    }
    if draft.publish_allowed {
        blockers.push("Public catalog publishing is not allowed for geometry export.".to_owned());
    }
    if !draft.user_review_required {
        blockers.push("Geometry export requires human review to remain enabled.".to_owned());
    }

    blockers
}

/// Build a product-safe V0 geometry export summary.
#[must_use]
pub fn geometry_export_user_summary(
    export_report: &GeometryExportReport,
) -> GeometryExportUserSummary {
    if export_report.status == GeometryExportStatus::Passed {
        GeometryExportUserSummary {
            title: "Geometry export complete".to_owned(),
            lines: vec![
                "Geometry-only GLB exported.".to_owned(),
                "No textures, collision, rigging, or animation are included.".to_owned(),
                "Godot import proof is required before calling this Godot-ready.".to_owned(),
                "Human review is still required.".to_owned(),
            ],
        }
    } else {
        GeometryExportUserSummary {
            title: "Geometry export blocked".to_owned(),
            lines: vec![
                "Geometry-only GLB export was blocked.".to_owned(),
                "No textures, collision, rigging, or animation are included.".to_owned(),
                "Godot import proof is required before calling this Godot-ready.".to_owned(),
                "Resolve the blockers before reviewing the asset package.".to_owned(),
            ],
        }
    }
}
