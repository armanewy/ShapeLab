
/// Current schema version for foundation drafts.
pub const FOUNDRY_FOUNDATION_DRAFT_SCHEMA_VERSION: u32 = 1;
/// Current schema version for adversarial foundation reports.
pub const FOUNDRY_FOUNDATION_ADVERSARIAL_SCHEMA_VERSION: u32 = 1;

/// Source of a draft foundation.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FoundationDraftSourceKind {
    /// Authored directly by a human.
    Human,
    /// Drafted with LLM assistance, without trusting the LLM as final authority.
    LlmAssisted,
    /// Deterministic internal fixture.
    GeneratedFixture,
}

/// Target quality requested by a draft.
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FoundationQualityTarget {
    /// Internal draft.
    Draft,
    /// Prototype target.
    Prototype,
    /// Usable target after gates and review.
    Usable,
    /// Showcase target after adversarial review.
    Showcase,
}

impl From<FoundationQualityTarget> for FoundryKitQualityTier {
    fn from(value: FoundationQualityTarget) -> Self {
        match value {
            FoundationQualityTarget::Draft => Self::Draft,
            FoundationQualityTarget::Prototype => Self::Prototype,
            FoundationQualityTarget::Usable => Self::Usable,
            FoundationQualityTarget::Showcase => Self::Showcase,
        }
    }
}

/// Draft catalog visibility. Foundation drafts default to internal-only.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FoundationCatalogVisibility {
    /// Internal authoring only.
    InternalOnly,
    /// Developer preview catalog only.
    DeveloperPreview,
    /// Default novice catalog. Validation rejects Draft and Prototype targets here.
    NoviceCatalog,
}

/// One draft family role.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DraftFamilyRole {
    /// Stable role ID.
    pub role_id: String,
    /// Human-facing label.
    pub label: String,
    /// Whether this role is required.
    pub required: bool,
    /// Product-safe role tags.
    pub tags: Vec<String>,
}

/// One draft socket/attachment expectation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DraftSocket {
    /// Stable socket ID.
    pub socket_id: String,
    /// Source role.
    pub from_role: String,
    /// Destination role.
    pub to_role: String,
    /// Product-safe compatibility tags.
    pub compatibility_tags: Vec<String>,
    /// Whether the socket is required.
    pub required: bool,
}

/// Draft family blueprint.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DraftFamilyBlueprint {
    /// Family ID.
    pub family_id: String,
    /// Human-facing family name.
    pub display_name: String,
    /// Role inventory.
    pub roles: Vec<DraftFamilyRole>,
    /// Required role IDs.
    pub required_roles: Vec<String>,
    /// Optional role IDs.
    pub optional_roles: Vec<String>,
    /// Socket/attachment expectations.
    pub sockets: Vec<DraftSocket>,
    /// Export part names expected by review.
    pub export_part_names: Vec<String>,
}

/// Draft provider slot.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DraftProviderSlot {
    /// Stable slot ID.
    pub slot_id: String,
    /// Role supplied by the slot.
    pub role_id: String,
    /// Whether the slot is required.
    pub required: bool,
    /// Compatibility tags.
    pub compatibility_tags: Vec<String>,
}

/// Draft provider pack summary.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DraftProviderPack {
    /// Stable pack ID.
    pub pack_id: String,
    /// Human-facing label.
    pub label: String,
    /// Provider slots supplied by this pack.
    pub supplied_slots: Vec<String>,
    /// Compatibility tags.
    pub compatibility_tags: Vec<String>,
}

/// Draft provider taxonomy.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DraftProviderTaxonomy {
    /// Stable taxonomy ID.
    pub taxonomy_id: String,
    /// Provider slots.
    pub provider_slots: Vec<DraftProviderSlot>,
    /// Draft provider packs.
    pub provider_packs: Vec<DraftProviderPack>,
}

/// Draft style pack.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DraftStylePack {
    /// Stable style ID.
    pub style_id: String,
    /// Human-facing style label.
    pub display_name: String,
    /// Bevel language draft.
    pub bevel_language: String,
    /// Proportion language draft.
    pub proportion_language: String,
    /// Detail-density policy draft.
    pub detail_density_policy: String,
    /// Silhouette policy draft.
    pub silhouette_policy: String,
    /// Symmetry/asymmetry policy draft.
    pub symmetry_policy: String,
    /// Allowed provider tags.
    pub allowed_provider_tags: Vec<String>,
    /// Forbidden provider tags.
    pub forbidden_provider_tags: Vec<String>,
    /// Additional style IDs this draft intentionally evaluates in its matrix.
    #[serde(default)]
    pub compatibility_style_ids: Vec<String>,
}

/// Draft novice control.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DraftControl {
    /// Stable control ID.
    pub control_id: String,
    /// Product-facing label.
    pub label: String,
    /// Product-facing description.
    pub description: String,
    /// Control kind.
    pub kind: ControlProfileControlKind,
    /// Whether this is a primary novice control.
    pub primary: bool,
    /// Whether this is visible.
    pub visible: bool,
    /// Owned family slots.
    pub owned_family_slots: Vec<String>,
    /// Owned provider slots.
    pub owned_provider_slots: Vec<String>,
    /// Topology behavior.
    pub topology_behavior: ControlProfileTopologyBehavior,
    /// Plain-language expected effect.
    pub visible_effect_expectation: String,
}

/// Draft control profile.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DraftControlProfile {
    /// Stable control profile ID.
    pub profile_id: String,
    /// Maximum primary controls for novice use.
    pub maximum_primary_controls: u32,
    /// Control rows.
    pub controls: Vec<DraftControl>,
}

/// Draft candidate strategy.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DraftCandidateStrategy {
    /// Stable strategy ID.
    pub strategy_id: String,
    /// Product-facing name.
    pub name: String,
    /// Product-facing explanation.
    pub explanation: String,
    /// Visible controls this strategy may change.
    pub allowed_controls: Vec<String>,
    /// Provider slots this strategy may change.
    pub allowed_provider_changes: Vec<String>,
}

/// Draft candidate strategy pack.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DraftCandidateStrategyPack {
    /// Stable pack ID.
    pub pack_id: String,
    /// Strategies.
    pub strategies: Vec<DraftCandidateStrategy>,
    /// Diversity goals.
    pub diversity_goals: Vec<String>,
}

/// Draft style/provider compatibility rule.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DraftCompatibilityRule {
    /// Style ID.
    pub style_id: String,
    /// Provider pack ID.
    pub provider_pack_id: String,
    /// Whether this pair is compatible.
    pub compatible: bool,
    /// Product-safe reason.
    pub reason: String,
}

/// Draft compatibility matrix.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DraftCompatibilityMatrix {
    /// Matrix ID.
    pub matrix_id: String,
    /// Compatibility rules.
    pub rules: Vec<DraftCompatibilityRule>,
}

/// Draft quality gate profile.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DraftQualityGateProfile {
    /// Profile ID.
    pub profile_id: String,
    /// Validation gate required.
    pub validation_required: bool,
    /// Contact sheet gate required.
    pub contact_sheet_required: bool,
    /// Package/export gate required.
    pub package_required: bool,
    /// Human review gate required.
    pub human_review_required: bool,
    /// Adversarial review gate required.
    pub adversarial_review_required: bool,
    /// Manual review gates.
    pub manual_review_gates: Vec<String>,
}

/// Draft test plan.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DraftTestPlan {
    /// Test plan ID.
    pub test_plan_id: String,
    /// Deterministic tests expected before promotion.
    pub tests: Vec<String>,
}

/// Draft review checklist.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DraftReviewChecklist {
    /// Checklist ID.
    pub checklist_id: String,
    /// Review checklist items.
    pub items: Vec<String>,
}

/// Draft repair suggestion.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DraftRepairSuggestion {
    /// Schema version.
    pub schema_version: u32,
    /// Draft ID being repaired.
    pub draft_id: String,
    /// Validation report reference.
    pub validation_report_ref: String,
    /// Suggested repair rows.
    pub suggestions: Vec<String>,
}

/// One adversarial review question.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DraftAdversarialQuestion {
    /// Stable question ID.
    pub question_id: String,
    /// Question text.
    pub question: String,
    /// Deterministic finding for the current draft.
    pub finding: String,
}

/// Deterministic adversarial report for one draft.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DraftAdversarialReport {
    /// Schema version.
    pub schema_version: u32,
    /// Draft ID.
    pub draft_id: String,
    /// Questions and findings.
    pub questions: Vec<DraftAdversarialQuestion>,
    /// Missing geometry/art ingredients.
    pub missing_geometry_art_ingredients: Vec<String>,
    /// Human review requirements.
    pub human_review_required: Vec<String>,
    /// Summary.
    pub summary: String,
}

/// Top-level foundation draft.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FoundryFoundationDraft {
    /// Draft schema version.
    pub schema_version: u32,
    /// Stable draft ID.
    pub draft_id: String,
    /// Source kind.
    pub source_kind: FoundationDraftSourceKind,
    /// Requested quality target.
    pub quality_target: FoundationQualityTarget,
    /// Catalog visibility policy.
    pub catalog_visibility: FoundationCatalogVisibility,
    /// Human review is required before publication.
    pub human_review_required: bool,
    /// Publishing is disabled by default.
    pub publish_allowed: bool,
    /// Product category.
    pub category: String,
    /// Family blueprint draft.
    pub family_blueprint: DraftFamilyBlueprint,
    /// Provider taxonomy draft.
    pub provider_taxonomy: DraftProviderTaxonomy,
    /// Style pack draft.
    pub style_pack: DraftStylePack,
    /// Control profile draft.
    pub control_profile: DraftControlProfile,
    /// Candidate strategy draft.
    pub candidate_strategy_pack: DraftCandidateStrategyPack,
    /// Compatibility matrix draft.
    pub compatibility_matrix: DraftCompatibilityMatrix,
    /// Quality gate draft.
    pub quality_gate_profile: Option<DraftQualityGateProfile>,
    /// Test plan draft.
    pub test_plan: DraftTestPlan,
    /// Review checklist draft.
    pub review_checklist: DraftReviewChecklist,
    /// Executed or proposed allowed authoring commands.
    #[serde(default)]
    pub command_log: Vec<FoundationAuthoringCommand>,
    /// Rejected raw command names observed in an imported draft.
    #[serde(default)]
    pub rejected_command_attempts: Vec<String>,
    /// Direct geometry payload attempt markers.
    #[serde(default)]
    pub direct_geometry_payload_attempts: Vec<String>,
}

/// Archetype materializer report schema version.
pub const ARCHETYPE_DRAFT_MATERIALIZATION_REPORT_SCHEMA_VERSION: u32 = 1;
/// The only archetype supported by the v0 draft materializer.
pub const BOX_PRIMITIVE_MATERIALIZER_ARCHETYPE_ID: &str = "box-primitive";

/// Deterministic report for one archetype draft materialization run.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ArchetypeDraftMaterializationReport {
    /// Report schema version.
    pub schema_version: u32,
    /// Archetype ID requested.
    pub archetype_id: String,
    /// Generated family ID.
    pub family_id: String,
    /// Generated style ID.
    pub style_id: String,
    /// Drafts are not publishable.
    pub publish_allowed: bool,
    /// Drafts are hidden from novice catalog.
    pub novice_visible: bool,
    /// Human review is required.
    pub human_review_required: bool,
    /// Showcase is not allowed for drafts.
    pub showcase_allowed: bool,
    /// Whether direct geometry payloads are present.
    pub geometry_payload_present: bool,
    /// Whether raw vertex payloads are present.
    pub raw_vertex_payload_present: bool,
    /// Missing taste-bearing provider work that must be authored/reviewed.
    pub missing_taste_bearing_providers: Vec<String>,
    /// Validation issue count.
    pub validation_issue_count: usize,
    /// Generated files.
    pub generated_files: Vec<String>,
}

/// LLM-callable, SDK-free authoring command contract.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
#[serde(tag = "command", content = "args")]
pub enum FoundationAuthoringCommand {
    /// Create or replace the family blueprint header.
    CreateFamilyBlueprint {
        /// Family ID.
        family_id: String,
        /// Human-facing family name.
        display_name: String,
    },
    /// Add a role.
    AddRole {
        /// Role ID.
        role_id: String,
        /// Product-facing label.
        label: String,
        /// Whether the role is required.
        required: bool,
    },
    /// Add a required role ID.
    AddRequiredRole {
        /// Role ID.
        role_id: String,
    },
    /// Add an optional role ID.
    AddOptionalRole {
        /// Role ID.
        role_id: String,
    },
    /// Add a socket.
    AddSocket {
        /// Socket ID.
        socket_id: String,
        /// Source role.
        from_role: String,
        /// Destination role.
        to_role: String,
    },
    /// Add a provider slot.
    AddProviderSlot {
        /// Slot ID.
        slot_id: String,
        /// Role supplied by the slot.
        role_id: String,
    },
    /// Attach a provider pack.
    AttachProviderPack {
        /// Provider pack ID.
        pack_id: String,
        /// Provider slot IDs.
        supplied_slots: Vec<String>,
    },
    /// Create or replace style pack header.
    CreateStylePack {
        /// Style ID.
        style_id: String,
        /// Human-facing display name.
        display_name: String,
    },
    /// Set one compatibility rule.
    SetCompatibilityRule {
        /// Style ID.
        style_id: String,
        /// Provider pack ID.
        provider_pack_id: String,
        /// Whether compatible.
        compatible: bool,
        /// Product-safe reason.
        reason: String,
    },
    /// Create or reset control profile.
    CreateControlProfile {
        /// Profile ID.
        profile_id: String,
    },
    /// Create one candidate strategy.
    CreateCandidateStrategy {
        /// Strategy ID.
        strategy_id: String,
        /// Name.
        name: String,
        /// Allowed controls.
        allowed_controls: Vec<String>,
    },
    /// Create or replace quality gate profile.
    CreateQualityGateProfile {
        /// Profile ID.
        profile_id: String,
        /// Contact sheet required.
        contact_sheet_required: bool,
    },
    /// Request contact sheet rendering through existing gates.
    RenderContactSheet {
        /// Output directory ref.
        out_dir: String,
    },
    /// Request validation.
    ValidateKit,
    /// Request package export.
    PackageKit {
        /// Output directory ref.
        out_dir: String,
    },
    /// Explain one validation failure.
    ExplainValidationFailure {
        /// Issue code.
        issue_code: String,
    },
    /// Suggest repair for one issue.
    SuggestRepair {
        /// Issue code.
        issue_code: String,
    },
}

/// Foundation draft validation issue.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FoundationDraftValidationIssue {
    /// Stable subject path.
    pub subject: String,
    /// Stable issue code.
    pub code: String,
    /// Human-readable message.
    pub message: String,
}

/// Foundation draft validation report.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FoundationDraftValidationReport {
    /// Validation issues.
    pub issues: Vec<FoundationDraftValidationIssue>,
}

impl FoundationDraftValidationReport {
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
        self.issues.push(FoundationDraftValidationIssue {
            subject: subject.into(),
            code: code.into(),
            message: message.into(),
        });
    }
}

/// Error returned by foundation command execution.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FoundationCommandError {
    message: String,
}

impl FoundationCommandError {
    fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl fmt::Display for FoundationCommandError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl std::error::Error for FoundationCommandError {}
