//! Foundry Author Studio contracts for internal kit authors.
//!
//! The studio is a technical authoring lane over Foundry kit packages. It does
//! not change the novice Visual Foundry surface and it does not bypass kit
//! validation, quality gates, or review manifests.

use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};

use crate::{
    ControlProfileControlKind, ControlProfileTopologyBehavior, DEFAULT_MAX_PRIMARY_NOVICE_CONTROLS,
    FoundryKitPackage,
};

/// Current schema version for Author Studio descriptors.
pub const FOUNDRY_AUTHOR_STUDIO_SCHEMA_VERSION: u32 = 1;

/// Stable Author Studio workflow steps.
#[must_use]
pub fn foundry_author_studio_steps() -> Vec<AuthorStudioStep> {
    vec![
        AuthorStudioStep::new(1, "kit_overview", "Kit Overview"),
        AuthorStudioStep::new(2, "family_blueprint", "Family Blueprint"),
        AuthorStudioStep::new(3, "provider_packs", "Provider Packs"),
        AuthorStudioStep::new(4, "style_compatibility", "Style Compatibility"),
        AuthorStudioStep::new(5, "controls", "Controls"),
        AuthorStudioStep::new(6, "candidate_strategies", "Candidate Strategies"),
        AuthorStudioStep::new(7, "preview_cameras", "Preview Cameras"),
        AuthorStudioStep::new(8, "quality_gates", "Quality Gates"),
        AuthorStudioStep::new(9, "review_package", "Review & Package"),
    ]
}

/// One Author Studio workflow step.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AuthorStudioStep {
    /// 1-based step index.
    pub index: u8,
    /// Stable step ID.
    pub step_id: String,
    /// Author-facing label.
    pub label: String,
}

impl AuthorStudioStep {
    fn new(index: u8, step_id: impl Into<String>, label: impl Into<String>) -> Self {
        Self {
            index,
            step_id: step_id.into(),
            label: label.into(),
        }
    }
}

/// Explicit gate for internal/pro authoring surfaces.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FoundryAuthorStudioGate {
    /// Developer/pro UI flag supplied by the host.
    pub developer_ui_enabled: bool,
}

impl FoundryAuthorStudioGate {
    /// Default release behavior: Author Studio is not shown.
    #[must_use]
    pub const fn default_release() -> Self {
        Self {
            developer_ui_enabled: false,
        }
    }

    /// Explicit developer/pro behavior.
    #[must_use]
    pub const fn developer_enabled() -> Self {
        Self {
            developer_ui_enabled: true,
        }
    }
}

/// Minimal Author Studio shell view.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FoundryAuthorStudioShell {
    /// Whether the surface is available in the current host mode.
    pub available: bool,
    /// Why the surface is unavailable.
    pub unavailable_reason: Option<String>,
    /// Ordered workflow steps.
    pub steps: Vec<AuthorStudioStep>,
}

/// Build the gated Author Studio shell contract.
#[must_use]
pub fn foundry_author_studio_shell(gate: FoundryAuthorStudioGate) -> FoundryAuthorStudioShell {
    if gate.developer_ui_enabled {
        FoundryAuthorStudioShell {
            available: true,
            unavailable_reason: None,
            steps: foundry_author_studio_steps(),
        }
    } else {
        FoundryAuthorStudioShell {
            available: false,
            unavailable_reason: Some(
                "Foundry Author Studio requires explicit developer authoring mode.".to_owned(),
            ),
            steps: Vec::new(),
        }
    }
}

/// Role/slot descriptor edited by Author Studio.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AuthorRoleDescriptor {
    /// Stable role ID.
    pub role_id: String,
    /// Author-facing display name.
    pub display_name: String,
    /// Author-facing role description.
    pub description: String,
    /// Whether the role is required.
    pub required: bool,
    /// Whether multiple role occurrences are allowed.
    pub repeated: bool,
    /// Whether the part is normally product-visible.
    pub default_visibility: bool,
    /// Export part name used by package output.
    pub export_part_name: String,
}

/// Socket/port descriptor used by technical authoring.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SocketPortDescriptor {
    /// Stable socket ID within the provider.
    pub socket_id: String,
    /// Stable port ID paired with the socket.
    pub port_id: String,
    /// Local role or occurrence target.
    pub target_role: String,
    /// Compatibility tags required by the attachment.
    pub compatibility_tags: Vec<String>,
    /// Allowed attachment modes.
    pub allowed_attachment_modes: Vec<String>,
    /// Whether the attachment metadata is required.
    pub required: bool,
    /// Author-facing notes.
    pub author_notes: String,
}

/// Provider descriptor registration form.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProviderDescriptor {
    /// Stable provider descriptor ID.
    pub provider_id: String,
    /// Display name for authoring tools.
    pub display_name: String,
    /// Semantic role filled by the provider.
    pub semantic_role: String,
    /// Provider slot filled by the provider.
    pub provider_slot: String,
    /// Style/detail tags.
    pub tags: Vec<String>,
    /// Compatibility tags used by style/provider validation.
    pub compatibility_tags: Vec<String>,
    /// Approximate triangle budget.
    pub approximate_triangle_budget: Option<u32>,
    /// Socket/port requirements.
    pub socket_requirements: Vec<SocketPortDescriptor>,
    /// Whether preview output exists.
    pub preview_available: bool,
    /// Whether this is metadata-only rather than actual mesh/component import.
    pub descriptor_only: bool,
}

/// Style compatibility editor model.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StyleCompatibilityDescriptor {
    /// Compatible style pack IDs.
    pub compatible_style_packs: Vec<String>,
    /// Incompatible style pack IDs with author reasons.
    pub incompatible_style_packs: BTreeMap<String, String>,
    /// Provider tags allowed by the style.
    pub allowed_provider_tags: Vec<String>,
    /// Provider tags rejected by the style.
    pub forbidden_provider_tags: Vec<String>,
    /// Detail density policy notes.
    pub detail_density_policy: String,
    /// Bevel language notes.
    pub bevel_language_notes: String,
    /// Proportion language notes.
    pub proportion_language_notes: String,
    /// Symmetry/asymmetry policy.
    pub symmetry_asymmetry_policy: String,
}

/// Control mapping edited by Author Studio.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ControlMappingDescriptor {
    /// Stable control ID.
    pub control_id: String,
    /// Human-facing label.
    pub label: String,
    /// Human-facing description.
    pub description: String,
    /// Control kind.
    pub kind: ControlProfileControlKind,
    /// Whether this is a primary novice control.
    pub primary: bool,
    /// Whether this is visible in the control profile.
    pub visible: bool,
    /// Owned family slots.
    pub owned_family_slots: Vec<String>,
    /// Owned provider slots.
    pub owned_provider_slots: Vec<String>,
    /// Response curve descriptor.
    pub response_curve_descriptor: String,
    /// Discrete option labels or IDs.
    pub discrete_options: Vec<String>,
    /// Optional provider slot binding.
    pub provider_slot_binding: Option<String>,
    /// Topology behavior.
    pub topology_behavior: ControlProfileTopologyBehavior,
    /// Plain-language disabled reason policy.
    pub disabled_reason_policy: String,
}

/// Candidate strategy editor model.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CandidateStrategyDescriptor {
    /// Stable strategy ID.
    pub strategy_id: String,
    /// User-facing strategy name.
    pub name: String,
    /// Short user-facing explanation.
    pub explanation: String,
    /// Controls the strategy may change.
    pub allowed_controls: Vec<String>,
    /// Provider slot changes the strategy may make.
    pub allowed_provider_changes: Vec<String>,
    /// Intensity policy.
    pub intensity_policy: String,
    /// Diversity policy.
    pub diversity_policy: String,
    /// Lock-respect policy.
    pub lock_respect_policy: String,
    /// Rejection policy.
    pub rejection_policy: String,
    /// Explanation template.
    pub explanation_template: String,
}

/// One camera spec edited by Author Studio.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PreviewCameraDescriptor {
    /// Stable camera ID.
    pub camera_id: String,
    /// Author-facing label.
    pub label: String,
    /// View name such as front, side, back, or three-quarter.
    pub view: String,
    /// Fitted-scale policy.
    pub fitted_scale_policy: String,
    /// Lighting policy.
    pub lighting_policy: String,
    /// Whether this output is currently supported.
    pub supported: bool,
    /// Honest unsupported reason when not supported.
    pub unsupported_reason: Option<String>,
}

/// Per-control option-gallery camera policy.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OptionGalleryCameraPolicy {
    /// Control ID using this option-gallery policy.
    pub control_id: String,
    /// Camera IDs used by the options in display order.
    pub option_camera_ids: Vec<String>,
    /// Fitted-scale policy IDs used by the options in display order.
    pub option_fitted_scale_policies: Vec<String>,
}

/// Preview camera editor model.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PreviewCameraPolicyDescriptor {
    /// Default camera.
    pub default_camera: PreviewCameraDescriptor,
    /// Direction board camera.
    pub direction_board_camera: PreviewCameraDescriptor,
    /// Option-gallery camera.
    pub option_gallery_camera: PreviewCameraDescriptor,
    /// Required contact-sheet cameras.
    pub contact_sheet_cameras: Vec<PreviewCameraDescriptor>,
    /// Per-control option-gallery consistency policies.
    pub option_gallery_policies: Vec<OptionGalleryCameraPolicy>,
}

/// Existing quality/package CLI task exposed through Author Studio.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AuthorQualityGateTask {
    /// `shape-cli foundry-kit validate`.
    ValidateKit,
    /// `shape-cli foundry-kit preview`.
    RenderPreview,
    /// `shape-cli foundry-kit contact-sheet`.
    RenderContactSheet,
    /// `cargo test -p shape-foundry-catalog --test box_primitive --jobs 1`.
    BoxPrimitiveGate,
    /// `shape-cli foundry-kit review`.
    ProduceReviewManifest,
    /// `shape-cli foundry-kit package`.
    PackageKit,
}

/// Honest launch status for a quality-gate task.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AuthorQualityGateLaunch {
    /// Task represented by this launch row.
    pub task: AuthorQualityGateTask,
    /// Whether Author Studio can launch it with the current metadata.
    pub supported: bool,
    /// Suggested CLI invocation when supported.
    pub invocation: Option<String>,
    /// Honest reason when unsupported.
    pub unsupported_reason: Option<String>,
}

/// Artifact refs collected by Author Studio.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AuthorQualityArtifactRefs {
    /// Current package manifest path or package directory for authored packages.
    pub package_manifest_ref: Option<String>,
    /// True only when the current package is verified to match its built-in backing.
    pub verified_built_in_backing: bool,
    /// Output directory for generated evidence.
    pub out_dir: String,
    /// Optional HQ quality report ref.
    pub quality_report_ref: Option<String>,
    /// Optional review manifest ref.
    pub review_manifest_ref: Option<String>,
    /// Optional contact sheet refs.
    pub contact_sheet_refs: Vec<String>,
}

/// Package exporter manifest refs.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AuthorPackageExportManifest {
    /// Kit package manifest ref.
    pub kit_manifest_ref: String,
    /// Provider pack manifest refs.
    pub provider_pack_refs: Vec<String>,
    /// Style pack manifest refs.
    pub style_pack_refs: Vec<String>,
    /// Control profile ref.
    pub control_profile_ref: String,
    /// Candidate strategy pack ref.
    pub candidate_strategy_pack_ref: String,
    /// Quality gate profile ref.
    pub quality_gate_profile_ref: String,
    /// Review manifest ref.
    pub review_manifest_ref: String,
    /// Quality report refs.
    pub quality_report_refs: Vec<String>,
    /// Contact sheet refs.
    pub contact_sheet_refs: Vec<String>,
}

/// Author Studio validation issue.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AuthorStudioValidationIssue {
    /// Stable subject path.
    pub subject: String,
    /// Stable issue code.
    pub code: String,
    /// Human-readable message.
    pub message: String,
}

/// Author Studio validation report.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct AuthorStudioValidationReport {
    /// Validation issues.
    pub issues: Vec<AuthorStudioValidationIssue>,
}

impl AuthorStudioValidationReport {
    /// Return true when the report has no issues.
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
        self.issues.push(AuthorStudioValidationIssue {
            subject: subject.into(),
            code: code.into(),
            message: message.into(),
        });
    }
}

/// Validate guided role descriptors.
#[must_use]
pub fn validate_author_roles(roles: &[AuthorRoleDescriptor]) -> AuthorStudioValidationReport {
    let mut report = AuthorStudioValidationReport::default();
    let mut ids = BTreeSet::new();
    for (index, role) in roles.iter().enumerate() {
        if role.role_id.trim().is_empty() {
            report.push(
                format!("roles.{index}.role_id"),
                "missing_role_id",
                "Role descriptors require a stable role ID.",
            );
        } else if !ids.insert(role.role_id.as_str()) {
            report.push(
                format!("roles.{index}.role_id"),
                "duplicate_role_id",
                "Role IDs must be unique within a family blueprint.",
            );
        }
        if role.display_name.trim().is_empty() || role.description.trim().is_empty() {
            report.push(
                format!("roles.{index}.display_name"),
                "missing_role_copy",
                "Role descriptors require a display name and description.",
            );
        }
        if role.export_part_name.trim().is_empty() {
            report.push(
                format!("roles.{index}.export_part_name"),
                "missing_export_part_name",
                "Role descriptors require an export part name.",
            );
        }
    }
    report
}

/// Validate provider descriptor socket/port metadata.
#[must_use]
pub fn validate_provider_descriptor(
    provider: &ProviderDescriptor,
    roles: &[AuthorRoleDescriptor],
) -> AuthorStudioValidationReport {
    let mut report = AuthorStudioValidationReport::default();
    let role_ids = roles
        .iter()
        .map(|role| role.role_id.as_str())
        .collect::<BTreeSet<_>>();
    if provider.provider_id.trim().is_empty() {
        report.push(
            "provider.provider_id",
            "missing_provider_id",
            "Provider descriptors require a stable ID.",
        );
    }
    if provider.display_name.trim().is_empty() {
        report.push(
            "provider.display_name",
            "missing_provider_label",
            "Provider descriptors require a display name.",
        );
    }
    if !role_ids.contains(provider.semantic_role.as_str()) {
        report.push(
            "provider.semantic_role",
            "dangling_provider_role",
            "Provider semantic role must reference the family role inventory.",
        );
    }
    if provider.provider_slot.trim().is_empty() {
        report.push(
            "provider.provider_slot",
            "missing_provider_slot",
            "Provider descriptors require a provider slot.",
        );
    }
    if !provider.descriptor_only {
        report.push(
            "provider.descriptor_only",
            "provider_import_must_be_descriptor_only",
            "Component/provider import is descriptor-only until mesh import support is reviewed.",
        );
    }
    let mut socket_ids = BTreeSet::new();
    for (index, socket) in provider.socket_requirements.iter().enumerate() {
        let subject = format!("provider.socket_requirements.{index}");
        if socket.required
            && (socket.socket_id.trim().is_empty()
                || socket.port_id.trim().is_empty()
                || socket.target_role.trim().is_empty())
        {
            report.push(
                &subject,
                "missing_required_socket_metadata",
                "Required socket descriptors need socket, port, and target role metadata.",
            );
        }
        if !socket.socket_id.trim().is_empty() && !socket_ids.insert(socket.socket_id.as_str()) {
            report.push(
                format!("{subject}.socket_id"),
                "duplicate_socket_id",
                "Socket IDs must be unique within one provider descriptor.",
            );
        }
        if !socket.target_role.trim().is_empty() && !role_ids.contains(socket.target_role.as_str())
        {
            report.push(
                format!("{subject}.target_role"),
                "dangling_socket_role",
                "Socket target role must reference the family role inventory.",
            );
        }
        if socket.required && socket.compatibility_tags.is_empty() {
            report.push(
                format!("{subject}.compatibility_tags"),
                "missing_required_socket_compatibility_tags",
                "Required sockets need compatibility tags.",
            );
        }
        if socket.required && socket.allowed_attachment_modes.is_empty() {
            report.push(
                format!("{subject}.allowed_attachment_modes"),
                "missing_required_socket_attachment_modes",
                "Required sockets need allowed attachment modes.",
            );
        }
        if socket
            .allowed_attachment_modes
            .iter()
            .any(|mode| mode.trim().is_empty())
        {
            report.push(
                format!("{subject}.allowed_attachment_modes"),
                "blank_socket_attachment_mode",
                "Attachment modes cannot be blank.",
            );
        }
        if socket.required && socket.author_notes.trim().is_empty() {
            report.push(
                format!("{subject}.author_notes"),
                "missing_required_socket_author_notes",
                "Required sockets need author-facing notes.",
            );
        }
    }
    report
}

/// Validate style compatibility authoring data.
#[must_use]
pub fn validate_style_compatibility(
    descriptor: &StyleCompatibilityDescriptor,
) -> AuthorStudioValidationReport {
    let mut report = AuthorStudioValidationReport::default();
    let allowed = descriptor
        .allowed_provider_tags
        .iter()
        .map(String::as_str)
        .collect::<BTreeSet<_>>();
    for forbidden in &descriptor.forbidden_provider_tags {
        if allowed.contains(forbidden.as_str()) {
            report.push(
                format!("style.forbidden_provider_tags.{forbidden}"),
                "style_tag_both_allowed_and_forbidden",
                "A provider tag cannot be both allowed and forbidden.",
            );
        }
    }
    for (style_id, reason) in &descriptor.incompatible_style_packs {
        if reason.trim().is_empty() {
            report.push(
                format!("style.incompatible_style_packs.{style_id}"),
                "missing_incompatibility_reason",
                "Incompatible style packs require author-facing reasons.",
            );
        }
        if descriptor
            .compatible_style_packs
            .iter()
            .any(|candidate| candidate == style_id)
        {
            report.push(
                format!("style.compatible_style_packs.{style_id}"),
                "style_pack_both_compatible_and_incompatible",
                "A style pack cannot be marked both compatible and incompatible.",
            );
        }
    }
    for (subject, value, code) in [
        (
            "style.detail_density_policy",
            &descriptor.detail_density_policy,
            "missing_detail_density_policy",
        ),
        (
            "style.bevel_language_notes",
            &descriptor.bevel_language_notes,
            "missing_bevel_language_notes",
        ),
        (
            "style.proportion_language_notes",
            &descriptor.proportion_language_notes,
            "missing_proportion_language_notes",
        ),
        (
            "style.symmetry_asymmetry_policy",
            &descriptor.symmetry_asymmetry_policy,
            "missing_symmetry_policy",
        ),
    ] {
        if value.trim().is_empty() {
            report.push(
                subject,
                code,
                "Style compatibility policies cannot be empty.",
            );
        }
    }
    report
}

/// Validate control mapping descriptors.
#[must_use]
pub fn validate_control_mappings(
    controls: &[ControlMappingDescriptor],
) -> AuthorStudioValidationReport {
    let mut report = AuthorStudioValidationReport::default();
    let primary_count = controls
        .iter()
        .filter(|control| control.visible && control.primary)
        .count() as u32;
    if primary_count > DEFAULT_MAX_PRIMARY_NOVICE_CONTROLS {
        report.push(
            "controls",
            "too_many_primary_controls",
            format!(
                "Default novice control profiles may expose at most {DEFAULT_MAX_PRIMARY_NOVICE_CONTROLS} primary controls."
            ),
        );
    }

    let mut family_slot_owners = BTreeMap::<&str, &str>::new();
    let mut provider_slot_owners = BTreeMap::<&str, &str>::new();
    let mut control_ids = BTreeSet::new();
    for control in controls.iter().filter(|control| control.visible) {
        if control.control_id.trim().is_empty() {
            report.push(
                "controls.control_id",
                "missing_control_id",
                "Every visible control requires a stable control ID.",
            );
        } else if !control_ids.insert(control.control_id.as_str()) {
            report.push(
                format!("controls.{}.control_id", control.control_id),
                "duplicate_control_id",
                "Control IDs must be unique within one mapping descriptor set.",
            );
        }
        if control.label.trim().is_empty() || control.description.trim().is_empty() {
            report.push(
                format!("controls.{}.label", control.control_id),
                "missing_control_copy",
                "Every visible control requires a human-facing label and description.",
            );
        }
        if control.disabled_reason_policy.trim().is_empty() {
            report.push(
                format!("controls.{}.disabled_reason_policy", control.control_id),
                "missing_disabled_reason_policy",
                "Every control needs a disabled reason policy.",
            );
        }
        if control.topology_behavior == ControlProfileTopologyBehavior::TopologyChanging
            && !matches!(control.kind, ControlProfileControlKind::Choice)
        {
            report.push(
                format!("controls.{}.kind", control.control_id),
                "topology_changing_control_must_be_discrete",
                "Topology-changing controls must be discrete whole-model choices.",
            );
        }
        if matches!(control.kind, ControlProfileControlKind::Choice)
            && control.discrete_options.is_empty()
        {
            report.push(
                format!("controls.{}.discrete_options", control.control_id),
                "choice_control_missing_options",
                "Choice controls require discrete whole-model options.",
            );
        }
        if control.topology_behavior == ControlProfileTopologyBehavior::TopologyChanging
            && control.discrete_options.is_empty()
        {
            report.push(
                format!("controls.{}.discrete_options", control.control_id),
                "topology_changing_control_missing_options",
                "Topology-changing controls require discrete whole-model options.",
            );
        }
        if control
            .discrete_options
            .iter()
            .any(|option| option.trim().is_empty())
        {
            report.push(
                format!("controls.{}.discrete_options", control.control_id),
                "blank_control_option",
                "Discrete control options cannot be blank.",
            );
        }
        if let Some(binding) = control.provider_slot_binding.as_deref()
            && !control
                .owned_provider_slots
                .iter()
                .any(|slot| slot == binding)
        {
            report.push(
                format!("controls.{}.provider_slot_binding", control.control_id),
                "provider_slot_binding_not_owned",
                "Provider slot bindings must reference a provider slot owned by the control.",
            );
        }
        for slot in &control.owned_family_slots {
            if let Some(previous) =
                family_slot_owners.insert(slot.as_str(), control.control_id.as_str())
            {
                report.push(
                    format!("controls.{}.owned_family_slots", control.control_id),
                    "duplicate_visible_slot_ownership",
                    format!(
                        "Visible controls '{}' and '{}' both own family slot '{}'.",
                        previous, control.control_id, slot
                    ),
                );
            }
        }
        for slot in &control.owned_provider_slots {
            if let Some(previous) =
                provider_slot_owners.insert(slot.as_str(), control.control_id.as_str())
            {
                report.push(
                    format!("controls.{}.owned_provider_slots", control.control_id),
                    "duplicate_visible_slot_ownership",
                    format!(
                        "Visible controls '{}' and '{}' both own provider slot '{}'.",
                        previous, control.control_id, slot
                    ),
                );
            }
        }
    }
    report
}

/// Validate candidate strategies against the visible customizer surface.
#[must_use]
pub fn validate_candidate_strategy_descriptors(
    strategies: &[CandidateStrategyDescriptor],
    controls: &[ControlMappingDescriptor],
) -> AuthorStudioValidationReport {
    let mut report = AuthorStudioValidationReport::default();
    let visible_controls = controls
        .iter()
        .filter(|control| control.visible)
        .map(|control| {
            (
                control.control_id.as_str(),
                (
                    control.label.as_str(),
                    control.owned_provider_slots.as_slice(),
                ),
            )
        })
        .collect::<BTreeMap<_, _>>();
    let labels = visible_controls
        .values()
        .map(|(label, _)| *label)
        .collect::<BTreeSet<_>>();
    let allowed_provider_slots = controls
        .iter()
        .filter(|control| control.visible)
        .flat_map(|control| control.owned_provider_slots.iter().map(String::as_str))
        .collect::<BTreeSet<_>>();
    let mut strategy_ids = BTreeSet::new();

    for strategy in strategies {
        if strategy.strategy_id.trim().is_empty() {
            report.push(
                "strategies.strategy_id",
                "missing_strategy_id",
                "Candidate strategies require a stable strategy ID.",
            );
        } else if !strategy_ids.insert(strategy.strategy_id.as_str()) {
            report.push(
                format!("strategies.{}.strategy_id", strategy.strategy_id),
                "duplicate_strategy_id",
                "Candidate strategy IDs must be unique.",
            );
        }
        if strategy.name.trim().is_empty() || strategy.explanation.trim().is_empty() {
            report.push(
                format!("strategies.{}.name", strategy.strategy_id),
                "missing_strategy_copy",
                "Candidate strategies require user-facing names and explanations.",
            );
        }
        for (subject, value, code) in [
            (
                "intensity_policy",
                &strategy.intensity_policy,
                "missing_intensity_policy",
            ),
            (
                "diversity_policy",
                &strategy.diversity_policy,
                "missing_diversity_policy",
            ),
            (
                "rejection_policy",
                &strategy.rejection_policy,
                "missing_rejection_policy",
            ),
            (
                "explanation_template",
                &strategy.explanation_template,
                "missing_explanation_template",
            ),
        ] {
            if value.trim().is_empty() {
                report.push(
                    format!("strategies.{}.{}", strategy.strategy_id, subject),
                    code,
                    "Candidate strategy policies cannot be empty.",
                );
            }
        }
        if strategy.lock_respect_policy.trim().is_empty() {
            report.push(
                format!("strategies.{}.lock_respect_policy", strategy.strategy_id),
                "missing_lock_respect_policy",
                "Candidate strategies must document lock-respect policy.",
            );
        }
        for control_id in &strategy.allowed_controls {
            if !visible_controls.contains_key(control_id.as_str()) {
                report.push(
                    format!("strategies.{}.allowed_controls", strategy.strategy_id),
                    "candidate_strategy_unknown_control",
                    "Candidate strategies must operate in visible customizer space.",
                );
            }
        }
        for provider_slot in &strategy.allowed_provider_changes {
            if !allowed_provider_slots.contains(provider_slot.as_str()) {
                report.push(
                    format!("strategies.{}.allowed_provider_changes", strategy.strategy_id),
                    "candidate_strategy_unknown_provider_slot",
                    "Candidate provider changes must reference visible control-owned provider slots.",
                );
            }
        }
        if contains_raw_recipe_marker(&strategy.explanation_template)
            || strategy
                .allowed_controls
                .iter()
                .any(|control| contains_raw_recipe_marker(control))
            || strategy
                .allowed_provider_changes
                .iter()
                .any(|slot| contains_raw_recipe_marker(slot))
        {
            report.push(
                format!("strategies.{}.explanation_template", strategy.strategy_id),
                "candidate_strategy_uses_raw_recipe_surface",
                "Candidate strategies must not expose raw recipe/scalar perturbations.",
            );
        }
        if !strategy.allowed_controls.is_empty()
            && !labels
                .iter()
                .any(|label| strategy.explanation_template.contains(*label))
        {
            report.push(
                format!("strategies.{}.explanation_template", strategy.strategy_id),
                "candidate_explanation_missing_user_facing_label",
                "Candidate explanations must use user-facing control labels.",
            );
        }
    }
    report
}

fn contains_raw_recipe_marker(value: &str) -> bool {
    let lower = value.to_ascii_lowercase();
    ["::", "scalar", "recipe", "path", "op_id", "semantic_id"]
        .iter()
        .any(|marker| lower.contains(marker))
}

/// Validate preview camera policy descriptors.
#[must_use]
pub fn validate_preview_camera_policy(
    policy: &PreviewCameraPolicyDescriptor,
) -> AuthorStudioValidationReport {
    let mut report = AuthorStudioValidationReport::default();
    let camera_ids = std::iter::once(&policy.default_camera)
        .chain(std::iter::once(&policy.direction_board_camera))
        .chain(std::iter::once(&policy.option_gallery_camera))
        .chain(policy.contact_sheet_cameras.iter())
        .map(|camera| camera.camera_id.as_str())
        .collect::<BTreeSet<_>>();

    let mut seen_camera_ids = BTreeSet::new();
    for camera in std::iter::once(&policy.default_camera)
        .chain(std::iter::once(&policy.direction_board_camera))
        .chain(std::iter::once(&policy.option_gallery_camera))
        .chain(policy.contact_sheet_cameras.iter())
    {
        if camera.camera_id.trim().is_empty()
            || camera.label.trim().is_empty()
            || camera.view.trim().is_empty()
        {
            report.push(
                format!("preview_cameras.{}", camera.camera_id),
                "missing_camera_identity",
                "Camera specs require stable ID, label, and view.",
            );
        }
        if !camera.camera_id.trim().is_empty() && !seen_camera_ids.insert(camera.camera_id.as_str())
        {
            report.push(
                format!("preview_cameras.{}", camera.camera_id),
                "duplicate_camera_id",
                "Camera IDs must be unique within one preview policy.",
            );
        }
        if !camera.supported
            && camera
                .unsupported_reason
                .as_deref()
                .unwrap_or("")
                .is_empty()
        {
            report.push(
                format!("preview_cameras.{}", camera.camera_id),
                "unsupported_camera_missing_reason",
                "Unsupported camera output must report an honest reason.",
            );
        }
        if camera.fitted_scale_policy.trim().is_empty() || camera.lighting_policy.trim().is_empty()
        {
            report.push(
                format!("preview_cameras.{}", camera.camera_id),
                "missing_camera_policy",
                "Camera specs require fitted-scale and lighting policies.",
            );
        }
    }
    for gallery in &policy.option_gallery_policies {
        if gallery.option_camera_ids.is_empty() {
            report.push(
                format!("option_gallery_policies.{}", gallery.control_id),
                "missing_option_gallery_camera",
                "Option galleries require a camera policy.",
            );
        }
        if gallery.option_fitted_scale_policies.is_empty() {
            report.push(
                format!("option_gallery_policies.{}", gallery.control_id),
                "missing_option_gallery_fitted_scale",
                "Option galleries require a fitted-scale policy.",
            );
        }
        if gallery
            .option_fitted_scale_policies
            .iter()
            .any(|policy| policy.trim().is_empty())
        {
            report.push(
                format!("option_gallery_policies.{}", gallery.control_id),
                "blank_option_gallery_fitted_scale",
                "Option-gallery fitted-scale policies cannot be blank.",
            );
        }
        if gallery.option_camera_ids.len() != gallery.option_fitted_scale_policies.len() {
            report.push(
                format!("option_gallery_policies.{}", gallery.control_id),
                "option_gallery_policy_length_mismatch",
                "Option-gallery camera and fitted-scale policy lists must align.",
            );
        }
        if !gallery
            .option_camera_ids
            .iter()
            .all(|camera_id| camera_ids.contains(camera_id.as_str()))
        {
            report.push(
                format!("option_gallery_policies.{}", gallery.control_id),
                "unknown_option_gallery_camera",
                "Option-gallery cameras must reference declared camera specs.",
            );
        }
        if gallery
            .option_camera_ids
            .iter()
            .collect::<BTreeSet<_>>()
            .len()
            > 1
            || gallery
                .option_fitted_scale_policies
                .iter()
                .collect::<BTreeSet<_>>()
                .len()
                > 1
        {
            report.push(
                format!("option_gallery_policies.{}", gallery.control_id),
                "option_gallery_camera_not_consistent",
                "All options in one control gallery must use the same camera and fitted scale.",
            );
        }
    }
    let required_views = ["front", "side", "back", "three-quarter"];
    let declared_views = policy
        .contact_sheet_cameras
        .iter()
        .map(|camera| camera.view.as_str())
        .collect::<BTreeSet<_>>();
    for required_view in required_views {
        if !declared_views.contains(required_view) {
            report.push(
                "contact_sheet_cameras",
                "missing_contact_sheet_view",
                format!("Contact sheets must declare a {required_view} view."),
            );
        }
    }
    report
}

/// Build honest launch rows for existing CLI-backed quality gates.
#[must_use]
pub fn author_quality_gate_launches(
    package: &FoundryKitPackage,
    artifacts: &AuthorQualityArtifactRefs,
) -> Vec<AuthorQualityGateLaunch> {
    let slug = package.kit.source_profile_slug.as_deref();
    let package_ref = artifacts
        .package_manifest_ref
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty());
    let verified_builtin_arg = artifacts
        .verified_built_in_backing
        .then_some(slug)
        .flatten();
    let current_package_arg = package_ref.or(verified_builtin_arg);
    let full_package_arg = if slug.is_some() && artifacts.verified_built_in_backing {
        package_ref.or(slug)
    } else {
        None
    };
    let out_dir = artifacts.out_dir.trim();
    let has_out_dir = !out_dir.is_empty();

    let unsupported = |task, reason: &str| AuthorQualityGateLaunch {
        task,
        supported: false,
        invocation: None,
        unsupported_reason: Some(reason.to_owned()),
    };
    let supported = |task, invocation: String| AuthorQualityGateLaunch {
        task,
        supported: true,
        invocation: Some(invocation),
        unsupported_reason: None,
    };

    vec![
        if let Some(kit_arg) = current_package_arg {
            supported(
                AuthorQualityGateTask::ValidateKit,
                format!("shape-cli foundry-kit validate {kit_arg}"),
            )
        } else {
            unsupported(
                AuthorQualityGateTask::ValidateKit,
                "Validation requires a package manifest reference or verified built-in backing.",
            )
        },
        if let (Some(kit_arg), true) = (full_package_arg, has_out_dir) {
            supported(
                AuthorQualityGateTask::RenderPreview,
                format!("shape-cli foundry-kit preview {kit_arg} --out-dir {out_dir}/preview"),
            )
        } else {
            unsupported(
                AuthorQualityGateTask::RenderPreview,
                "Preview rendering requires verified canonical built-in backing and an output directory.",
            )
        },
        if let (Some(kit_arg), true) = (full_package_arg, has_out_dir) {
            supported(
                AuthorQualityGateTask::RenderContactSheet,
                format!(
                    "shape-cli foundry-kit contact-sheet {kit_arg} --out-dir {out_dir}/contact-sheet"
                ),
            )
        } else {
            unsupported(
                AuthorQualityGateTask::RenderContactSheet,
                "Contact sheets require verified canonical built-in backing and an output directory.",
            )
        },
        if let (Some(_kit_arg), true, Some(slug)) = (
            full_package_arg,
            has_out_dir,
            package.kit.source_profile_slug.as_deref(),
        ) {
            supported(
                AuthorQualityGateTask::BoxPrimitiveGate,
                format!(
                    "cargo test -p shape-foundry-catalog --test box_primitive --jobs 1 # {slug} -> {out_dir}/box-primitive-gate"
                ),
            )
        } else {
            unsupported(
                AuthorQualityGateTask::BoxPrimitiveGate,
                "Box Primitive gate requires verified canonical built-in backing and an output directory.",
            )
        },
        match (&artifacts.quality_report_ref, full_package_arg, has_out_dir) {
            (Some(report_ref), Some(kit_arg), true) => supported(
                AuthorQualityGateTask::ProduceReviewManifest,
                format!(
                    "shape-cli foundry-kit review {kit_arg} --quality-report {report_ref} --out {out_dir}/review-manifest.json"
                ),
            ),
            _ => unsupported(
                AuthorQualityGateTask::ProduceReviewManifest,
                "Review manifest generation requires a quality report reference and output directory.",
            ),
        },
        if !has_out_dir {
            unsupported(
                AuthorQualityGateTask::PackageKit,
                "Package export requires an output directory.",
            )
        } else if let Some(kit_arg) = if slug.is_some() {
            full_package_arg
        } else {
            package_ref
        } {
            supported(
                AuthorQualityGateTask::PackageKit,
                format!("shape-cli foundry-kit package {kit_arg} --out-dir {out_dir}/package"),
            )
        } else if slug.is_some() {
            unsupported(
                AuthorQualityGateTask::PackageKit,
                "Source-backed package export requires verified canonical built-in backing.",
            )
        } else {
            unsupported(
                AuthorQualityGateTask::PackageKit,
                "Package export requires a package manifest reference.",
            )
        },
    ]
}

/// Build package/review refs for Author Studio export.
#[must_use]
pub fn author_package_export_manifest(
    _package: &FoundryKitPackage,
    artifacts: &AuthorQualityArtifactRefs,
) -> AuthorPackageExportManifest {
    AuthorPackageExportManifest {
        kit_manifest_ref: "kit-manifest.json".to_owned(),
        provider_pack_refs: vec!["provider-pack.json".to_owned()],
        style_pack_refs: vec!["style-pack.json".to_owned()],
        control_profile_ref: "control-profile.json".to_owned(),
        candidate_strategy_pack_ref: "candidate-strategy-pack.json".to_owned(),
        quality_gate_profile_ref: "quality-gate-profile.json".to_owned(),
        review_manifest_ref: artifacts
            .review_manifest_ref
            .clone()
            .unwrap_or_else(|| "review-manifest.json".to_owned()),
        quality_report_refs: artifacts.quality_report_ref.iter().cloned().collect(),
        contact_sheet_refs: artifacts.contact_sheet_refs.clone(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        CANDIDATE_STRATEGY_PACK_SCHEMA_VERSION, CONTROL_PROFILE_SCHEMA_VERSION,
        CandidateStrategyPack, CatalogVisibilityPolicy, ControlOptionVisibility, ControlProfile,
        ExportPartNamingPolicy, FAMILY_BLUEPRINT_SCHEMA_VERSION,
        FOUNDRY_KIT_PACKAGE_SCHEMA_VERSION, FOUNDRY_KIT_SCHEMA_VERSION, FamilyBlueprint,
        FamilyBlueprintRole, FoundryKit, FoundryKitQualityTier, FutureMaterialVocabulary,
        HighLevelScalePolicy, KIT_CATALOG_MANIFEST_SCHEMA_VERSION,
        KIT_COMPATIBILITY_MATRIX_SCHEMA_VERSION, KIT_REVIEW_MANIFEST_SCHEMA_VERSION,
        KitCandidateStrategy, KitCatalogManifest, KitCompatibilityMatrix, KitReviewManifest,
        PROVIDER_PACK_SCHEMA_VERSION, PreviewCameraPolicy, ProviderPack, ProviderPackOption,
        ProviderSlotExpectation, QUALITY_GATE_PROFILE_SCHEMA_VERSION, QualityGateProfile,
        STYLE_PACK_SCHEMA_VERSION, StylePack,
    };

    #[test]
    fn author_studio_is_gated_from_default_release() {
        let shell = foundry_author_studio_shell(FoundryAuthorStudioGate::default_release());
        assert!(!shell.available);
        assert!(shell.steps.is_empty());
        assert!(
            shell
                .unavailable_reason
                .as_deref()
                .is_some_and(|reason| reason.contains("developer"))
        );
    }

    #[test]
    fn author_studio_developer_gate_exposes_workflow_steps() {
        let shell = foundry_author_studio_shell(FoundryAuthorStudioGate::developer_enabled());
        assert!(shell.available);
        assert_eq!(shell.steps.len(), 9);
        assert_eq!(shell.steps[0].label, "Kit Overview");
        assert_eq!(shell.steps[8].label, "Review & Package");
    }

    #[test]
    fn role_labeling_descriptors_validate_inventory() {
        let roles = sample_roles();
        assert!(validate_author_roles(&roles).is_valid());
        let mut duplicate = roles.clone();
        duplicate.push(roles[0].clone());
        assert!(
            validate_author_roles(&duplicate)
                .issues
                .iter()
                .any(|issue| issue.code == "duplicate_role_id")
        );
    }

    #[test]
    fn socket_port_descriptor_validation_catches_missing_and_dangling_refs() {
        let provider = ProviderDescriptor {
            provider_id: "provider_a".to_owned(),
            display_name: "Provider A".to_owned(),
            semantic_role: "missing".to_owned(),
            provider_slot: "body_slot".to_owned(),
            tags: vec!["clean".to_owned()],
            compatibility_tags: vec!["plain".to_owned()],
            approximate_triangle_budget: Some(512),
            preview_available: false,
            descriptor_only: true,
            socket_requirements: vec![
                SocketPortDescriptor {
                    socket_id: "attach".to_owned(),
                    port_id: String::new(),
                    target_role: "detail".to_owned(),
                    compatibility_tags: Vec::new(),
                    allowed_attachment_modes: vec!["snap".to_owned()],
                    required: true,
                    author_notes: "Required detail attachment.".to_owned(),
                },
                SocketPortDescriptor {
                    socket_id: "attach".to_owned(),
                    port_id: "other".to_owned(),
                    target_role: "missing".to_owned(),
                    compatibility_tags: vec!["plain".to_owned()],
                    allowed_attachment_modes: vec!["snap".to_owned()],
                    required: true,
                    author_notes: "Duplicate socket.".to_owned(),
                },
            ],
        };
        let report = validate_provider_descriptor(&provider, &sample_roles());
        let codes = issue_codes(&report);
        assert!(codes.contains("dangling_provider_role"));
        assert!(codes.contains("missing_required_socket_metadata"));
        assert!(codes.contains("missing_required_socket_compatibility_tags"));
        assert!(codes.contains("duplicate_socket_id"));
        assert!(codes.contains("dangling_socket_role"));
    }

    #[test]
    fn provider_descriptors_reject_unsupported_import_claims_and_sparse_sockets() {
        let provider = ProviderDescriptor {
            provider_id: "provider_a".to_owned(),
            display_name: "Provider A".to_owned(),
            semantic_role: "body".to_owned(),
            provider_slot: "body_slot".to_owned(),
            tags: vec!["clean".to_owned()],
            compatibility_tags: vec!["plain".to_owned()],
            approximate_triangle_budget: Some(512),
            preview_available: false,
            descriptor_only: false,
            socket_requirements: vec![SocketPortDescriptor {
                socket_id: "attach".to_owned(),
                port_id: "attach_port".to_owned(),
                target_role: "detail".to_owned(),
                compatibility_tags: vec!["plain".to_owned()],
                allowed_attachment_modes: Vec::new(),
                required: true,
                author_notes: String::new(),
            }],
        };
        let report = validate_provider_descriptor(&provider, &sample_roles());
        let codes = issue_codes(&report);
        assert!(codes.contains("provider_import_must_be_descriptor_only"));
        assert!(codes.contains("missing_required_socket_attachment_modes"));
        assert!(codes.contains("missing_required_socket_author_notes"));
    }

    #[test]
    fn style_compatibility_validation_rejects_conflicting_tags() {
        let descriptor = StyleCompatibilityDescriptor {
            compatible_style_packs: vec!["plain".to_owned()],
            incompatible_style_packs: BTreeMap::from([("plain".to_owned(), String::new())]),
            allowed_provider_tags: vec!["decorative".to_owned()],
            forbidden_provider_tags: vec!["decorative".to_owned()],
            detail_density_policy: "Readable at thumbnail size.".to_owned(),
            bevel_language_notes: "Broad bevels.".to_owned(),
            proportion_language_notes: "Plain proportions.".to_owned(),
            symmetry_asymmetry_policy: "Mostly symmetric.".to_owned(),
        };
        let report = validate_style_compatibility(&descriptor);
        let codes = issue_codes(&report);
        assert!(codes.contains("style_tag_both_allowed_and_forbidden"));
        assert!(codes.contains("style_pack_both_compatible_and_incompatible"));
        assert!(codes.contains("missing_incompatibility_reason"));
    }

    #[test]
    fn control_mapping_rejects_duplicate_ownership_and_bad_topology_controls() {
        let controls = vec![
            control(
                "shape",
                "Shape",
                "body_slot",
                ControlProfileControlKind::Continuous,
            ),
            ControlMappingDescriptor {
                control_id: "shape_alt".to_owned(),
                label: "Shape Alt".to_owned(),
                description: "Changes the body.".to_owned(),
                kind: ControlProfileControlKind::Continuous,
                primary: true,
                visible: true,
                owned_family_slots: Vec::new(),
                owned_provider_slots: vec!["body_slot".to_owned()],
                response_curve_descriptor: "linear".to_owned(),
                discrete_options: Vec::new(),
                provider_slot_binding: Some("body_slot".to_owned()),
                topology_behavior: ControlProfileTopologyBehavior::TopologyChanging,
                disabled_reason_policy: "Requires a body provider.".to_owned(),
            },
        ];
        let report = validate_control_mappings(&controls);
        let codes = issue_codes(&report);
        assert!(codes.contains("duplicate_visible_slot_ownership"));
        assert!(codes.contains("topology_changing_control_must_be_discrete"));
    }

    #[test]
    fn control_mapping_rejects_empty_choice_options_and_unowned_bindings() {
        let controls = vec![ControlMappingDescriptor {
            control_id: "body_shape".to_owned(),
            label: "Body Shape".to_owned(),
            description: "Changes the body.".to_owned(),
            kind: ControlProfileControlKind::Choice,
            primary: true,
            visible: true,
            owned_family_slots: Vec::new(),
            owned_provider_slots: vec!["body_slot".to_owned()],
            response_curve_descriptor: "discrete".to_owned(),
            discrete_options: Vec::new(),
            provider_slot_binding: Some("detail_slot".to_owned()),
            topology_behavior: ControlProfileTopologyBehavior::TopologyChanging,
            disabled_reason_policy: "Requires a body provider.".to_owned(),
        }];
        let report = validate_control_mappings(&controls);
        let codes = issue_codes(&report);
        assert!(codes.contains("choice_control_missing_options"));
        assert!(codes.contains("topology_changing_control_missing_options"));
        assert!(codes.contains("provider_slot_binding_not_owned"));
    }

    #[test]
    fn primary_controls_are_limited_to_seven_by_default() {
        let controls = (0..8)
            .map(|index| {
                control(
                    &format!("control_{index}"),
                    &format!("Control {index}"),
                    &format!("slot_{index}"),
                    ControlProfileControlKind::Choice,
                )
            })
            .collect::<Vec<_>>();
        assert!(
            validate_control_mappings(&controls)
                .issues
                .iter()
                .any(|issue| issue.code == "too_many_primary_controls")
        );
    }

    #[test]
    fn candidate_strategy_validation_requires_user_facing_labels() {
        let controls = vec![control(
            "armor_mass",
            "Armor Mass",
            "armor_slot",
            ControlProfileControlKind::Choice,
        )];
        let strategies = vec![CandidateStrategyDescriptor {
            strategy_id: "heavy".to_owned(),
            name: "Heavy".to_owned(),
            explanation: "Heavier silhouette.".to_owned(),
            allowed_controls: vec!["armor_mass".to_owned()],
            allowed_provider_changes: vec!["armor_slot".to_owned()],
            intensity_policy: "medium".to_owned(),
            diversity_policy: "avoid duplicates".to_owned(),
            lock_respect_policy: "Respect locked controls.".to_owned(),
            rejection_policy: "Reject invalid output.".to_owned(),
            explanation_template: "Changes scalar::armor.mass".to_owned(),
        }];
        let report = validate_candidate_strategy_descriptors(&strategies, &controls);
        let codes = issue_codes(&report);
        assert!(codes.contains("candidate_strategy_uses_raw_recipe_surface"));
        assert!(codes.contains("candidate_explanation_missing_user_facing_label"));

        let fixed = vec![CandidateStrategyDescriptor {
            explanation_template: "Armor Mass becomes heavier.".to_owned(),
            ..strategies[0].clone()
        }];
        assert!(validate_candidate_strategy_descriptors(&fixed, &controls).is_valid());
    }

    #[test]
    fn candidate_strategy_validation_rejects_unknown_provider_changes_and_empty_policies() {
        let controls = vec![control(
            "armor_mass",
            "Armor Mass",
            "armor_slot",
            ControlProfileControlKind::Choice,
        )];
        let strategies = vec![CandidateStrategyDescriptor {
            strategy_id: String::new(),
            name: "Heavy".to_owned(),
            explanation: "Heavier silhouette.".to_owned(),
            allowed_controls: vec!["armor_mass".to_owned()],
            allowed_provider_changes: vec!["missing_slot".to_owned()],
            intensity_policy: String::new(),
            diversity_policy: String::new(),
            lock_respect_policy: String::new(),
            rejection_policy: String::new(),
            explanation_template: String::new(),
        }];
        let report = validate_candidate_strategy_descriptors(&strategies, &controls);
        let codes = issue_codes(&report);
        assert!(codes.contains("missing_strategy_id"));
        assert!(codes.contains("missing_intensity_policy"));
        assert!(codes.contains("missing_diversity_policy"));
        assert!(codes.contains("missing_lock_respect_policy"));
        assert!(codes.contains("missing_rejection_policy"));
        assert!(codes.contains("missing_explanation_template"));
        assert!(codes.contains("candidate_strategy_unknown_provider_slot"));
    }

    #[test]
    fn preview_camera_policy_validates_gallery_consistency() {
        let policy = PreviewCameraPolicyDescriptor {
            default_camera: camera("default", "three-quarter"),
            direction_board_camera: camera("direction", "three-quarter"),
            option_gallery_camera: camera("option", "three-quarter"),
            contact_sheet_cameras: vec![
                camera("front", "front"),
                camera("side", "side"),
                camera("back", "back"),
                camera("three-quarter", "three-quarter"),
            ],
            option_gallery_policies: vec![OptionGalleryCameraPolicy {
                control_id: "support_style".to_owned(),
                option_camera_ids: vec!["option".to_owned(), "front".to_owned()],
                option_fitted_scale_policies: vec!["fit_model".to_owned(), "fit_part".to_owned()],
            }],
        };
        let report = validate_preview_camera_policy(&policy);
        assert!(
            report
                .issues
                .iter()
                .any(|issue| issue.code == "option_gallery_camera_not_consistent")
        );
    }

    #[test]
    fn preview_camera_policy_rejects_missing_identity_and_scale_policies() {
        let policy = PreviewCameraPolicyDescriptor {
            default_camera: PreviewCameraDescriptor {
                camera_id: String::new(),
                label: String::new(),
                view: String::new(),
                fitted_scale_policy: "fit_model".to_owned(),
                lighting_policy: "clay_reference".to_owned(),
                supported: true,
                unsupported_reason: None,
            },
            direction_board_camera: camera("direction", "three-quarter"),
            option_gallery_camera: camera("option", "three-quarter"),
            contact_sheet_cameras: vec![
                camera("front", "front"),
                camera("side", "side"),
                camera("back", "back"),
                camera("three-quarter", "three-quarter"),
            ],
            option_gallery_policies: vec![OptionGalleryCameraPolicy {
                control_id: "support_style".to_owned(),
                option_camera_ids: vec!["option".to_owned()],
                option_fitted_scale_policies: Vec::new(),
            }],
        };
        let report = validate_preview_camera_policy(&policy);
        let codes = issue_codes(&report);
        assert!(codes.contains("missing_camera_identity"));
        assert!(codes.contains("missing_option_gallery_fitted_scale"));
        assert!(codes.contains("option_gallery_policy_length_mismatch"));
    }

    #[test]
    fn quality_gate_runner_emits_honest_unsupported_states() {
        let mut package = sample_package();
        package.kit.source_profile_slug = None;
        let launches = author_quality_gate_launches(
            &package,
            &AuthorQualityArtifactRefs {
                package_manifest_ref: None,
                verified_built_in_backing: false,
                out_dir: "target/author".to_owned(),
                quality_report_ref: None,
                review_manifest_ref: None,
                contact_sheet_refs: Vec::new(),
            },
        );
        assert!(launches.iter().any(|launch| {
            launch.task == AuthorQualityGateTask::ValidateKit && !launch.supported
        }));
        assert!(launches.iter().any(|launch| {
            launch.task == AuthorQualityGateTask::BoxPrimitiveGate
                && !launch.supported
                && launch.unsupported_reason.is_some()
        }));
        assert!(launches.iter().any(|launch| {
            launch.task == AuthorQualityGateTask::ProduceReviewManifest && !launch.supported
        }));
    }

    #[test]
    fn quality_gate_runner_uses_package_ref_for_authored_packages() {
        let mut package = sample_package();
        package.kit.source_profile_slug = Some("sample".to_owned());
        let launches = author_quality_gate_launches(
            &package,
            &AuthorQualityArtifactRefs {
                package_manifest_ref: Some("target/author/foundry-kit-package.json".to_owned()),
                verified_built_in_backing: false,
                out_dir: "target/author".to_owned(),
                quality_report_ref: Some("target/author/quality-report.json".to_owned()),
                review_manifest_ref: None,
                contact_sheet_refs: Vec::new(),
            },
        );
        let validate = launches
            .iter()
            .find(|launch| launch.task == AuthorQualityGateTask::ValidateKit)
            .expect("validate launch row");
        assert!(validate.supported);
        assert!(validate.invocation.as_deref().is_some_and(|invocation| {
            invocation.contains("target/author/foundry-kit-package.json")
        }));
        assert!(launches.iter().any(|launch| {
            launch.task == AuthorQualityGateTask::PackageKit
                && !launch.supported
                && launch
                    .unsupported_reason
                    .as_deref()
                    .is_some_and(|reason| reason.contains("verified canonical"))
        }));
        assert!(launches.iter().any(|launch| {
            launch.task == AuthorQualityGateTask::ProduceReviewManifest && !launch.supported
        }));
    }

    #[test]
    fn quality_gate_runner_launches_verified_builtin_rows() {
        let package = sample_package();
        let launches = author_quality_gate_launches(
            &package,
            &AuthorQualityArtifactRefs {
                package_manifest_ref: None,
                verified_built_in_backing: true,
                out_dir: "target/author".to_owned(),
                quality_report_ref: Some("target/author/quality-report.json".to_owned()),
                review_manifest_ref: None,
                contact_sheet_refs: Vec::new(),
            },
        );
        assert!(launches.iter().all(|launch| launch.supported));
        assert!(launches.iter().any(|launch| {
            launch.task == AuthorQualityGateTask::BoxPrimitiveGate
                && launch
                    .invocation
                    .as_deref()
                    .is_some_and(|invocation| invocation.contains("box_primitive"))
        }));
    }

    #[test]
    fn package_export_manifest_includes_review_manifest() {
        let package = sample_package();
        let manifest = author_package_export_manifest(
            &package,
            &AuthorQualityArtifactRefs {
                package_manifest_ref: None,
                verified_built_in_backing: true,
                out_dir: "target/author".to_owned(),
                quality_report_ref: Some("quality-report.json".to_owned()),
                review_manifest_ref: Some("review-manifest.json".to_owned()),
                contact_sheet_refs: vec!["contact-sheet.png".to_owned()],
            },
        );
        assert_eq!(manifest.review_manifest_ref, "review-manifest.json");
        assert_eq!(manifest.kit_manifest_ref, "kit-manifest.json");
        assert_eq!(manifest.provider_pack_refs, vec!["provider-pack.json"]);
        assert_eq!(manifest.style_pack_refs, vec!["style-pack.json"]);
        assert_eq!(manifest.control_profile_ref, "control-profile.json");
        assert_eq!(
            manifest.candidate_strategy_pack_ref,
            "candidate-strategy-pack.json"
        );
        assert_eq!(
            manifest.quality_gate_profile_ref,
            "quality-gate-profile.json"
        );
        assert_eq!(manifest.contact_sheet_refs, vec!["contact-sheet.png"]);
    }

    fn issue_codes(report: &AuthorStudioValidationReport) -> BTreeSet<&str> {
        report
            .issues
            .iter()
            .map(|issue| issue.code.as_str())
            .collect()
    }

    fn sample_roles() -> Vec<AuthorRoleDescriptor> {
        vec![
            AuthorRoleDescriptor {
                role_id: "body".to_owned(),
                display_name: "Body".to_owned(),
                description: "Main box body.".to_owned(),
                required: true,
                repeated: false,
                default_visibility: true,
                export_part_name: "Body".to_owned(),
            },
            AuthorRoleDescriptor {
                role_id: "detail".to_owned(),
                display_name: "Detail".to_owned(),
                description: "Optional box detail.".to_owned(),
                required: true,
                repeated: false,
                default_visibility: true,
                export_part_name: "Detail".to_owned(),
            },
        ]
    }

    fn control(
        id: &str,
        label: &str,
        slot: &str,
        kind: ControlProfileControlKind,
    ) -> ControlMappingDescriptor {
        ControlMappingDescriptor {
            control_id: id.to_owned(),
            label: label.to_owned(),
            description: format!("Adjusts {label}."),
            kind,
            primary: true,
            visible: true,
            owned_family_slots: Vec::new(),
            owned_provider_slots: vec![slot.to_owned()],
            response_curve_descriptor: "linear".to_owned(),
            discrete_options: vec!["A".to_owned(), "B".to_owned()],
            provider_slot_binding: Some(slot.to_owned()),
            topology_behavior: if matches!(kind, ControlProfileControlKind::Choice) {
                ControlProfileTopologyBehavior::TopologyChanging
            } else {
                ControlProfileTopologyBehavior::TopologyPreserving
            },
            disabled_reason_policy: "Requires a compatible provider option.".to_owned(),
        }
    }

    fn camera(id: &str, view: &str) -> PreviewCameraDescriptor {
        PreviewCameraDescriptor {
            camera_id: id.to_owned(),
            label: id.to_owned(),
            view: view.to_owned(),
            fitted_scale_policy: "fit_model".to_owned(),
            lighting_policy: "clay_reference".to_owned(),
            supported: true,
            unsupported_reason: None,
        }
    }

    fn sample_package() -> FoundryKitPackage {
        FoundryKitPackage {
            schema_version: FOUNDRY_KIT_PACKAGE_SCHEMA_VERSION,
            kit: FoundryKit {
                schema_version: FOUNDRY_KIT_SCHEMA_VERSION,
                kit_id: "sample-kit".to_owned(),
                display_name: "Sample Kit".to_owned(),
                family_blueprint_id: "sample-family".to_owned(),
                provider_pack_id: "sample-provider".to_owned(),
                style_pack_id: "sample-style".to_owned(),
                control_profile_id: "sample-controls".to_owned(),
                candidate_strategy_pack_id: "sample-strategies".to_owned(),
                quality_gate_profile_id: "sample-quality".to_owned(),
                compatibility_matrix_id: "sample-compatibility".to_owned(),
                review_manifest_id: "sample-review".to_owned(),
                catalog_manifest_id: "sample-catalog".to_owned(),
                preview_camera_policy: PreviewCameraPolicy {
                    policy_id: "sample-preview".to_owned(),
                    required_views: vec!["front".to_owned(), "three-quarter".to_owned()],
                    clay_preview_required: true,
                    contact_sheet_required: false,
                },
                quality_tier: FoundryKitQualityTier::Draft,
                catalog_visibility_policy: CatalogVisibilityPolicy::hidden(
                    "Draft kits stay hidden.",
                ),
                source_profile_slug: Some("sample".to_owned()),
                category_chips: vec!["Author".to_owned()],
            },
            family_blueprint: FamilyBlueprint {
                schema_version: FAMILY_BLUEPRINT_SCHEMA_VERSION,
                family_id: "sample-family".to_owned(),
                display_name: "Sample Family".to_owned(),
                semantic_roles: vec![FamilyBlueprintRole {
                    role_id: "body".to_owned(),
                    label: "Body".to_owned(),
                    required: true,
                    tags: vec!["box".to_owned()],
                }],
                required_roles: vec!["body".to_owned()],
                optional_roles: Vec::new(),
                provider_slots: vec![ProviderSlotExpectation {
                    slot_id: "body_slot".to_owned(),
                    role_id: "body".to_owned(),
                    required: true,
                    attachment_tags: vec!["center".to_owned()],
                }],
                attachment_expectations: Vec::new(),
                scale_policy: HighLevelScalePolicy {
                    label: "Box scale".to_owned(),
                    allowed_range: Some("Authored box scale.".to_owned()),
                },
                export_part_naming_policy: ExportPartNamingPolicy {
                    strategy: "role labels".to_owned(),
                    required_part_names: vec!["Body".to_owned()],
                },
            },
            provider_pack: ProviderPack {
                schema_version: PROVIDER_PACK_SCHEMA_VERSION,
                pack_id: "sample-provider".to_owned(),
                family_id: Some("sample-family".to_owned()),
                compatible_family_ids: vec!["sample-family".to_owned()],
                provider_slots_supplied: vec!["body_slot".to_owned()],
                provider_options: vec![ProviderPackOption {
                    option_id: "simple_body".to_owned(),
                    slot_id: "body_slot".to_owned(),
                    label: "Simple Body".to_owned(),
                    semantic_roles: vec!["body".to_owned()],
                    compatibility_tags: vec!["plain".to_owned()],
                    detail_density_tags: vec!["clean".to_owned()],
                    triangle_budget_estimate: Some(512),
                }],
                semantic_role_coverage: vec!["body".to_owned()],
                socket_attachment_tags: vec!["center".to_owned()],
                detail_density_tags: vec!["clean".to_owned()],
                triangle_budget_estimates: BTreeMap::from([("simple_body".to_owned(), 512)]),
                compatibility_tags: vec!["plain".to_owned()],
            },
            style_pack: StylePack {
                schema_version: STYLE_PACK_SCHEMA_VERSION,
                style_id: "sample-style".to_owned(),
                display_name: "Sample Style".to_owned(),
                compatible_family_ids: vec!["sample-family".to_owned()],
                bevel_language: "Readable bevels.".to_owned(),
                proportion_language: "Plain proportions.".to_owned(),
                detail_density_policy: "Moderate details.".to_owned(),
                silhouette_exaggeration_policy: "Readable outline.".to_owned(),
                symmetry_asymmetry_policy: "Mostly symmetric.".to_owned(),
                allowed_provider_tags: vec!["plain".to_owned()],
                forbidden_provider_tags: Vec::new(),
                compatible_provider_packs: vec!["sample-provider".to_owned()],
                incompatible_provider_packs: Vec::new(),
                future_material_vocabulary: Some(FutureMaterialVocabulary {
                    label: "Reserved only".to_owned(),
                    tags: vec!["metal".to_owned()],
                }),
            },
            control_profile: ControlProfile {
                schema_version: CONTROL_PROFILE_SCHEMA_VERSION,
                profile_id: "sample-controls".to_owned(),
                family_id: "sample-family".to_owned(),
                style_id: Some("sample-style".to_owned()),
                maximum_primary_controls: DEFAULT_MAX_PRIMARY_NOVICE_CONTROLS,
                controls: vec![crate::ControlProfileControl {
                    control_id: "body_shape".to_owned(),
                    label: "Body Shape".to_owned(),
                    description: "Choose the body silhouette.".to_owned(),
                    kind: ControlProfileControlKind::Choice,
                    owned_family_slots: Vec::new(),
                    owned_provider_slots: vec!["body_slot".to_owned()],
                    visible_effect_expectation: "Body silhouette changes.".to_owned(),
                    topology_behavior: ControlProfileTopologyBehavior::TopologyChanging,
                    option_visibility: ControlOptionVisibility {
                        hide_invalid_from_novices: true,
                        show_plain_language_reasons: true,
                    },
                    default_locked: false,
                    primary: true,
                    visible: true,
                }],
            },
            candidate_strategy_pack: CandidateStrategyPack {
                schema_version: CANDIDATE_STRATEGY_PACK_SCHEMA_VERSION,
                pack_id: "sample-strategies".to_owned(),
                strategies: vec![KitCandidateStrategy {
                    strategy_id: "plain".to_owned(),
                    name: "Plain".to_owned(),
                    allowed_controls: vec!["body_shape".to_owned()],
                    explanation_templates: vec!["Body Shape becomes broader.".to_owned()],
                }],
                allowed_controls: vec!["body_shape".to_owned()],
                allowed_provider_choices: BTreeMap::from([(
                    "body_slot".to_owned(),
                    vec!["simple_body".to_owned()],
                )]),
                diversity_goals: vec!["shape".to_owned()],
                invalid_state_rejection_policy: "Reject invalid output.".to_owned(),
                lock_respect_policy: "Respect locked controls.".to_owned(),
            },
            quality_gate_profile: QualityGateProfile {
                schema_version: QUALITY_GATE_PROFILE_SCHEMA_VERSION,
                profile_id: "sample-quality".to_owned(),
                required_tier: FoundryKitQualityTier::Draft,
                mesh_gates: vec!["model validates".to_owned()],
                candidate_gates: vec!["six candidates".to_owned()],
                contact_sheet_gates: Vec::new(),
                export_gates: vec!["package export".to_owned()],
                manual_review_gates: vec!["manual review".to_owned()],
            },
            compatibility_matrix: KitCompatibilityMatrix {
                schema_version: KIT_COMPATIBILITY_MATRIX_SCHEMA_VERSION,
                matrix_id: "sample-compatibility".to_owned(),
                compatible_style_provider_pairs: Vec::new(),
                incompatible_style_provider_pairs: Vec::new(),
            },
            review_manifest: KitReviewManifest {
                schema_version: KIT_REVIEW_MANIFEST_SCHEMA_VERSION,
                manifest_id: "sample-review".to_owned(),
                tier_requested: FoundryKitQualityTier::Draft,
                tier_achieved: FoundryKitQualityTier::Draft,
                reviewer: None,
                human_approval_marker: false,
                adversarial_review_marker: false,
                visual_review_notes: Vec::new(),
                contact_sheet_paths: Vec::new(),
                benchmark_refs: Vec::new(),
                known_limitations: vec!["Descriptor only.".to_owned()],
                blocked_reasons: vec!["Manual review pending.".to_owned()],
            },
            catalog_manifest: KitCatalogManifest {
                schema_version: KIT_CATALOG_MANIFEST_SCHEMA_VERSION,
                catalog_id: "sample-catalog".to_owned(),
                kit_ids: vec!["sample-kit".to_owned()],
                default_visible_kit_ids: Vec::new(),
                developer_preview_kit_ids: vec!["sample-kit".to_owned()],
                hidden_kit_reasons: BTreeMap::from([(
                    "sample-kit".to_owned(),
                    "Draft kits stay hidden.".to_owned(),
                )]),
            },
        }
    }
}
