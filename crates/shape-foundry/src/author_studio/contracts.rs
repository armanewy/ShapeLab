
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
