//! SDK-free foundation draft contracts for LLM-assisted Foundry authoring.
//!
//! This module models structured draft kit foundations. It does not call an
//! LLM, generate meshes, inject geometry payloads, mutate recipes directly, or
//! publish content to the novice catalog.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

use serde::{Deserialize, Serialize};

use crate::{
    AttachmentExpectation, CANDIDATE_STRATEGY_PACK_SCHEMA_VERSION, CONTROL_PROFILE_SCHEMA_VERSION,
    CandidateStrategyPack, CatalogVisibilityPolicy, ControlOptionVisibility, ControlProfile,
    ControlProfileControl, ControlProfileControlKind, ControlProfileTopologyBehavior,
    DEFAULT_MAX_PRIMARY_NOVICE_CONTROLS, ExportPartNamingPolicy, FAMILY_BLUEPRINT_SCHEMA_VERSION,
    FOUNDRY_KIT_PACKAGE_SCHEMA_VERSION, FOUNDRY_KIT_SCHEMA_VERSION, FamilyBlueprint,
    FamilyBlueprintRole, FoundryKit, FoundryKitPackage, FoundryKitQualityTier,
    FutureMaterialVocabulary, HighLevelScalePolicy, KIT_CATALOG_MANIFEST_SCHEMA_VERSION,
    KIT_COMPATIBILITY_MATRIX_SCHEMA_VERSION, KIT_REVIEW_MANIFEST_SCHEMA_VERSION,
    KitCandidateStrategy, KitCatalogManifest, KitCompatibilityMatrix, KitReviewManifest,
    PROVIDER_PACK_SCHEMA_VERSION, PreviewCameraPolicy, ProviderPack, ProviderPackOption,
    ProviderSlotExpectation, QUALITY_GATE_PROFILE_SCHEMA_VERSION, QualityGateProfile,
    STYLE_PACK_SCHEMA_VERSION, StylePack, StyleProviderCompatibility, StyleProviderIncompatibility,
    validate_foundry_kit_package,
};

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

/// Return the command names that are explicitly forbidden.
#[must_use]
pub fn forbidden_foundation_command_names() -> &'static [&'static str] {
    &[
        "SetRawVertexPositions",
        "InjectMeshPayload",
        "BypassValidation",
        "SilentlyFixBrokenTopology",
        "CreateUnboundedRandomVariants",
        "PublishToNoviceCatalog",
        "MutateRecipeDirectly",
        "HideValidationFailure",
    ]
}

/// Parse an authoring command from JSON.
pub fn parse_foundation_authoring_command_json(
    json: &str,
) -> Result<FoundationAuthoringCommand, serde_json::Error> {
    serde_json::from_str(json)
}

/// Execute an allowed foundation authoring command against a draft.
pub fn execute_foundation_authoring_command(
    draft: &mut FoundryFoundationDraft,
    command: FoundationAuthoringCommand,
) -> Result<(), FoundationCommandError> {
    match &command {
        FoundationAuthoringCommand::CreateFamilyBlueprint {
            family_id,
            display_name,
        } => {
            draft.family_blueprint.family_id = family_id.clone();
            draft.family_blueprint.display_name = display_name.clone();
        }
        FoundationAuthoringCommand::AddRole {
            role_id,
            label,
            required,
        } => {
            if draft
                .family_blueprint
                .roles
                .iter()
                .any(|role| role.role_id == *role_id)
            {
                return Err(FoundationCommandError::new(format!(
                    "Role '{role_id}' already exists."
                )));
            }
            draft.family_blueprint.roles.push(DraftFamilyRole {
                role_id: role_id.clone(),
                label: label.clone(),
                required: *required,
                tags: Vec::new(),
            });
            if *required && !draft.family_blueprint.required_roles.contains(role_id) {
                draft.family_blueprint.required_roles.push(role_id.clone());
            }
        }
        FoundationAuthoringCommand::AddRequiredRole { role_id } => {
            if !draft.family_blueprint.required_roles.contains(role_id) {
                draft.family_blueprint.required_roles.push(role_id.clone());
            }
        }
        FoundationAuthoringCommand::AddOptionalRole { role_id } => {
            if !draft.family_blueprint.optional_roles.contains(role_id) {
                draft.family_blueprint.optional_roles.push(role_id.clone());
            }
        }
        FoundationAuthoringCommand::AddSocket {
            socket_id,
            from_role,
            to_role,
        } => draft.family_blueprint.sockets.push(DraftSocket {
            socket_id: socket_id.clone(),
            from_role: from_role.clone(),
            to_role: to_role.clone(),
            compatibility_tags: vec!["authored_attachment".to_owned()],
            required: true,
        }),
        FoundationAuthoringCommand::AddProviderSlot { slot_id, role_id } => {
            draft
                .provider_taxonomy
                .provider_slots
                .push(DraftProviderSlot {
                    slot_id: slot_id.clone(),
                    role_id: role_id.clone(),
                    required: true,
                    compatibility_tags: vec!["draft_provider".to_owned()],
                });
        }
        FoundationAuthoringCommand::AttachProviderPack {
            pack_id,
            supplied_slots,
        } => draft
            .provider_taxonomy
            .provider_packs
            .push(DraftProviderPack {
                pack_id: pack_id.clone(),
                label: title_from_id(pack_id),
                supplied_slots: supplied_slots.clone(),
                compatibility_tags: vec!["draft_provider".to_owned()],
            }),
        FoundationAuthoringCommand::CreateStylePack {
            style_id,
            display_name,
        } => {
            draft.style_pack.style_id = style_id.clone();
            draft.style_pack.display_name = display_name.clone();
        }
        FoundationAuthoringCommand::SetCompatibilityRule {
            style_id,
            provider_pack_id,
            compatible,
            reason,
        } => draft
            .compatibility_matrix
            .rules
            .push(DraftCompatibilityRule {
                style_id: style_id.clone(),
                provider_pack_id: provider_pack_id.clone(),
                compatible: *compatible,
                reason: reason.clone(),
            }),
        FoundationAuthoringCommand::CreateControlProfile { profile_id } => {
            draft.control_profile.profile_id = profile_id.clone();
            draft.control_profile.controls.clear();
        }
        FoundationAuthoringCommand::CreateCandidateStrategy {
            strategy_id,
            name,
            allowed_controls,
        } => draft
            .candidate_strategy_pack
            .strategies
            .push(DraftCandidateStrategy {
                strategy_id: strategy_id.clone(),
                name: name.clone(),
                explanation: format!("{name} adjusts visible controls."),
                allowed_controls: allowed_controls.clone(),
                allowed_provider_changes: Vec::new(),
            }),
        FoundationAuthoringCommand::CreateQualityGateProfile {
            profile_id,
            contact_sheet_required,
        } => {
            draft.quality_gate_profile = Some(DraftQualityGateProfile {
                profile_id: profile_id.clone(),
                validation_required: true,
                contact_sheet_required: *contact_sheet_required,
                package_required: true,
                human_review_required: true,
                adversarial_review_required: matches!(
                    draft.quality_target,
                    FoundationQualityTarget::Showcase
                ),
                manual_review_gates: vec!["Human visual review required.".to_owned()],
            });
        }
        FoundationAuthoringCommand::RenderContactSheet { .. }
        | FoundationAuthoringCommand::ValidateKit
        | FoundationAuthoringCommand::PackageKit { .. }
        | FoundationAuthoringCommand::ExplainValidationFailure { .. }
        | FoundationAuthoringCommand::SuggestRepair { .. } => {}
    }
    draft.command_log.push(command);
    Ok(())
}

/// Create a deterministic foundation draft template.
#[must_use]
pub fn foundation_draft_template(
    category: impl Into<String>,
    family_id: impl Into<String>,
) -> FoundryFoundationDraft {
    let category = category.into();
    let family_id = normalize_id(&family_id.into());
    let display_name = title_from_id(&family_id);
    let primary_role = primary_role_for_family(&family_id);
    let secondary_role = secondary_role_for_family(&family_id);
    let primary_slot = format!("{primary_role}_slot");
    let secondary_slot = format!("{secondary_role}_slot");
    let style_id = format!("{family_id}_foundation_style");
    let provider_pack_id = format!("{family_id}_foundation_providers");
    let control_id = format!("{primary_role}_shape");

    FoundryFoundationDraft {
        schema_version: FOUNDRY_FOUNDATION_DRAFT_SCHEMA_VERSION,
        draft_id: format!("{family_id}_foundation_draft"),
        source_kind: FoundationDraftSourceKind::Human,
        quality_target: FoundationQualityTarget::Draft,
        catalog_visibility: FoundationCatalogVisibility::InternalOnly,
        human_review_required: true,
        publish_allowed: false,
        category,
        family_blueprint: DraftFamilyBlueprint {
            family_id: family_id.clone(),
            display_name,
            roles: vec![
                DraftFamilyRole {
                    role_id: primary_role.clone(),
                    label: title_from_id(&primary_role),
                    required: true,
                    tags: vec!["primary".to_owned()],
                },
                DraftFamilyRole {
                    role_id: secondary_role.clone(),
                    label: title_from_id(&secondary_role),
                    required: false,
                    tags: vec!["support".to_owned()],
                },
            ],
            required_roles: vec![primary_role.clone()],
            optional_roles: vec![secondary_role.clone()],
            sockets: vec![DraftSocket {
                socket_id: format!("{secondary_role}_to_{primary_role}"),
                from_role: secondary_role.clone(),
                to_role: primary_role.clone(),
                compatibility_tags: vec!["foundation_attachment".to_owned()],
                required: false,
            }],
            export_part_names: vec![title_from_id(&primary_role)],
        },
        provider_taxonomy: DraftProviderTaxonomy {
            taxonomy_id: format!("{family_id}_provider_taxonomy"),
            provider_slots: vec![
                DraftProviderSlot {
                    slot_id: primary_slot.clone(),
                    role_id: primary_role.clone(),
                    required: true,
                    compatibility_tags: vec!["foundation".to_owned()],
                },
                DraftProviderSlot {
                    slot_id: secondary_slot.clone(),
                    role_id: secondary_role.clone(),
                    required: false,
                    compatibility_tags: vec!["foundation".to_owned()],
                },
            ],
            provider_packs: vec![DraftProviderPack {
                pack_id: provider_pack_id.clone(),
                label: format!("{} Foundation Providers", title_from_id(&family_id)),
                supplied_slots: vec![primary_slot.clone(), secondary_slot.clone()],
                compatibility_tags: vec!["foundation".to_owned()],
            }],
        },
        style_pack: DraftStylePack {
            style_id: style_id.clone(),
            display_name: "Foundation Style".to_owned(),
            bevel_language: "Readable broad forms first; details require later art review."
                .to_owned(),
            proportion_language: "Use clear whole-model proportions suitable for a first pass."
                .to_owned(),
            detail_density_policy: "Keep details sparse until geometry is authored.".to_owned(),
            silhouette_policy: "Prioritize recognizable whole-model silhouette.".to_owned(),
            symmetry_policy: "Default to symmetric foundations unless the brief says otherwise."
                .to_owned(),
            allowed_provider_tags: vec!["foundation".to_owned()],
            forbidden_provider_tags: vec!["photoreal_material".to_owned()],
            compatibility_style_ids: Vec::new(),
        },
        control_profile: DraftControlProfile {
            profile_id: format!("{family_id}_foundation_controls"),
            maximum_primary_controls: DEFAULT_MAX_PRIMARY_NOVICE_CONTROLS,
            controls: vec![DraftControl {
                control_id: control_id.clone(),
                label: format!("{} Shape", title_from_id(&primary_role)),
                description: "Choose the main whole-model silhouette.".to_owned(),
                kind: ControlProfileControlKind::Choice,
                primary: true,
                visible: true,
                owned_family_slots: Vec::new(),
                owned_provider_slots: vec![primary_slot.clone()],
                topology_behavior: ControlProfileTopologyBehavior::TopologyChanging,
                visible_effect_expectation: "The main silhouette changes visibly.".to_owned(),
            }],
        },
        candidate_strategy_pack: DraftCandidateStrategyPack {
            pack_id: format!("{family_id}_foundation_strategies"),
            strategies: vec![DraftCandidateStrategy {
                strategy_id: "balanced".to_owned(),
                name: "Balanced".to_owned(),
                explanation: "Balanced foundation direction using visible controls.".to_owned(),
                allowed_controls: vec![control_id],
                allowed_provider_changes: vec![primary_slot],
            }],
            diversity_goals: vec!["silhouette".to_owned(), "proportion".to_owned()],
        },
        compatibility_matrix: DraftCompatibilityMatrix {
            matrix_id: format!("{family_id}_foundation_compatibility"),
            rules: vec![DraftCompatibilityRule {
                style_id,
                provider_pack_id,
                compatible: true,
                reason: "Foundation style and provider taxonomy share foundation tags.".to_owned(),
            }],
        },
        quality_gate_profile: Some(DraftQualityGateProfile {
            profile_id: format!("{family_id}_foundation_quality"),
            validation_required: true,
            contact_sheet_required: false,
            package_required: true,
            human_review_required: true,
            adversarial_review_required: false,
            manual_review_gates: vec!["Human review required before catalog exposure.".to_owned()],
        }),
        test_plan: DraftTestPlan {
            test_plan_id: format!("{family_id}_foundation_tests"),
            tests: vec![
                "Validate foundation draft schema.".to_owned(),
                "Materialized package remains Draft/Internal.".to_owned(),
            ],
        },
        review_checklist: DraftReviewChecklist {
            checklist_id: format!("{family_id}_foundation_review"),
            items: vec![
                "Confirm geometry/art ingredients are supplied by humans or reviewed tools."
                    .to_owned(),
                "Confirm no novice catalog visibility is enabled.".to_owned(),
            ],
        },
        command_log: Vec::new(),
        rejected_command_attempts: Vec::new(),
        direct_geometry_payload_attempts: Vec::new(),
    }
}

/// Materialize a structured internal draft from a supported archetype.
pub fn materialize_archetype_foundation_draft(
    archetype_id: &str,
    family_id: &str,
    style_id: &str,
) -> Result<FoundryFoundationDraft, String> {
    let normalized_archetype = archetype_id.trim().replace('_', "-").to_ascii_lowercase();
    if normalized_archetype != BOX_PRIMITIVE_MATERIALIZER_ARCHETYPE_ID {
        return Err(format!(
            "unsupported archetype '{archetype_id}'; v0 supports box-primitive only"
        ));
    }
    let family_id = normalize_id(family_id);
    let style_id = normalize_id(style_id);
    if family_id.is_empty() || style_id.is_empty() {
        return Err("family-id and style-id must normalize to non-empty IDs".to_owned());
    }
    Ok(box_primitive_archetype_draft(&family_id, &style_id))
}

/// Build a materialization report for an archetype draft.
#[must_use]
pub fn archetype_draft_materialization_report(
    draft: &FoundryFoundationDraft,
    generated_files: Vec<String>,
) -> ArchetypeDraftMaterializationReport {
    let validation = validate_foundation_draft(draft);
    ArchetypeDraftMaterializationReport {
        schema_version: ARCHETYPE_DRAFT_MATERIALIZATION_REPORT_SCHEMA_VERSION,
        archetype_id: BOX_PRIMITIVE_MATERIALIZER_ARCHETYPE_ID.to_owned(),
        family_id: draft.family_blueprint.family_id.clone(),
        style_id: draft.style_pack.style_id.clone(),
        publish_allowed: draft.publish_allowed,
        novice_visible: matches!(
            draft.catalog_visibility,
            FoundationCatalogVisibility::NoviceCatalog
        ),
        human_review_required: draft.human_review_required,
        showcase_allowed: false,
        geometry_payload_present: !draft.direct_geometry_payload_attempts.is_empty(),
        raw_vertex_payload_present: draft
            .direct_geometry_payload_attempts
            .iter()
            .any(|attempt| attempt.to_ascii_lowercase().contains("vertex")),
        missing_taste_bearing_providers: vec![
            "authored Box Primitive provider geometry choices".to_owned(),
            "contact sheets and human review before any profile promotion".to_owned(),
        ],
        validation_issue_count: validation.issues.len(),
        generated_files,
    }
}

fn box_primitive_archetype_draft(family_id: &str, style_id: &str) -> FoundryFoundationDraft {
    let required_roles = ["body"];
    let optional_roles: [&str; 0] = [];
    let roles = required_roles
        .iter()
        .map(|role| DraftFamilyRole {
            role_id: (*role).to_owned(),
            label: title_from_id(role),
            required: true,
            tags: vec!["box_primitive".to_owned(), "required".to_owned()],
        })
        .chain(optional_roles.iter().map(|role| DraftFamilyRole {
            role_id: (*role).to_owned(),
            label: title_from_id(role),
            required: false,
            tags: vec!["box_primitive".to_owned(), "optional".to_owned()],
        }))
        .collect::<Vec<_>>();
    let provider_slots = roles
        .iter()
        .map(|role| DraftProviderSlot {
            slot_id: format!("{}_slot", role.role_id),
            role_id: role.role_id.clone(),
            required: role.required,
            compatibility_tags: vec!["box_primitive".to_owned()],
        })
        .collect::<Vec<_>>();
    let slot_ids = provider_slots
        .iter()
        .map(|slot| slot.slot_id.clone())
        .collect::<Vec<_>>();
    let provider_pack_id = format!("{family_id}_draft_providers");
    let quality_profile_id = format!("{family_id}_draft_quality");
    let matrix_id = format!("{family_id}_draft_compatibility");
    let draft_id = format!("{family_id}_box_primitive_archetype_draft");

    FoundryFoundationDraft {
        schema_version: FOUNDRY_FOUNDATION_DRAFT_SCHEMA_VERSION,
        draft_id: draft_id.clone(),
        source_kind: FoundationDraftSourceKind::GeneratedFixture,
        quality_target: FoundationQualityTarget::Draft,
        catalog_visibility: FoundationCatalogVisibility::InternalOnly,
        human_review_required: true,
        publish_allowed: false,
        category: "box-primitive".to_owned(),
        family_blueprint: DraftFamilyBlueprint {
            family_id: family_id.to_owned(),
            display_name: title_from_id(family_id),
            roles,
            required_roles: required_roles
                .iter()
                .map(|role| (*role).to_owned())
                .collect(),
            optional_roles: optional_roles
                .iter()
                .map(|role| (*role).to_owned())
                .collect(),
            sockets: Vec::new(),
            export_part_names: required_roles
                .iter()
                .map(|role| title_from_id(role))
                .collect(),
        },
        provider_taxonomy: DraftProviderTaxonomy {
            taxonomy_id: format!("{family_id}_box_primitive_provider_taxonomy"),
            provider_slots,
            provider_packs: vec![DraftProviderPack {
                pack_id: provider_pack_id.clone(),
                label: format!("{} Box Primitive Draft Providers", title_from_id(family_id)),
                supplied_slots: slot_ids.clone(),
                compatibility_tags: vec!["box_primitive".to_owned()],
            }],
        },
        style_pack: DraftStylePack {
            style_id: style_id.to_owned(),
            display_name: title_from_id(style_id),
            bevel_language: "Box Primitive uses simple edge softness only.".to_owned(),
            proportion_language: "Use visible box proportions without changing topology."
                .to_owned(),
            detail_density_policy: "No detail modules are part of the Box Primitive baseline."
                .to_owned(),
            silhouette_policy: "Preserve a readable closed box silhouette in pure clay.".to_owned(),
            symmetry_policy: "Default to axis-aligned bilateral symmetry.".to_owned(),
            allowed_provider_tags: vec!["box_primitive".to_owned()],
            forbidden_provider_tags: vec!["raw_mesh_payload".to_owned()],
            compatibility_style_ids: Vec::new(),
        },
        control_profile: DraftControlProfile {
            profile_id: format!("{family_id}_box_primitive_controls"),
            maximum_primary_controls: DEFAULT_MAX_PRIMARY_NOVICE_CONTROLS,
            controls: box_primitive_draft_controls(),
        },
        candidate_strategy_pack: DraftCandidateStrategyPack {
            pack_id: format!("{family_id}_box_primitive_strategies"),
            strategies: box_primitive_draft_strategies(),
            diversity_goals: vec![
                "whole-asset proportions".to_owned(),
                "edge softness endpoints".to_owned(),
            ],
        },
        compatibility_matrix: DraftCompatibilityMatrix {
            matrix_id,
            rules: vec![DraftCompatibilityRule {
                style_id: style_id.to_owned(),
                provider_pack_id,
                compatible: true,
                reason: "Box Primitive archetype draft uses matching box_primitive tags."
                    .to_owned(),
            }],
        },
        quality_gate_profile: Some(DraftQualityGateProfile {
            profile_id: quality_profile_id,
            validation_required: true,
            contact_sheet_required: true,
            package_required: true,
            human_review_required: true,
            adversarial_review_required: true,
            manual_review_gates: vec![
                "Pure Clay and Semantic Clay contact sheets required before promotion.".to_owned(),
                "Human/adversarial review required before catalog visibility.".to_owned(),
            ],
        }),
        test_plan: DraftTestPlan {
            test_plan_id: format!("{family_id}_box_primitive_test_plan"),
            tests: vec![
                "Validate generated draft schema.".to_owned(),
                "Confirm publish_allowed false and novice_visible false.".to_owned(),
                "Reject raw geometry or vertex payloads.".to_owned(),
                "Generate contact sheets before profile promotion.".to_owned(),
            ],
        },
        review_checklist: DraftReviewChecklist {
            checklist_id: format!("{family_id}_box_primitive_review"),
            items: vec![
                "Confirm the required Box Primitive body role is present.".to_owned(),
                "Confirm taste-bearing providers are authored, not generated as raw vertices."
                    .to_owned(),
                "Confirm no novice catalog visibility before human review.".to_owned(),
            ],
        },
        command_log: Vec::new(),
        rejected_command_attempts: Vec::new(),
        direct_geometry_payload_attempts: Vec::new(),
    }
}

fn box_primitive_draft_controls() -> Vec<DraftControl> {
    vec![
        draft_control(
            "proportions",
            "Proportions",
            &["body_proportions"],
            &[],
            ControlProfileTopologyBehavior::TopologyPreserving,
        ),
        draft_control(
            "edge_softness",
            "Edge Softness",
            &["body_edge_softness"],
            &[],
            ControlProfileTopologyBehavior::TopologyPreserving,
        ),
    ]
}

fn draft_control(
    control_id: &str,
    label: &str,
    family_slots: &[&str],
    provider_slots: &[&str],
    topology_behavior: ControlProfileTopologyBehavior,
) -> DraftControl {
    DraftControl {
        control_id: control_id.to_owned(),
        label: label.to_owned(),
        description: format!("{label} must visibly change the Box Primitive clay preview."),
        kind: if matches!(
            topology_behavior,
            ControlProfileTopologyBehavior::TopologyPreserving
        ) {
            ControlProfileControlKind::Continuous
        } else {
            ControlProfileControlKind::Choice
        },
        primary: true,
        visible: true,
        owned_family_slots: family_slots.iter().map(|slot| (*slot).to_owned()).collect(),
        owned_provider_slots: provider_slots
            .iter()
            .map(|slot| (*slot).to_owned())
            .collect(),
        topology_behavior,
        visible_effect_expectation: "Visible in Pure Clay before semantic display assistance."
            .to_owned(),
    }
}

fn box_primitive_draft_strategies() -> Vec<DraftCandidateStrategy> {
    [
        ("compact_box", "Compact Box", &["proportions"][..]),
        ("wide_box", "Wide Box", &["proportions"][..]),
        ("tall_box", "Tall Box", &["proportions"][..]),
        ("flat_box", "Flat Box", &["proportions"][..]),
        ("soft_edged_box", "Soft-Edged Box", &["edge_softness"][..]),
        ("sharp_utility_box", "Sharp Box", &["edge_softness"][..]),
    ]
    .into_iter()
    .map(|(id, name, controls)| DraftCandidateStrategy {
        strategy_id: id.to_owned(),
        name: name.to_owned(),
        explanation: format!("{name} Box Primitive draft direction through visible controls."),
        allowed_controls: controls
            .iter()
            .map(|control| (*control).to_owned())
            .collect(),
        allowed_provider_changes: Vec::new(),
    })
    .collect()
}

/// Return deterministic internal foundation fixtures.
#[must_use]
pub fn foundation_draft_fixtures() -> Vec<FoundryFoundationDraft> {
    [("boxes", "box_primitive_core")]
        .into_iter()
        .map(|(category, family)| {
            let mut draft = foundation_draft_template(category, family);
            draft.source_kind = FoundationDraftSourceKind::GeneratedFixture;
            draft.draft_id = format!("{family}_draft");
            draft.quality_target = FoundationQualityTarget::Draft;
            draft.catalog_visibility = FoundationCatalogVisibility::InternalOnly;
            draft.human_review_required = true;
            draft.publish_allowed = false;
            draft
        })
        .collect()
}

/// Validate a foundation draft.
#[must_use]
pub fn validate_foundation_draft(
    draft: &FoundryFoundationDraft,
) -> FoundationDraftValidationReport {
    let mut report = FoundationDraftValidationReport::default();
    if draft.schema_version != FOUNDRY_FOUNDATION_DRAFT_SCHEMA_VERSION {
        report.push(
            "schema_version",
            "unsupported_foundation_draft_schema",
            "Unsupported foundation draft schema version.",
        );
    }
    if draft.draft_id.trim().is_empty() {
        report.push(
            "draft_id",
            "missing_draft_id",
            "Foundation drafts require a stable draft ID.",
        );
    }
    if draft.publish_allowed {
        report.push(
            "publish_allowed",
            "foundation_publish_not_allowed",
            "Foundation drafts cannot publish directly in Wave 36.",
        );
    }
    if draft.publish_allowed && !draft.human_review_required {
        report.push(
            "publish_allowed",
            "publish_requires_human_review",
            "Publishing cannot be allowed without human review.",
        );
    }
    if draft.catalog_visibility != FoundationCatalogVisibility::InternalOnly {
        report.push(
            "catalog_visibility",
            "foundation_draft_must_remain_internal",
            "Foundation drafts must remain internal-only in Wave 36.",
        );
    }
    if matches!(
        draft.quality_target,
        FoundationQualityTarget::Draft | FoundationQualityTarget::Prototype
    ) && draft.catalog_visibility == FoundationCatalogVisibility::NoviceCatalog
    {
        report.push(
            "catalog_visibility",
            "draft_or_prototype_cannot_be_novice_visible",
            "Draft and Prototype foundation drafts cannot be visible in the novice catalog.",
        );
    }
    validate_roles(draft, &mut report);
    validate_provider_slots(draft, &mut report);
    validate_style_compatibility(draft, &mut report);
    validate_controls(draft, &mut report);
    validate_candidate_strategies(draft, &mut report);
    validate_quality_gate(draft, &mut report);
    validate_forbidden_attempts(draft, &mut report);
    report
}

fn validate_roles(draft: &FoundryFoundationDraft, report: &mut FoundationDraftValidationReport) {
    let role_ids = draft
        .family_blueprint
        .roles
        .iter()
        .map(|role| role.role_id.as_str())
        .collect::<BTreeSet<_>>();
    if draft.family_blueprint.required_roles.is_empty() {
        report.push(
            "family_blueprint.required_roles",
            "missing_required_roles",
            "Foundation drafts require at least one required role.",
        );
    }
    for role_id in &draft.family_blueprint.required_roles {
        if !role_ids.contains(role_id.as_str()) {
            report.push(
                format!("family_blueprint.required_roles.{role_id}"),
                "missing_required_role_definition",
                "Required roles must exist in the family role inventory.",
            );
        }
    }
    for role in &draft.family_blueprint.roles {
        if contains_raw_authoring_marker(&role.label) {
            report.push(
                format!("family_blueprint.roles.{}.label", role.role_id),
                "technical_term_in_novice_label",
                "Novice-facing role labels must not expose technical authoring terms.",
            );
        }
    }
}

fn validate_provider_slots(
    draft: &FoundryFoundationDraft,
    report: &mut FoundationDraftValidationReport,
) {
    let role_ids = draft
        .family_blueprint
        .roles
        .iter()
        .map(|role| role.role_id.as_str())
        .collect::<BTreeSet<_>>();
    let provider_slots = draft
        .provider_taxonomy
        .provider_slots
        .iter()
        .map(|slot| slot.slot_id.as_str())
        .collect::<BTreeSet<_>>();
    if draft.provider_taxonomy.provider_slots.is_empty() {
        report.push(
            "provider_taxonomy.provider_slots",
            "missing_provider_slots",
            "Foundation drafts require provider slots for authored geometry providers.",
        );
    }
    for role_id in &draft.family_blueprint.required_roles {
        let covered = draft
            .provider_taxonomy
            .provider_slots
            .iter()
            .any(|slot| slot.required && slot.role_id == *role_id);
        if !covered {
            report.push(
                format!("provider_taxonomy.provider_slots.{role_id}"),
                "missing_provider_slot_for_required_role",
                "Every required role needs a required provider slot.",
            );
        }
    }
    for slot in &draft.provider_taxonomy.provider_slots {
        if !role_ids.contains(slot.role_id.as_str()) {
            report.push(
                format!("provider_taxonomy.provider_slots.{}", slot.slot_id),
                "provider_slot_unknown_role",
                "Provider slots must reference known roles.",
            );
        }
    }
    for provider_pack in &draft.provider_taxonomy.provider_packs {
        for slot_id in &provider_pack.supplied_slots {
            if !provider_slots.contains(slot_id.as_str()) {
                report.push(
                    format!("provider_taxonomy.provider_packs.{}", provider_pack.pack_id),
                    "provider_pack_unknown_slot",
                    "Provider packs must reference known provider slots.",
                );
            }
        }
    }
}

fn validate_style_compatibility(
    draft: &FoundryFoundationDraft,
    report: &mut FoundationDraftValidationReport,
) {
    let allowed = draft
        .style_pack
        .allowed_provider_tags
        .iter()
        .map(String::as_str)
        .collect::<BTreeSet<_>>();
    for tag in &draft.style_pack.forbidden_provider_tags {
        if allowed.contains(tag.as_str()) {
            report.push(
                format!("style_pack.forbidden_provider_tags.{tag}"),
                "incoherent_style_provider_compatibility",
                "A provider tag cannot be both allowed and forbidden.",
            );
        }
    }
    let mut seen_pairs = BTreeMap::<(&str, &str), bool>::new();
    let provider_pack_ids = draft
        .provider_taxonomy
        .provider_packs
        .iter()
        .map(|pack| pack.pack_id.as_str())
        .collect::<BTreeSet<_>>();
    let mut style_ids = draft
        .style_pack
        .compatibility_style_ids
        .iter()
        .map(String::as_str)
        .collect::<BTreeSet<_>>();
    style_ids.insert(draft.style_pack.style_id.as_str());
    for rule in &draft.compatibility_matrix.rules {
        let key = (rule.style_id.as_str(), rule.provider_pack_id.as_str());
        if !style_ids.contains(rule.style_id.as_str()) {
            report.push(
                format!("compatibility_matrix.rules.{}", rule.style_id),
                "compatibility_unknown_style",
                "Compatibility rules must reference the draft style or an explicitly listed review style.",
            );
        }
        if !provider_pack_ids.contains(rule.provider_pack_id.as_str()) {
            report.push(
                format!(
                    "compatibility_matrix.rules.{}.{}",
                    rule.style_id, rule.provider_pack_id
                ),
                "compatibility_unknown_provider_pack",
                "Compatibility rules must reference declared provider packs.",
            );
        }
        if let Some(previous) = seen_pairs.insert(key, rule.compatible)
            && previous != rule.compatible
        {
            report.push(
                format!(
                    "compatibility_matrix.rules.{}.{}",
                    rule.style_id, rule.provider_pack_id
                ),
                "incoherent_style_provider_compatibility",
                "A style/provider pair cannot be both compatible and incompatible.",
            );
        }
        if rule.reason.trim().is_empty() {
            report.push(
                format!(
                    "compatibility_matrix.rules.{}.{}",
                    rule.style_id, rule.provider_pack_id
                ),
                "missing_compatibility_reason",
                "Compatibility rules require a product-safe reason.",
            );
        }
    }
}

fn validate_controls(draft: &FoundryFoundationDraft, report: &mut FoundationDraftValidationReport) {
    let primary_count = draft
        .control_profile
        .controls
        .iter()
        .filter(|control| control.visible && control.primary)
        .count() as u32;
    if primary_count > DEFAULT_MAX_PRIMARY_NOVICE_CONTROLS
        || primary_count > draft.control_profile.maximum_primary_controls
    {
        report.push(
            "control_profile.controls",
            "too_many_primary_controls",
            "Foundation drafts may expose at most seven primary novice controls by default.",
        );
    }
    let mut family_owners = BTreeMap::<&str, &str>::new();
    let mut provider_owners = BTreeMap::<&str, &str>::new();
    let provider_slots = draft
        .provider_taxonomy
        .provider_slots
        .iter()
        .map(|slot| slot.slot_id.as_str())
        .collect::<BTreeSet<_>>();
    for control in draft
        .control_profile
        .controls
        .iter()
        .filter(|control| control.visible)
    {
        if contains_raw_authoring_marker(&control.label)
            || contains_raw_authoring_marker(&control.description)
        {
            report.push(
                format!("control_profile.controls.{}.label", control.control_id),
                "technical_term_in_novice_label",
                "Novice-facing controls must not expose technical authoring terms.",
            );
        }
        for slot in &control.owned_family_slots {
            if let Some(previous) = family_owners.insert(slot.as_str(), control.control_id.as_str())
            {
                report.push(
                    format!(
                        "control_profile.controls.{}.owned_family_slots",
                        control.control_id
                    ),
                    "duplicate_slot_ownership",
                    format!(
                        "Visible controls '{}' and '{}' both own family slot '{}'.",
                        previous, control.control_id, slot
                    ),
                );
            }
        }
        for slot in &control.owned_provider_slots {
            if !provider_slots.contains(slot.as_str()) {
                report.push(
                    format!(
                        "control_profile.controls.{}.owned_provider_slots",
                        control.control_id
                    ),
                    "control_unknown_provider_slot",
                    "Control-owned provider slots must reference declared provider slots.",
                );
            }
            if let Some(previous) =
                provider_owners.insert(slot.as_str(), control.control_id.as_str())
            {
                report.push(
                    format!(
                        "control_profile.controls.{}.owned_provider_slots",
                        control.control_id
                    ),
                    "duplicate_slot_ownership",
                    format!(
                        "Visible controls '{}' and '{}' both own provider slot '{}'.",
                        previous, control.control_id, slot
                    ),
                );
            }
        }
    }
}

fn validate_candidate_strategies(
    draft: &FoundryFoundationDraft,
    report: &mut FoundationDraftValidationReport,
) {
    if draft.candidate_strategy_pack.strategies.is_empty() {
        report.push(
            "candidate_strategy_pack.strategies",
            "empty_candidate_strategy",
            "Foundation drafts require at least one candidate strategy.",
        );
    }
    let visible_controls = draft
        .control_profile
        .controls
        .iter()
        .filter(|control| control.visible)
        .map(|control| control.control_id.as_str())
        .collect::<BTreeSet<_>>();
    let provider_slots = draft
        .provider_taxonomy
        .provider_slots
        .iter()
        .map(|slot| slot.slot_id.as_str())
        .collect::<BTreeSet<_>>();
    for strategy in &draft.candidate_strategy_pack.strategies {
        if strategy.allowed_controls.is_empty() {
            report.push(
                format!(
                    "candidate_strategy_pack.strategies.{}",
                    strategy.strategy_id
                ),
                "empty_candidate_strategy",
                "Candidate strategies must operate on visible controls.",
            );
        }
        for control_id in &strategy.allowed_controls {
            if !visible_controls.contains(control_id.as_str()) {
                report.push(
                    format!(
                        "candidate_strategy_pack.strategies.{}",
                        strategy.strategy_id
                    ),
                    "candidate_strategy_not_control_space",
                    "Candidate strategies must operate in visible control space.",
                );
            }
        }
        for slot_id in &strategy.allowed_provider_changes {
            if !provider_slots.contains(slot_id.as_str()) {
                report.push(
                    format!(
                        "candidate_strategy_pack.strategies.{}",
                        strategy.strategy_id
                    ),
                    "candidate_strategy_unknown_provider_slot",
                    "Candidate provider changes must reference declared provider slots.",
                );
            }
        }
    }
}

fn validate_quality_gate(
    draft: &FoundryFoundationDraft,
    report: &mut FoundationDraftValidationReport,
) {
    let Some(quality_gate) = &draft.quality_gate_profile else {
        report.push(
            "quality_gate_profile",
            "missing_quality_gate",
            "Foundation drafts require a quality gate profile.",
        );
        return;
    };
    if matches!(
        draft.quality_target,
        FoundationQualityTarget::Usable | FoundationQualityTarget::Showcase
    ) && !quality_gate.contact_sheet_required
    {
        report.push(
            "quality_gate_profile.contact_sheet_required",
            "usable_or_showcase_requires_contact_sheet",
            "Usable and Showcase targets require a contact sheet gate.",
        );
    }
    if !quality_gate.human_review_required {
        report.push(
            "quality_gate_profile.human_review_required",
            "quality_gate_requires_human_review",
            "Foundation draft quality gates must require human review.",
        );
    }
}

fn validate_forbidden_attempts(
    draft: &FoundryFoundationDraft,
    report: &mut FoundationDraftValidationReport,
) {
    for command in &draft.rejected_command_attempts {
        if forbidden_foundation_command_names()
            .iter()
            .any(|forbidden| forbidden == command)
        {
            report.push(
                "rejected_command_attempts",
                "forbidden_command_attempt",
                format!("Forbidden command '{command}' cannot be accepted."),
            );
        }
    }
    if !draft.direct_geometry_payload_attempts.is_empty() {
        report.push(
            "direct_geometry_payload_attempts",
            "direct_geometry_payload_attempt",
            "Foundation drafts must not include direct geometry payload attempts.",
        );
    }
}

/// Materialize a foundation draft into an internal-only kit package draft.
pub fn materialize_foundation_draft_package(
    draft: &FoundryFoundationDraft,
) -> Result<FoundryKitPackage, FoundationDraftValidationReport> {
    let report = validate_foundation_draft(draft);
    if !report.is_valid() {
        return Err(report);
    }
    let quality_gate = draft
        .quality_gate_profile
        .as_ref()
        .expect("validated draft has a quality gate");
    let family_id = draft.family_blueprint.family_id.clone();
    let kit_id = format!("{}-foundation-kit-draft", normalize_id(&family_id));
    let provider_pack_id = draft
        .provider_taxonomy
        .provider_packs
        .first()
        .map(|pack| pack.pack_id.clone())
        .unwrap_or_else(|| format!("{family_id}_providers"));
    let style_id = draft.style_pack.style_id.clone();
    let control_profile_id = draft.control_profile.profile_id.clone();
    let strategy_pack_id = draft.candidate_strategy_pack.pack_id.clone();
    let quality_profile_id = quality_gate.profile_id.clone();
    let matrix_id = draft.compatibility_matrix.matrix_id.clone();
    let review_id = format!("{}-review", draft.draft_id);
    let catalog_id = format!("{}-catalog", draft.draft_id);
    let category_chip = title_from_id(&draft.category);

    let package = FoundryKitPackage {
        schema_version: FOUNDRY_KIT_PACKAGE_SCHEMA_VERSION,
        kit: FoundryKit {
            schema_version: FOUNDRY_KIT_SCHEMA_VERSION,
            kit_id: kit_id.clone(),
            display_name: format!("{} Foundation Draft", draft.family_blueprint.display_name),
            family_blueprint_id: family_id.clone(),
            provider_pack_id: provider_pack_id.clone(),
            style_pack_id: style_id.clone(),
            control_profile_id: control_profile_id.clone(),
            candidate_strategy_pack_id: strategy_pack_id.clone(),
            quality_gate_profile_id: quality_profile_id.clone(),
            compatibility_matrix_id: matrix_id.clone(),
            review_manifest_id: review_id.clone(),
            catalog_manifest_id: catalog_id.clone(),
            preview_camera_policy: PreviewCameraPolicy {
                policy_id: format!("{}-preview", draft.draft_id),
                required_views: vec!["front".to_owned(), "three-quarter".to_owned()],
                clay_preview_required: true,
                contact_sheet_required: quality_gate.contact_sheet_required,
            },
            quality_tier: FoundryKitQualityTier::Draft,
            catalog_visibility_policy: CatalogVisibilityPolicy {
                default_novice_catalog: false,
                developer_preview_catalog: false,
                showcase_badge_allowed: false,
                hidden_reason: Some(
                    "Foundation drafts stay internal until human review approves authored geometry."
                        .to_owned(),
                ),
            },
            source_profile_slug: None,
            category_chips: vec![category_chip],
        },
        family_blueprint: FamilyBlueprint {
            schema_version: FAMILY_BLUEPRINT_SCHEMA_VERSION,
            family_id: family_id.clone(),
            display_name: draft.family_blueprint.display_name.clone(),
            semantic_roles: draft
                .family_blueprint
                .roles
                .iter()
                .map(|role| FamilyBlueprintRole {
                    role_id: role.role_id.clone(),
                    label: role.label.clone(),
                    required: role.required,
                    tags: role.tags.clone(),
                })
                .collect(),
            required_roles: draft.family_blueprint.required_roles.clone(),
            optional_roles: draft.family_blueprint.optional_roles.clone(),
            provider_slots: draft
                .provider_taxonomy
                .provider_slots
                .iter()
                .map(|slot| ProviderSlotExpectation {
                    slot_id: slot.slot_id.clone(),
                    role_id: slot.role_id.clone(),
                    required: slot.required,
                    attachment_tags: slot.compatibility_tags.clone(),
                })
                .collect(),
            attachment_expectations: draft
                .family_blueprint
                .sockets
                .iter()
                .map(|socket| AttachmentExpectation {
                    expectation_id: socket.socket_id.clone(),
                    from_role: socket.from_role.clone(),
                    to_role: socket.to_role.clone(),
                    compatibility_tags: socket.compatibility_tags.clone(),
                    required: socket.required,
                })
                .collect(),
            scale_policy: HighLevelScalePolicy {
                label: "Foundation draft scale".to_owned(),
                allowed_range: Some("Author must confirm production scale.".to_owned()),
            },
            export_part_naming_policy: ExportPartNamingPolicy {
                strategy: "role labels".to_owned(),
                required_part_names: draft.family_blueprint.export_part_names.clone(),
            },
        },
        provider_pack: ProviderPack {
            schema_version: PROVIDER_PACK_SCHEMA_VERSION,
            pack_id: provider_pack_id.clone(),
            family_id: Some(family_id.clone()),
            compatible_family_ids: vec![family_id.clone()],
            provider_slots_supplied: draft
                .provider_taxonomy
                .provider_slots
                .iter()
                .map(|slot| slot.slot_id.clone())
                .collect(),
            provider_options: draft
                .provider_taxonomy
                .provider_slots
                .iter()
                .map(|slot| ProviderPackOption {
                    option_id: format!("{}_foundation_option", slot.slot_id),
                    slot_id: slot.slot_id.clone(),
                    label: format!("{} Foundation Option", title_from_id(&slot.role_id)),
                    semantic_roles: vec![slot.role_id.clone()],
                    compatibility_tags: slot.compatibility_tags.clone(),
                    detail_density_tags: vec!["draft".to_owned()],
                    triangle_budget_estimate: None,
                })
                .collect(),
            semantic_role_coverage: draft
                .provider_taxonomy
                .provider_slots
                .iter()
                .map(|slot| slot.role_id.clone())
                .collect(),
            socket_attachment_tags: draft
                .family_blueprint
                .sockets
                .iter()
                .flat_map(|socket| socket.compatibility_tags.clone())
                .collect(),
            detail_density_tags: vec!["draft".to_owned()],
            triangle_budget_estimates: BTreeMap::new(),
            compatibility_tags: draft.style_pack.allowed_provider_tags.clone(),
        },
        style_pack: StylePack {
            schema_version: STYLE_PACK_SCHEMA_VERSION,
            style_id: style_id.clone(),
            display_name: draft.style_pack.display_name.clone(),
            compatible_family_ids: vec![family_id.clone()],
            bevel_language: draft.style_pack.bevel_language.clone(),
            proportion_language: draft.style_pack.proportion_language.clone(),
            detail_density_policy: draft.style_pack.detail_density_policy.clone(),
            silhouette_exaggeration_policy: draft.style_pack.silhouette_policy.clone(),
            symmetry_asymmetry_policy: draft.style_pack.symmetry_policy.clone(),
            allowed_provider_tags: draft.style_pack.allowed_provider_tags.clone(),
            forbidden_provider_tags: draft.style_pack.forbidden_provider_tags.clone(),
            compatible_provider_packs: vec![provider_pack_id.clone()],
            incompatible_provider_packs: Vec::new(),
            future_material_vocabulary: Some(FutureMaterialVocabulary {
                label: "Reserved metadata only".to_owned(),
                tags: Vec::new(),
            }),
        },
        control_profile: ControlProfile {
            schema_version: CONTROL_PROFILE_SCHEMA_VERSION,
            profile_id: control_profile_id,
            family_id: family_id.clone(),
            style_id: Some(style_id.clone()),
            maximum_primary_controls: draft.control_profile.maximum_primary_controls,
            controls: draft
                .control_profile
                .controls
                .iter()
                .map(|control| ControlProfileControl {
                    control_id: control.control_id.clone(),
                    label: control.label.clone(),
                    description: control.description.clone(),
                    kind: control.kind,
                    owned_family_slots: control.owned_family_slots.clone(),
                    owned_provider_slots: control.owned_provider_slots.clone(),
                    visible_effect_expectation: control.visible_effect_expectation.clone(),
                    topology_behavior: control.topology_behavior,
                    option_visibility: ControlOptionVisibility {
                        hide_invalid_from_novices: true,
                        show_plain_language_reasons: true,
                    },
                    default_locked: false,
                    primary: control.primary,
                    visible: control.visible,
                })
                .collect(),
        },
        candidate_strategy_pack: CandidateStrategyPack {
            schema_version: CANDIDATE_STRATEGY_PACK_SCHEMA_VERSION,
            pack_id: strategy_pack_id,
            strategies: draft
                .candidate_strategy_pack
                .strategies
                .iter()
                .map(|strategy| KitCandidateStrategy {
                    strategy_id: strategy.strategy_id.clone(),
                    name: strategy.name.clone(),
                    allowed_controls: strategy.allowed_controls.clone(),
                    explanation_templates: vec![strategy.explanation.clone()],
                })
                .collect(),
            allowed_controls: draft
                .candidate_strategy_pack
                .strategies
                .iter()
                .flat_map(|strategy| strategy.allowed_controls.clone())
                .collect(),
            allowed_provider_choices: allowed_provider_choices_for_draft(draft),
            diversity_goals: draft.candidate_strategy_pack.diversity_goals.clone(),
            invalid_state_rejection_policy: "Reject invalid foundation draft states.".to_owned(),
            lock_respect_policy: "Respect locked controls.".to_owned(),
        },
        quality_gate_profile: QualityGateProfile {
            schema_version: QUALITY_GATE_PROFILE_SCHEMA_VERSION,
            profile_id: quality_profile_id,
            required_tier: FoundryKitQualityTier::Draft,
            mesh_gates: vec!["Authored geometry required before promotion.".to_owned()],
            candidate_gates: vec!["Candidate strategies must stay in control space.".to_owned()],
            contact_sheet_gates: if quality_gate.contact_sheet_required {
                vec!["Contact sheet required before target-tier review.".to_owned()]
            } else {
                Vec::new()
            },
            export_gates: vec!["Package export must remain internal.".to_owned()],
            manual_review_gates: quality_gate.manual_review_gates.clone(),
        },
        compatibility_matrix: KitCompatibilityMatrix {
            schema_version: KIT_COMPATIBILITY_MATRIX_SCHEMA_VERSION,
            matrix_id,
            compatible_style_provider_pairs: draft
                .compatibility_matrix
                .rules
                .iter()
                .filter(|rule| rule.compatible)
                .map(|rule| StyleProviderCompatibility {
                    style_id: rule.style_id.clone(),
                    provider_pack_id: rule.provider_pack_id.clone(),
                    reason: rule.reason.clone(),
                })
                .collect(),
            incompatible_style_provider_pairs: draft
                .compatibility_matrix
                .rules
                .iter()
                .filter(|rule| !rule.compatible)
                .map(|rule| StyleProviderIncompatibility {
                    style_id: rule.style_id.clone(),
                    provider_pack_id: rule.provider_pack_id.clone(),
                    hidden_reason: rule.reason.clone(),
                })
                .collect(),
        },
        review_manifest: KitReviewManifest {
            schema_version: KIT_REVIEW_MANIFEST_SCHEMA_VERSION,
            manifest_id: review_id,
            tier_requested: draft.quality_target.into(),
            tier_achieved: FoundryKitQualityTier::Draft,
            reviewer: None,
            human_approval_marker: false,
            adversarial_review_marker: false,
            visual_review_notes: Vec::new(),
            contact_sheet_paths: Vec::new(),
            benchmark_refs: Vec::new(),
            known_limitations: vec![
                "Foundation draft only; production geometry and taste review are pending."
                    .to_owned(),
            ],
            blocked_reasons: vec![
                "Human review required before catalog exposure.".to_owned(),
                "Authored geometry ingredients are required before promotion.".to_owned(),
            ],
        },
        catalog_manifest: KitCatalogManifest {
            schema_version: KIT_CATALOG_MANIFEST_SCHEMA_VERSION,
            catalog_id,
            kit_ids: vec![kit_id.clone()],
            default_visible_kit_ids: Vec::new(),
            developer_preview_kit_ids: Vec::new(),
            hidden_kit_reasons: BTreeMap::from([(
                kit_id,
                "Foundation drafts stay internal.".to_owned(),
            )]),
        },
    };
    let kit_report = validate_foundry_kit_package(&package);
    if !kit_report.is_valid() {
        let mut report = FoundationDraftValidationReport::default();
        for issue in kit_report.issues {
            report.push(
                format!("materialized_package.{}", issue.subject),
                format!("materialized_{}", issue.code),
                issue.message,
            );
        }
        return Err(report);
    }
    Ok(package)
}

fn allowed_provider_choices_for_draft(
    draft: &FoundryFoundationDraft,
) -> BTreeMap<String, Vec<String>> {
    let provider_slots = draft
        .provider_taxonomy
        .provider_slots
        .iter()
        .map(|slot| slot.slot_id.as_str())
        .collect::<BTreeSet<_>>();
    let mut choices = BTreeMap::<String, BTreeSet<String>>::new();
    for strategy in &draft.candidate_strategy_pack.strategies {
        for slot_id in &strategy.allowed_provider_changes {
            if provider_slots.contains(slot_id.as_str()) {
                choices
                    .entry(slot_id.clone())
                    .or_default()
                    .insert(format!("{slot_id}_foundation_option"));
            }
        }
    }
    choices
        .into_iter()
        .map(|(slot, options)| (slot, options.into_iter().collect()))
        .collect()
}

/// Build a deterministic adversarial report for a draft.
#[must_use]
pub fn foundation_adversarial_report(draft: &FoundryFoundationDraft) -> DraftAdversarialReport {
    let primary_count = draft
        .control_profile
        .controls
        .iter()
        .filter(|control| control.visible && control.primary)
        .count();
    let provider_slot_count = draft.provider_taxonomy.provider_slots.len();
    let contact_sheet_required = draft
        .quality_gate_profile
        .as_ref()
        .is_some_and(|gate| gate.contact_sheet_required);
    let missing_geometry = vec![
        "Reviewed authored geometry or procedural art ingredients.".to_owned(),
        "Clay contact sheets once geometry exists.".to_owned(),
        "Human taste review notes.".to_owned(),
    ];
    let questions = vec![
        question(
            "over_abstracted",
            "Is this over-abstracted?",
            if draft.family_blueprint.roles.len() > 6 {
                "Role count is high; consolidate before authoring geometry."
            } else {
                "Role count is compact enough for a foundation draft."
            },
        ),
        question(
            "fewer_controls",
            "Does the kit need fewer controls?",
            if primary_count > 5 {
                "Primary controls are approaching novice complexity; reduce before promotion."
            } else {
                "Primary control count is restrained for a draft."
            },
        ),
        question(
            "provider_reuse",
            "Are provider slots reusable?",
            if provider_slot_count == 0 {
                "No provider slots are declared."
            } else {
                "Provider slots are explicit and can be reviewed for reuse."
            },
        ),
        question(
            "style_salad",
            "Could this become style salad?",
            if draft.style_pack.allowed_provider_tags.len() > 4 {
                "Allowed tags are broad; style compatibility needs pruning."
            } else {
                "Style tag surface is narrow enough for a draft."
            },
        ),
        question(
            "clear_labels",
            "Are noob-facing labels clear?",
            if validate_foundation_draft(draft)
                .issues
                .iter()
                .any(|issue| issue.code == "technical_term_in_novice_label")
            {
                "Some labels expose technical language."
            } else {
                "No technical label leakage detected."
            },
        ),
        question(
            "too_many_choices",
            "Are there too many choices for a novice?",
            if primary_count > DEFAULT_MAX_PRIMARY_NOVICE_CONTROLS as usize {
                "Primary choices exceed the novice control limit."
            } else {
                "Primary choices stay within the novice limit."
            },
        ),
        question(
            "mechanical_gates",
            "Are quality gates only mechanical?",
            if draft.review_checklist.items.is_empty() {
                "Review checklist is empty; add human taste gates."
            } else {
                "Review checklist includes human-review evidence."
            },
        ),
        question(
            "contact_sheets",
            "What visual contact sheets are required?",
            if contact_sheet_required {
                "Contact sheet gate is required for the target."
            } else {
                "Draft target does not require a contact sheet yet; require one before Usable."
            },
        ),
        question(
            "human_review",
            "What human review is required?",
            "Human review must approve geometry, controls, labels, quality evidence, and catalog visibility.",
        ),
        question(
            "missing_geometry",
            "What geometry/art ingredients are missing?",
            "Taste-bearing geometry, visual variants, and contact-sheet renders are not supplied by the foundation draft.",
        ),
        question(
            "procedural_filler",
            "What prevents this from becoming procedural filler?",
            "Internal-only visibility, validation gates, contact sheets, and human review block automatic promotion.",
        ),
    ];
    DraftAdversarialReport {
        schema_version: FOUNDRY_FOUNDATION_ADVERSARIAL_SCHEMA_VERSION,
        draft_id: draft.draft_id.clone(),
        questions,
        missing_geometry_art_ingredients: missing_geometry,
        human_review_required: vec![
            "Approve authored geometry.".to_owned(),
            "Approve contact sheets before Usable or Showcase claims.".to_owned(),
            "Approve catalog visibility explicitly.".to_owned(),
        ],
        summary: format!(
            "{} remains an internal foundation draft until authored geometry and review evidence pass.",
            draft.draft_id
        ),
    }
}

/// Suggest deterministic repairs from a validation report.
#[must_use]
pub fn suggest_foundation_repairs(
    draft: &FoundryFoundationDraft,
    validation_report_ref: impl Into<String>,
    report: &FoundationDraftValidationReport,
) -> DraftRepairSuggestion {
    DraftRepairSuggestion {
        schema_version: FOUNDRY_FOUNDATION_DRAFT_SCHEMA_VERSION,
        draft_id: draft.draft_id.clone(),
        validation_report_ref: validation_report_ref.into(),
        suggestions: report
            .issues
            .iter()
            .map(|issue| repair_for_issue(&issue.code))
            .collect::<BTreeSet<_>>()
            .into_iter()
            .map(str::to_owned)
            .collect(),
    }
}

fn repair_for_issue(code: &str) -> &'static str {
    match code {
        "missing_required_roles" => "Add at least one required family role.",
        "too_many_primary_controls" => "Reduce visible primary controls to seven or fewer.",
        "technical_term_in_novice_label" => "Replace technical labels with product-facing words.",
        "duplicate_slot_ownership" => "Assign each visible slot to one control.",
        "missing_provider_slots" | "missing_provider_slot_for_required_role" => {
            "Add provider slots for every required role."
        }
        "incoherent_style_provider_compatibility" => {
            "Remove contradictory style/provider compatibility rules."
        }
        "empty_candidate_strategy" | "candidate_strategy_not_control_space" => {
            "Make candidate strategies operate on visible controls."
        }
        "missing_quality_gate" => "Add a quality gate profile.",
        "usable_or_showcase_requires_contact_sheet" => {
            "Require contact sheets for Usable or Showcase targets."
        }
        "publish_requires_human_review" | "draft_or_prototype_cannot_be_novice_visible" => {
            "Keep the draft internal until explicit human review approves promotion."
        }
        "forbidden_command_attempt" | "direct_geometry_payload_attempt" => {
            "Remove forbidden commands and direct geometry payloads."
        }
        _ => "Review the validation issue and update the structured draft.",
    }
}

fn question(
    question_id: impl Into<String>,
    question: impl Into<String>,
    finding: impl Into<String>,
) -> DraftAdversarialQuestion {
    DraftAdversarialQuestion {
        question_id: question_id.into(),
        question: question.into(),
        finding: finding.into(),
    }
}

fn contains_raw_authoring_marker(value: &str) -> bool {
    let lower = value.to_ascii_lowercase();
    [
        "::",
        "scalar",
        "recipe",
        "semantic id",
        "semantic_id",
        "operation id",
        "operation_id",
        "provider id",
        "provider_id",
        "compiler",
        "decompiler",
        "raw vertex",
    ]
    .iter()
    .any(|marker| lower.contains(marker))
}

fn primary_role_for_family(family_id: &str) -> String {
    let _ = family_id;
    "body".to_owned()
}

fn secondary_role_for_family(family_id: &str) -> String {
    let _ = family_id;
    "detail".to_owned()
}

fn normalize_id(value: &str) -> String {
    value
        .trim()
        .to_ascii_lowercase()
        .chars()
        .map(|ch| if ch.is_ascii_alphanumeric() { ch } else { '_' })
        .collect::<String>()
        .split('_')
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>()
        .join("_")
}

fn title_from_id(value: &str) -> String {
    value
        .split(['_', '-'])
        .filter(|part| !part.is_empty())
        .map(|part| {
            let mut chars = part.chars();
            match chars.next() {
                Some(first) => format!("{}{}", first.to_ascii_uppercase(), chars.as_str()),
                None => String::new(),
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{foundry_kit_visibility_decision, validate_foundry_kit_package};

    #[test]
    fn foundation_draft_schema_roundtrips() {
        let draft = foundation_draft_template("boxes", "box_primitive");
        let json = serde_json::to_string(&draft).expect("serialize");
        let roundtrip: FoundryFoundationDraft = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(roundtrip, draft);
        assert_eq!(
            roundtrip.schema_version,
            FOUNDRY_FOUNDATION_DRAFT_SCHEMA_VERSION
        );
    }

    #[test]
    fn new_draft_defaults_to_internal_reviewed_unpublished() {
        let draft = foundation_draft_template("boxes", "box_primitive");
        assert_eq!(
            draft.catalog_visibility,
            FoundationCatalogVisibility::InternalOnly
        );
        assert!(draft.human_review_required);
        assert!(!draft.publish_allowed);
    }

    #[test]
    fn forbidden_commands_cannot_deserialize_or_execute() {
        for command in forbidden_foundation_command_names() {
            let json = format!(r#"{{"command":"{command}","args":{{}}}}"#);
            assert!(parse_foundation_authoring_command_json(&json).is_err());
        }
        let extra_args = r#"{
            "command": "CreateFamilyBlueprint",
            "args": {
                "family_id": "box_primitive",
                "display_name": "Box Primitive",
                "raw_vertex_positions": [[0, 0, 0]]
            }
        }"#;
        assert!(parse_foundation_authoring_command_json(extra_args).is_err());
        let mut draft = foundation_draft_template("boxes", "box_primitive");
        draft
            .rejected_command_attempts
            .push("InjectMeshPayload".to_owned());
        draft
            .direct_geometry_payload_attempts
            .push("vertices:[0,0,0]".to_owned());
        let report = validate_foundation_draft(&draft);
        let codes = issue_codes(&report);
        assert!(codes.contains("forbidden_command_attempt"));
        assert!(codes.contains("direct_geometry_payload_attempt"));
    }

    #[test]
    fn foundation_draft_rejects_unknown_geometry_fields() {
        let draft = foundation_draft_template("boxes", "box_primitive");
        let mut value = serde_json::to_value(&draft).expect("to value");
        let object = value.as_object_mut().expect("object");
        object.insert(
            "mesh_payload".to_owned(),
            serde_json::json!({"vertices": [[0, 0, 0]]}),
        );
        object.insert(
            "raw_vertex_positions".to_owned(),
            serde_json::json!([[0, 0, 0]]),
        );
        assert!(serde_json::from_value::<FoundryFoundationDraft>(value).is_err());
    }

    #[test]
    fn allowed_commands_execute_in_structured_space() {
        let mut draft = foundation_draft_template("boxes", "box_primitive");
        execute_foundation_authoring_command(
            &mut draft,
            FoundationAuthoringCommand::AddRole {
                role_id: "edge_detail".to_owned(),
                label: "Edge Detail".to_owned(),
                required: false,
            },
        )
        .expect("execute");
        assert!(
            draft
                .family_blueprint
                .roles
                .iter()
                .any(|role| role.role_id == "edge_detail")
        );
    }

    #[test]
    fn draft_validation_rejects_primary_control_overload_and_technical_labels() {
        let mut draft = foundation_draft_template("boxes", "box_primitive");
        draft.control_profile.controls = (0..8)
            .map(|index| DraftControl {
                control_id: format!("control_{index}"),
                label: if index == 0 {
                    "scalar::body.path".to_owned()
                } else {
                    format!("Control {index}")
                },
                description: "Visible change.".to_owned(),
                kind: ControlProfileControlKind::Choice,
                primary: true,
                visible: true,
                owned_family_slots: Vec::new(),
                owned_provider_slots: vec![format!("slot_{index}")],
                topology_behavior: ControlProfileTopologyBehavior::TopologyChanging,
                visible_effect_expectation: "Visible change.".to_owned(),
            })
            .collect();
        let report = validate_foundation_draft(&draft);
        let codes = issue_codes(&report);
        assert!(codes.contains("too_many_primary_controls"));
        assert!(codes.contains("technical_term_in_novice_label"));
    }

    #[test]
    fn draft_validation_rejects_missing_quality_gate_and_contact_sheet_gap() {
        let mut draft = foundation_draft_template("boxes", "box_primitive");
        draft.quality_gate_profile = None;
        assert!(issue_codes(&validate_foundation_draft(&draft)).contains("missing_quality_gate"));

        let mut usable = foundation_draft_template("boxes", "box_primitive");
        usable.quality_target = FoundationQualityTarget::Usable;
        usable
            .quality_gate_profile
            .as_mut()
            .expect("quality gate")
            .contact_sheet_required = false;
        assert!(
            issue_codes(&validate_foundation_draft(&usable))
                .contains("usable_or_showcase_requires_contact_sheet")
        );
    }

    #[test]
    fn draft_validation_rejects_slot_and_candidate_incoherence() {
        let mut draft = foundation_draft_template("boxes", "box_primitive");
        draft.control_profile.controls.push(DraftControl {
            control_id: "conflict".to_owned(),
            label: "Conflict".to_owned(),
            description: "Conflicting control.".to_owned(),
            kind: ControlProfileControlKind::Choice,
            primary: true,
            visible: true,
            owned_family_slots: Vec::new(),
            owned_provider_slots: draft.control_profile.controls[0]
                .owned_provider_slots
                .clone(),
            topology_behavior: ControlProfileTopologyBehavior::TopologyChanging,
            visible_effect_expectation: "Visible change.".to_owned(),
        });
        draft.candidate_strategy_pack.strategies[0]
            .allowed_controls
            .push("missing_control".to_owned());
        draft.candidate_strategy_pack.strategies[0]
            .allowed_provider_changes
            .push("missing_slot".to_owned());
        let report = validate_foundation_draft(&draft);
        let codes = issue_codes(&report);
        assert!(codes.contains("duplicate_slot_ownership"));
        assert!(codes.contains("candidate_strategy_not_control_space"));
        assert!(codes.contains("candidate_strategy_unknown_provider_slot"));
    }

    #[test]
    fn draft_validation_rejects_unknown_control_provider_slots() {
        let mut draft = foundation_draft_template("boxes", "box_primitive");
        draft.control_profile.controls[0]
            .owned_provider_slots
            .push("missing_slot".to_owned());
        let report = validate_foundation_draft(&draft);
        assert!(issue_codes(&report).contains("control_unknown_provider_slot"));
    }

    #[test]
    fn draft_validation_rejects_unknown_compatibility_references() {
        let mut draft = foundation_draft_template("boxes", "box_primitive");
        draft
            .compatibility_matrix
            .rules
            .push(DraftCompatibilityRule {
                style_id: "unknown_style".to_owned(),
                provider_pack_id: "unknown_provider_pack".to_owned(),
                compatible: true,
                reason: "Invalid row used for validation coverage.".to_owned(),
            });
        let report = validate_foundation_draft(&draft);
        let codes = issue_codes(&report);
        assert!(codes.contains("compatibility_unknown_style"));
        assert!(codes.contains("compatibility_unknown_provider_pack"));

        let mut explicit_review_style = foundation_draft_template("boxes", "box_primitive");
        explicit_review_style
            .style_pack
            .compatibility_style_ids
            .push("review_style".to_owned());
        explicit_review_style
            .compatibility_matrix
            .rules
            .push(DraftCompatibilityRule {
                style_id: "review_style".to_owned(),
                provider_pack_id: explicit_review_style.provider_taxonomy.provider_packs[0]
                    .pack_id
                    .clone(),
                compatible: true,
                reason: "Explicitly listed review style is allowed.".to_owned(),
            });
        assert!(validate_foundation_draft(&explicit_review_style).is_valid());
    }

    #[test]
    fn draft_validation_rejects_publish_and_visibility_overclaims() {
        let mut draft = foundation_draft_template("boxes", "box_primitive");
        draft.human_review_required = false;
        draft.publish_allowed = true;
        draft.catalog_visibility = FoundationCatalogVisibility::NoviceCatalog;
        let report = validate_foundation_draft(&draft);
        let codes = issue_codes(&report);
        assert!(codes.contains("foundation_publish_not_allowed"));
        assert!(codes.contains("foundation_draft_must_remain_internal"));
        assert!(codes.contains("publish_requires_human_review"));
        assert!(codes.contains("draft_or_prototype_cannot_be_novice_visible"));
    }

    #[test]
    fn usable_or_showcase_drafts_still_cannot_publish_or_leave_internal_catalog() {
        let mut draft = foundation_draft_template("boxes", "box_primitive");
        draft.quality_target = FoundationQualityTarget::Usable;
        draft.publish_allowed = true;
        draft.catalog_visibility = FoundationCatalogVisibility::NoviceCatalog;
        draft
            .quality_gate_profile
            .as_mut()
            .expect("quality gate")
            .contact_sheet_required = true;
        let report = validate_foundation_draft(&draft);
        let codes = issue_codes(&report);
        assert!(codes.contains("foundation_publish_not_allowed"));
        assert!(codes.contains("foundation_draft_must_remain_internal"));
    }

    #[test]
    fn materialized_kit_remains_draft_until_reviewed() {
        let draft = foundation_draft_template("boxes", "box_primitive");
        let package = materialize_foundation_draft_package(&draft).expect("materialize");
        assert_eq!(package.kit.quality_tier, FoundryKitQualityTier::Draft);
        assert!(!package.kit.catalog_visibility_policy.default_novice_catalog);
        assert!(
            !package
                .kit
                .catalog_visibility_policy
                .developer_preview_catalog
        );
        assert!(package.catalog_manifest.default_visible_kit_ids.is_empty());
        assert!(
            package
                .catalog_manifest
                .developer_preview_kit_ids
                .is_empty()
        );
        assert!(validate_foundry_kit_package(&package).is_valid());
        assert!(
            package
                .candidate_strategy_pack
                .allowed_provider_choices
                .contains_key("body_slot")
        );
        let decision =
            foundry_kit_visibility_decision(&package.kit, &package.review_manifest, false);
        assert!(!decision.visible);
    }

    #[test]
    fn materialization_maps_invalid_package_report_back_to_foundation_errors() {
        let mut draft = foundation_draft_template("boxes", "box_primitive");
        draft.compatibility_matrix.rules = vec![DraftCompatibilityRule {
            style_id: draft.style_pack.style_id.clone(),
            provider_pack_id: draft.provider_taxonomy.provider_packs[0].pack_id.clone(),
            compatible: false,
            reason: "Conflicts with this provider pack.".to_owned(),
        }];
        assert!(validate_foundation_draft(&draft).is_valid());
        let report = materialize_foundation_draft_package(&draft)
            .expect_err("materialization should reject invalid kit package");
        assert!(issue_codes(&report).contains("materialized_style_provider_pair_incompatible"));
    }

    #[test]
    fn archetype_materializer_box_primitive_draft_is_internal_only() {
        for (family_id, style_id) in [
            ("box-primitive", "plain-clay"),
            ("wide-box", "plain-clay"),
            ("soft-box", "plain-clay"),
        ] {
            let draft =
                materialize_archetype_foundation_draft("box-primitive", family_id, style_id)
                    .expect("box primitive archetype materializes");
            assert!(validate_foundation_draft(&draft).is_valid());
            assert!(!draft.publish_allowed);
            assert_eq!(
                draft.catalog_visibility,
                FoundationCatalogVisibility::InternalOnly
            );
            assert!(draft.human_review_required);
            assert!(draft.direct_geometry_payload_attempts.is_empty());
            assert_eq!(draft.control_profile.controls.len(), 2);
            assert_eq!(draft.family_blueprint.required_roles, vec!["body"]);
            assert!(draft.family_blueprint.optional_roles.is_empty());
            assert!(draft.family_blueprint.sockets.is_empty());
        }
        let draft =
            materialize_archetype_foundation_draft("box-primitive", "box-primitive", "plain-clay")
                .expect("box primitive archetype materializes");
        assert!(validate_foundation_draft(&draft).is_valid());
        assert_eq!(draft.family_blueprint.family_id, "box_primitive");
        assert_eq!(draft.style_pack.style_id, "plain_clay");
    }

    #[test]
    fn archetype_materializer_rejects_invalid_archetype_and_geometry_payloads() {
        assert!(
            materialize_archetype_foundation_draft(
                "unsupported-archetype",
                "box-primitive",
                "plain-clay",
            )
            .is_err()
        );
        let mut draft =
            materialize_archetype_foundation_draft("box-primitive", "box-primitive", "plain-clay")
                .expect("box primitive archetype materializes");
        draft
            .direct_geometry_payload_attempts
            .push("raw vertex payload".to_owned());
        let report = validate_foundation_draft(&draft);
        assert!(issue_codes(&report).contains("direct_geometry_payload_attempt"));
    }

    #[test]
    fn archetype_materializer_report_is_deterministic() {
        let first =
            materialize_archetype_foundation_draft("box-primitive", "box-primitive", "plain-clay")
                .expect("first draft");
        let second =
            materialize_archetype_foundation_draft("box-primitive", "box-primitive", "plain-clay")
                .expect("second draft");
        assert_eq!(first, second);
        let files = vec![
            "family-blueprint-draft.json".to_owned(),
            "materialization-report.json".to_owned(),
        ];
        let first_report = archetype_draft_materialization_report(&first, files.clone());
        let second_report = archetype_draft_materialization_report(&second, files);
        assert_eq!(first_report, second_report);
        assert!(!first_report.publish_allowed);
        assert!(!first_report.novice_visible);
        assert!(first_report.human_review_required);
        assert!(!first_report.missing_taste_bearing_providers.is_empty());
    }

    #[test]
    fn fixture_drafts_are_internal_draft_only() {
        let fixtures = foundation_draft_fixtures();
        assert_eq!(fixtures.len(), 1);
        for draft in fixtures {
            assert_eq!(draft.quality_target, FoundationQualityTarget::Draft);
            assert_eq!(
                draft.catalog_visibility,
                FoundationCatalogVisibility::InternalOnly
            );
            assert!(!draft.publish_allowed);
            assert!(validate_foundation_draft(&draft).is_valid());
        }
    }

    #[test]
    fn adversarial_report_is_deterministic_and_complete() {
        let draft = foundation_draft_template("boxes", "box_primitive");
        let first = foundation_adversarial_report(&draft);
        let second = foundation_adversarial_report(&draft);
        assert_eq!(first, second);
        assert_eq!(first.questions.len(), 11);
        assert!(
            first
                .questions
                .iter()
                .any(|question| question.question.contains("procedural filler"))
        );
    }

    fn issue_codes(report: &FoundationDraftValidationReport) -> BTreeSet<&str> {
        report
            .issues
            .iter()
            .map(|issue| issue.code.as_str())
            .collect()
    }
}
