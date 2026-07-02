//! Internal Foundry archetype authoring contracts.
//!
//! Archetypes are pro/internal grammar contracts. They are not product profiles,
//! do not generate geometry by themselves, and must not be published directly
//! to the novice Visual Foundry catalog.

use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};

/// Current schema version for Foundry archetype contracts.
pub const FOUNDRY_ARCHETYPE_SCHEMA_VERSION: u32 = 1;
/// The only archetype supported by v0.
pub const BOX_PRIMITIVE_ARCHETYPE_ID: &str = "box-primitive";

/// Internal/pro authoring contract for one reusable asset grammar.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FoundryArchetype {
    /// Stable archetype ID.
    pub archetype_id: String,
    /// Author-facing name.
    pub display_name: String,
    /// Author-facing description.
    pub description: String,
    /// Required role templates.
    pub role_templates: Vec<ArchetypeRoleTemplate>,
    /// Optional role templates.
    pub optional_role_templates: Vec<ArchetypeRoleTemplate>,
    /// Provider slot templates compatible with roles.
    pub provider_slot_templates: Vec<ArchetypeProviderSlotTemplate>,
    /// Primary control axis templates.
    pub control_axis_templates: Vec<ArchetypeControlAxisTemplate>,
    /// Candidate strategy templates.
    pub candidate_strategy_templates: Vec<ArchetypeCandidateStrategyTemplate>,
    /// Quality gate templates.
    pub quality_gate_templates: Vec<ArchetypeQualityGateTemplate>,
    /// Normalized style tags that may build on this archetype.
    pub compatible_style_tags: Vec<String>,
    /// Archetype contract version.
    pub version: u32,
    /// Archetypes are internal contracts and must not be directly published.
    pub publish_allowed: bool,
    /// Archetypes must not appear in novice Visual Foundry.
    pub novice_visible: bool,
    /// Validation rejects any embedded geometry payload.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub geometry_payload: Option<String>,
    /// Validation rejects any raw vertex payload.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub raw_vertex_payload: Option<String>,
}

/// One archetype role template.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ArchetypeRoleTemplate {
    /// Stable role ID.
    pub role_id: String,
    /// Product-safe role label.
    pub display_name: String,
    /// Whether the role is required.
    pub required: bool,
    /// Repeat policy for this role.
    pub repeat_policy: ArchetypeRepeatPolicy,
    /// Provider slot IDs allowed to satisfy this role.
    pub allowed_provider_slot_ids: Vec<String>,
    /// Semantic part group used by preview/review tooling.
    pub semantic_part_group: String,
}

/// Role repeat policy.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ArchetypeRepeatPolicy {
    /// Exactly one role instance.
    Once,
    /// Zero or one role instance.
    ZeroOrOne,
    /// Zero or more role instances.
    ZeroOrMore,
    /// One or more role instances.
    OneOrMore,
}

/// Provider slot template.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ArchetypeProviderSlotTemplate {
    /// Stable slot ID.
    pub slot_id: String,
    /// Author-facing label.
    pub display_name: String,
    /// Role this slot targets.
    pub target_role_id: String,
    /// Whether this slot is required.
    pub required: bool,
    /// Style tags compatible with this slot.
    pub compatible_style_tags: Vec<String>,
    /// Whether a style pack may bias this slot.
    pub may_be_style_biased: bool,
}

/// Primary control axis template.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ArchetypeControlAxisTemplate {
    /// Stable control ID.
    pub control_id: String,
    /// Product-facing label.
    pub display_name: String,
    /// Roles owned by this control.
    pub owns_role_ids: Vec<String>,
    /// Provider slots owned by this control.
    pub owns_provider_slot_ids: Vec<String>,
    /// Topology behavior expected for this control.
    pub topology_behavior: ArchetypeTopologyBehavior,
    /// Expected visibility level in clay preview.
    pub expected_visibility_level: ArchetypeVisibilityLevel,
}

/// Control topology behavior.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ArchetypeTopologyBehavior {
    /// Continuous control path must preserve topology.
    ContinuousPreservesTopology,
    /// Discrete/gallery control path may change topology.
    DiscreteTopologyChanging,
}

/// Expected clay-readability level for a control.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ArchetypeVisibilityLevel {
    /// Visible in the primary silhouette.
    MajorSilhouette,
    /// Visible as secondary structure.
    StructuralRead,
    /// Visible as detail language.
    DetailRead,
}

/// Candidate strategy template.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ArchetypeCandidateStrategyTemplate {
    /// Stable strategy ID.
    pub strategy_id: String,
    /// Product-facing strategy label.
    pub display_name: String,
    /// Controls this strategy is intended to change.
    pub intended_changed_controls: Vec<String>,
    /// Roles this strategy is intended to change.
    pub intended_changed_roles: Vec<String>,
    /// Strategy mode.
    pub mode: ArchetypeCandidateStrategyMode,
}

/// Candidate strategy mode.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ArchetypeCandidateStrategyMode {
    /// Whole-asset variation strategy.
    WholeAssetVariation,
    /// Control endpoint proof strategy.
    ControlEndpointProof,
}

/// Quality gate template.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ArchetypeQualityGateTemplate {
    /// Stable gate ID.
    pub gate_id: String,
    /// Human-facing gate label.
    pub display_name: String,
    /// Whether this gate is required.
    pub required: bool,
    /// Evidence kind required by this gate.
    pub evidence_kind: ArchetypeEvidenceKind,
}

/// Quality evidence kind.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ArchetypeEvidenceKind {
    /// Pure clay preview evidence.
    PureClayPreview,
    /// Semantic clay preview evidence.
    SemanticClayPreview,
    /// Control endpoint contact sheet.
    ControlEndpointSheet,
    /// Candidate survivor contact sheet.
    CandidateContactSheet,
    /// Floating part / attachment validation.
    AttachmentValidation,
    /// Export contract validation.
    ExportValidation,
}

/// One archetype validation issue.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FoundryArchetypeValidationIssue {
    /// Stable subject path.
    pub subject: String,
    /// Stable issue code.
    pub code: String,
    /// Human-readable message.
    pub message: String,
}

/// Archetype validation report.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct FoundryArchetypeValidationReport {
    /// Issues discovered during validation.
    pub issues: Vec<FoundryArchetypeValidationIssue>,
}

impl FoundryArchetypeValidationReport {
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
        self.issues.push(FoundryArchetypeValidationIssue {
            subject: subject.into(),
            code: code.into(),
            message: message.into(),
        });
    }
}

/// Return the only v0 archetype contract.
#[must_use]
pub fn box_primitive_archetype() -> FoundryArchetype {
    let role_templates = [role("body", "Body", true, "primary_mass")]
        .into_iter()
        .map(with_default_slot)
        .collect::<Vec<_>>();

    FoundryArchetype {
        archetype_id: BOX_PRIMITIVE_ARCHETYPE_ID.to_owned(),
        display_name: "Box Primitive".to_owned(),
        description:
            "Internal contract for a closed box-like volume with readable proportions and edge softness."
                .to_owned(),
        role_templates,
        optional_role_templates: Vec::new(),
        provider_slot_templates: box_primitive_provider_slots(),
        control_axis_templates: box_primitive_controls(),
        candidate_strategy_templates: box_primitive_candidate_strategies(),
        quality_gate_templates: box_primitive_quality_gates(),
        compatible_style_tags: vec!["box-primitive".to_owned(), "plain-clay".to_owned()],
        version: FOUNDRY_ARCHETYPE_SCHEMA_VERSION,
        publish_allowed: false,
        novice_visible: false,
        geometry_payload: None,
        raw_vertex_payload: None,
    }
}

fn role(
    role_id: &str,
    display_name: &str,
    required: bool,
    semantic_part_group: &str,
) -> ArchetypeRoleTemplate {
    ArchetypeRoleTemplate {
        role_id: role_id.to_owned(),
        display_name: display_name.to_owned(),
        required,
        repeat_policy: if required {
            ArchetypeRepeatPolicy::Once
        } else {
            ArchetypeRepeatPolicy::ZeroOrMore
        },
        allowed_provider_slot_ids: Vec::new(),
        semantic_part_group: semantic_part_group.to_owned(),
    }
}

fn with_default_slot(mut role: ArchetypeRoleTemplate) -> ArchetypeRoleTemplate {
    role.allowed_provider_slot_ids = vec![slot_id_for_role(&role.role_id)];
    role
}

fn slot_id_for_role(role_id: &str) -> String {
    format!("{role_id}_slot")
}

fn box_primitive_provider_slots() -> Vec<ArchetypeProviderSlotTemplate> {
    [("body", "Body Choices", true, false)]
        .into_iter()
        .map(|(role_id, display_name, required, may_be_style_biased)| {
            ArchetypeProviderSlotTemplate {
                slot_id: slot_id_for_role(role_id),
                display_name: display_name.to_owned(),
                target_role_id: role_id.to_owned(),
                required,
                compatible_style_tags: vec!["box-primitive".to_owned(), "plain-clay".to_owned()],
                may_be_style_biased,
            }
        })
        .collect()
}

fn box_primitive_controls() -> Vec<ArchetypeControlAxisTemplate> {
    vec![
        control(
            "proportions",
            "Proportions",
            &["body"],
            &["body"],
            ArchetypeTopologyBehavior::ContinuousPreservesTopology,
            ArchetypeVisibilityLevel::MajorSilhouette,
        ),
        control(
            "edge_softness",
            "Edge Softness",
            &["body"],
            &["body"],
            ArchetypeTopologyBehavior::ContinuousPreservesTopology,
            ArchetypeVisibilityLevel::StructuralRead,
        ),
    ]
}

fn control(
    control_id: &str,
    display_name: &str,
    owns_role_ids: &[&str],
    owns_provider_role_ids: &[&str],
    topology_behavior: ArchetypeTopologyBehavior,
    expected_visibility_level: ArchetypeVisibilityLevel,
) -> ArchetypeControlAxisTemplate {
    ArchetypeControlAxisTemplate {
        control_id: control_id.to_owned(),
        display_name: display_name.to_owned(),
        owns_role_ids: owns_role_ids
            .iter()
            .map(|value| (*value).to_owned())
            .collect(),
        owns_provider_slot_ids: owns_provider_role_ids
            .iter()
            .map(|value| slot_id_for_role(value))
            .collect(),
        topology_behavior,
        expected_visibility_level,
    }
}

fn box_primitive_candidate_strategies() -> Vec<ArchetypeCandidateStrategyTemplate> {
    vec![
        strategy("compact_box", "Compact Box", &["proportions"], &["body"]),
        strategy("wide_box", "Wide Box", &["proportions"], &["body"]),
        strategy("tall_box", "Tall Box", &["proportions"], &["body"]),
        strategy("flat_box", "Flat Box", &["proportions"], &["body"]),
        strategy(
            "soft_edged_box",
            "Soft-Edged Box",
            &["edge_softness"],
            &["body"],
        ),
        strategy(
            "sharp_utility_box",
            "Sharp Box",
            &["edge_softness"],
            &["body"],
        ),
    ]
}

fn strategy(
    strategy_id: &str,
    display_name: &str,
    intended_changed_controls: &[&str],
    intended_changed_roles: &[&str],
) -> ArchetypeCandidateStrategyTemplate {
    ArchetypeCandidateStrategyTemplate {
        strategy_id: strategy_id.to_owned(),
        display_name: display_name.to_owned(),
        intended_changed_controls: intended_changed_controls
            .iter()
            .map(|value| (*value).to_owned())
            .collect(),
        intended_changed_roles: intended_changed_roles
            .iter()
            .map(|value| (*value).to_owned())
            .collect(),
        mode: ArchetypeCandidateStrategyMode::WholeAssetVariation,
    }
}

fn box_primitive_quality_gates() -> Vec<ArchetypeQualityGateTemplate> {
    vec![
        gate(
            "pure_clay_readability",
            "Pure Clay Readability",
            ArchetypeEvidenceKind::PureClayPreview,
        ),
        gate(
            "semantic_clay_readability",
            "Semantic Clay Readability",
            ArchetypeEvidenceKind::SemanticClayPreview,
        ),
        gate(
            "visible_control_endpoints",
            "Visible Control Endpoints",
            ArchetypeEvidenceKind::ControlEndpointSheet,
        ),
        gate(
            "visible_candidate_survivors",
            "Visible Candidate Survivors",
            ArchetypeEvidenceKind::CandidateContactSheet,
        ),
        gate(
            "no_floating_parts",
            "No Floating Parts",
            ArchetypeEvidenceKind::AttachmentValidation,
        ),
        gate(
            "export_clean",
            "Export Clean",
            ArchetypeEvidenceKind::ExportValidation,
        ),
    ]
}

fn gate(
    gate_id: &str,
    display_name: &str,
    evidence_kind: ArchetypeEvidenceKind,
) -> ArchetypeQualityGateTemplate {
    ArchetypeQualityGateTemplate {
        gate_id: gate_id.to_owned(),
        display_name: display_name.to_owned(),
        required: true,
        evidence_kind,
    }
}

/// Validate a Foundry archetype contract.
#[must_use]
pub fn validate_foundry_archetype(
    archetype: &FoundryArchetype,
) -> FoundryArchetypeValidationReport {
    let mut report = FoundryArchetypeValidationReport::default();

    validate_identifier(
        &mut report,
        "archetype_id",
        &archetype.archetype_id,
        "invalid_archetype_id",
    );
    validate_non_empty_label(&mut report, "display_name", &archetype.display_name);
    validate_non_empty_label(&mut report, "description", &archetype.description);
    if archetype.version != FOUNDRY_ARCHETYPE_SCHEMA_VERSION {
        report.push(
            "version",
            "unsupported_archetype_version",
            "Archetype version is not supported.",
        );
    }
    if archetype.publish_allowed {
        report.push(
            "publish_allowed",
            "archetype_publish_forbidden",
            "Archetypes are authoring contracts and cannot publish directly.",
        );
    }
    if archetype.novice_visible {
        report.push(
            "novice_visible",
            "archetype_novice_visibility_forbidden",
            "Archetype internals must not appear in novice Visual Foundry.",
        );
    }
    if archetype.geometry_payload.is_some() {
        report.push(
            "geometry_payload",
            "geometry_payload_rejected",
            "Archetypes cannot contain geometry payloads.",
        );
    }
    if archetype.raw_vertex_payload.is_some() {
        report.push(
            "raw_vertex_payload",
            "raw_vertex_payload_rejected",
            "Archetypes cannot contain raw vertex payloads.",
        );
    }

    let mut required_role_ids: BTreeSet<String> = BTreeSet::new();
    let mut all_role_ids: BTreeSet<String> = BTreeSet::new();
    for (index, role) in archetype.role_templates.iter().enumerate() {
        validate_role(
            &mut report,
            format!("role_templates.{index}"),
            role,
            true,
            &mut required_role_ids,
            &mut all_role_ids,
        );
    }
    for (index, role) in archetype.optional_role_templates.iter().enumerate() {
        validate_role(
            &mut report,
            format!("optional_role_templates.{index}"),
            role,
            false,
            &mut BTreeSet::new(),
            &mut all_role_ids,
        );
    }
    if archetype.role_templates.is_empty() {
        report.push(
            "role_templates",
            "missing_required_roles",
            "Archetype must define required roles.",
        );
    }

    let mut provider_slot_ids: BTreeSet<String> = BTreeSet::new();
    for (index, slot) in archetype.provider_slot_templates.iter().enumerate() {
        validate_provider_slot(
            &mut report,
            format!("provider_slot_templates.{index}"),
            slot,
            &all_role_ids,
            &mut provider_slot_ids,
        );
    }

    let mut control_ids: BTreeSet<String> = BTreeSet::new();
    for (index, control) in archetype.control_axis_templates.iter().enumerate() {
        validate_control(
            &mut report,
            format!("control_axis_templates.{index}"),
            control,
            &all_role_ids,
            &provider_slot_ids,
            &mut control_ids,
        );
    }

    let mut strategy_ids: BTreeSet<String> = BTreeSet::new();
    for (index, strategy) in archetype.candidate_strategy_templates.iter().enumerate() {
        validate_strategy(
            &mut report,
            format!("candidate_strategy_templates.{index}"),
            strategy,
            &all_role_ids,
            &control_ids,
            &mut strategy_ids,
        );
    }

    if archetype.quality_gate_templates.is_empty() {
        report.push(
            "quality_gate_templates",
            "empty_quality_gates",
            "Archetype quality gates must not be empty.",
        );
    }
    let mut gate_ids: BTreeSet<String> = BTreeSet::new();
    for (index, gate) in archetype.quality_gate_templates.iter().enumerate() {
        let subject = format!("quality_gate_templates.{index}");
        validate_identifier(
            &mut report,
            format!("{subject}.gate_id"),
            &gate.gate_id,
            "invalid_quality_gate_id",
        );
        validate_non_empty_label(
            &mut report,
            format!("{subject}.display_name"),
            &gate.display_name,
        );
        if !gate_ids.insert(gate.gate_id.clone()) {
            report.push(
                format!("{subject}.gate_id"),
                "duplicate_quality_gate_id",
                "Quality gate IDs must be unique.",
            );
        }
    }

    for (index, tag) in archetype.compatible_style_tags.iter().enumerate() {
        validate_tag(&mut report, format!("compatible_style_tags.{index}"), tag);
    }

    report
}

fn validate_role(
    report: &mut FoundryArchetypeValidationReport,
    subject: String,
    role: &ArchetypeRoleTemplate,
    expected_required: bool,
    required_role_ids: &mut BTreeSet<String>,
    all_role_ids: &mut BTreeSet<String>,
) {
    validate_identifier(
        report,
        format!("{subject}.role_id"),
        &role.role_id,
        "invalid_role_id",
    );
    validate_non_empty_label(
        report,
        format!("{subject}.display_name"),
        &role.display_name,
    );
    validate_product_safe_label(
        report,
        format!("{subject}.display_name"),
        &role.display_name,
    );
    validate_identifier(
        report,
        format!("{subject}.semantic_part_group"),
        &role.semantic_part_group,
        "invalid_semantic_part_group",
    );
    if role.required != expected_required {
        report.push(
            format!("{subject}.required"),
            "role_required_flag_mismatch",
            "Role required flag must match required or optional role list.",
        );
    }
    if expected_required && !required_role_ids.insert(role.role_id.clone()) {
        report.push(
            format!("{subject}.role_id"),
            "duplicate_required_role_id",
            "Required role IDs must be unique.",
        );
    }
    if !all_role_ids.insert(role.role_id.clone()) {
        report.push(
            format!("{subject}.role_id"),
            "duplicate_role_id",
            "Role IDs must be unique across required and optional roles.",
        );
    }
    for (index, slot_id) in role.allowed_provider_slot_ids.iter().enumerate() {
        validate_identifier(
            report,
            format!("{subject}.allowed_provider_slot_ids.{index}"),
            slot_id,
            "invalid_allowed_provider_slot_id",
        );
    }
}

fn validate_provider_slot(
    report: &mut FoundryArchetypeValidationReport,
    subject: String,
    slot: &ArchetypeProviderSlotTemplate,
    all_role_ids: &BTreeSet<String>,
    provider_slot_ids: &mut BTreeSet<String>,
) {
    validate_identifier(
        report,
        format!("{subject}.slot_id"),
        &slot.slot_id,
        "invalid_provider_slot_id",
    );
    validate_non_empty_label(
        report,
        format!("{subject}.display_name"),
        &slot.display_name,
    );
    validate_identifier(
        report,
        format!("{subject}.target_role_id"),
        &slot.target_role_id,
        "invalid_provider_slot_role_id",
    );
    if !all_role_ids.contains(&slot.target_role_id) {
        report.push(
            format!("{subject}.target_role_id"),
            "unknown_provider_slot_role",
            "Provider slot references an unknown role.",
        );
    }
    if !provider_slot_ids.insert(slot.slot_id.clone()) {
        report.push(
            format!("{subject}.slot_id"),
            "duplicate_provider_slot_id",
            "Provider slot IDs must be unique.",
        );
    }
    for (index, tag) in slot.compatible_style_tags.iter().enumerate() {
        validate_tag(
            report,
            format!("{subject}.compatible_style_tags.{index}"),
            tag,
        );
    }
}

fn validate_control(
    report: &mut FoundryArchetypeValidationReport,
    subject: String,
    control: &ArchetypeControlAxisTemplate,
    all_role_ids: &BTreeSet<String>,
    provider_slot_ids: &BTreeSet<String>,
    control_ids: &mut BTreeSet<String>,
) {
    validate_identifier(
        report,
        format!("{subject}.control_id"),
        &control.control_id,
        "invalid_control_id",
    );
    validate_non_empty_label(
        report,
        format!("{subject}.display_name"),
        &control.display_name,
    );
    validate_product_safe_label(
        report,
        format!("{subject}.display_name"),
        &control.display_name,
    );
    if !control_ids.insert(control.control_id.clone()) {
        report.push(
            format!("{subject}.control_id"),
            "duplicate_control_id",
            "Control IDs must be unique.",
        );
    }
    for (index, role_id) in control.owns_role_ids.iter().enumerate() {
        validate_identifier(
            report,
            format!("{subject}.owns_role_ids.{index}"),
            role_id,
            "invalid_control_role_id",
        );
        if !all_role_ids.contains(role_id) {
            report.push(
                format!("{subject}.owns_role_ids.{index}"),
                "unknown_control_role",
                "Control references an unknown role.",
            );
        }
    }
    for (index, slot_id) in control.owns_provider_slot_ids.iter().enumerate() {
        validate_identifier(
            report,
            format!("{subject}.owns_provider_slot_ids.{index}"),
            slot_id,
            "invalid_control_provider_slot_id",
        );
        if !provider_slot_ids.contains(slot_id) {
            report.push(
                format!("{subject}.owns_provider_slot_ids.{index}"),
                "unknown_control_provider_slot",
                "Control references an unknown provider slot.",
            );
        }
    }
}

fn validate_strategy(
    report: &mut FoundryArchetypeValidationReport,
    subject: String,
    strategy: &ArchetypeCandidateStrategyTemplate,
    all_role_ids: &BTreeSet<String>,
    control_ids: &BTreeSet<String>,
    strategy_ids: &mut BTreeSet<String>,
) {
    validate_identifier(
        report,
        format!("{subject}.strategy_id"),
        &strategy.strategy_id,
        "invalid_candidate_strategy_id",
    );
    validate_non_empty_label(
        report,
        format!("{subject}.display_name"),
        &strategy.display_name,
    );
    if !strategy_ids.insert(strategy.strategy_id.clone()) {
        report.push(
            format!("{subject}.strategy_id"),
            "duplicate_candidate_strategy_id",
            "Candidate strategy IDs must be unique.",
        );
    }
    for (index, control_id) in strategy.intended_changed_controls.iter().enumerate() {
        validate_identifier(
            report,
            format!("{subject}.intended_changed_controls.{index}"),
            control_id,
            "invalid_candidate_strategy_control_id",
        );
        if !control_ids.contains(control_id) {
            report.push(
                format!("{subject}.intended_changed_controls.{index}"),
                "unknown_candidate_strategy_control",
                "Candidate strategy references an unknown control.",
            );
        }
    }
    for (index, role_id) in strategy.intended_changed_roles.iter().enumerate() {
        validate_identifier(
            report,
            format!("{subject}.intended_changed_roles.{index}"),
            role_id,
            "invalid_candidate_strategy_role_id",
        );
        if !all_role_ids.contains(role_id) {
            report.push(
                format!("{subject}.intended_changed_roles.{index}"),
                "unknown_candidate_strategy_role",
                "Candidate strategy references an unknown role.",
            );
        }
    }
}

fn validate_identifier(
    report: &mut FoundryArchetypeValidationReport,
    subject: impl Into<String>,
    value: &str,
    code: &'static str,
) {
    if !is_normalized_identifier(value) {
        report.push(
            subject,
            code,
            "Identifier must be lowercase ASCII with letters, numbers, hyphens, or underscores.",
        );
    }
}

fn validate_tag(
    report: &mut FoundryArchetypeValidationReport,
    subject: impl Into<String>,
    value: &str,
) {
    validate_identifier(report, subject, value, "style_tag_not_normalized");
}

fn validate_non_empty_label(
    report: &mut FoundryArchetypeValidationReport,
    subject: impl Into<String>,
    value: &str,
) {
    if value.trim().is_empty() {
        report.push(subject, "empty_label", "Display labels must not be empty.");
    }
}

fn validate_product_safe_label(
    report: &mut FoundryArchetypeValidationReport,
    subject: impl Into<String>,
    value: &str,
) {
    let lower = value.to_ascii_lowercase();
    for forbidden in [
        "scalar path",
        "provider id",
        "semantic id",
        "operation id",
        "socket",
        "port id",
        "fragment",
        "remap",
        "conformance",
    ] {
        if lower.contains(forbidden) {
            report.push(
                subject,
                "role_label_not_product_safe",
                "Display label contains internal authoring terminology.",
            );
            break;
        }
    }
}

fn is_normalized_identifier(value: &str) -> bool {
    !value.is_empty()
        && value.chars().all(|character| {
            character.is_ascii_lowercase()
                || character.is_ascii_digit()
                || character == '-'
                || character == '_'
        })
}
