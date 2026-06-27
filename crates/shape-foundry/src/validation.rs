//! Validation for foundry source and control contracts.

use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};
use shape_family::ParameterExecutionPolicy;

use crate::{
    CATALOG_LOCK_KEY_FAMILY, CATALOG_LOCK_KEY_STYLE, CUSTOMIZER_PROFILE_SCHEMA_VERSION,
    CatalogContentRef, ChoiceOption, ClosedInterval, ControlKind, ControlSlotBinding,
    ControlTopologyBehavior, ControlValue, CustomizerControl, CustomizerProfile,
    DomainCertification, FOUNDRY_ASSET_DOCUMENT_SCHEMA_VERSION,
    FOUNDRY_PACK_DOCUMENT_SCHEMA_VERSION, FoundryAssetDocument, FoundryCommand, FoundryLockTarget,
    FoundryPackDocument, PackCoherencePolicy, ProviderOption, ResponseCurve, SharedProviderPolicy,
    VariationChannel, VariationIntent, VariationScope, document_catalog_refs,
};

/// One foundry contract validation issue.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FoundryValidationIssue {
    /// Stable subject path.
    pub subject: String,
    /// Stable issue code.
    pub code: String,
    /// Human-readable message.
    pub message: String,
}

/// Foundry contract validation report.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct FoundryValidationReport {
    /// Issues discovered during validation.
    pub issues: Vec<FoundryValidationIssue>,
}

impl FoundryValidationReport {
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
        self.issues.push(FoundryValidationIssue {
            subject: subject.into(),
            code: code.into(),
            message: message.into(),
        });
    }

    fn extend_prefixed(&mut self, prefix: &str, nested: FoundryValidationReport) {
        for issue in nested.issues {
            self.issues.push(FoundryValidationIssue {
                subject: format!("{prefix}.{}", issue.subject),
                code: issue.code,
                message: issue.message,
            });
        }
    }
}

/// Validate a foundry asset document contract.
#[must_use]
pub fn validate_foundry_document(document: &FoundryAssetDocument) -> FoundryValidationReport {
    let mut report = FoundryValidationReport::default();
    if document.schema_version != FOUNDRY_ASSET_DOCUMENT_SCHEMA_VERSION {
        report.push(
            "schema_version",
            "unsupported_foundry_document_schema",
            "Foundry asset document schema version is not supported.",
        );
    }
    validate_identifier(&mut report, "document_id", &document.document_id.0);
    validate_ref(
        &mut report,
        "family_content_ref",
        &document.family_content_ref,
    );
    validate_ref(
        &mut report,
        "style_content_ref",
        &document.style_content_ref,
    );
    validate_ref(
        &mut report,
        "family_implementation_ref",
        &document.family_implementation_ref,
    );
    validate_ref(
        &mut report,
        "style_implementation_ref",
        &document.style_implementation_ref,
    );
    validate_ref(
        &mut report,
        "customizer_profile_ref",
        &document.customizer_profile_ref,
    );
    for (control_id, value) in &document.control_state {
        validate_identifier(
            &mut report,
            format!("control_state.{control_id}.key"),
            control_id,
        );
        validate_control_value(
            &mut report,
            format!("control_state.{control_id}.value"),
            value,
        );
    }
    for (role, override_row) in &document.provider_overrides {
        validate_identifier(&mut report, format!("provider_overrides.{role}.key"), role);
        if role != &override_row.role {
            report.push(
                format!("provider_overrides.{role}.role"),
                "provider_override_role_mismatch",
                "Provider override map key must match the override role.",
            );
        }
        validate_ref(
            &mut report,
            format!("provider_overrides.{role}.provider_ref"),
            &override_row.provider_ref,
        );
    }
    for (index, lock) in document.foundry_locks.iter().enumerate() {
        if matches!(lock.reason.as_deref(), Some("")) {
            report.push(
                format!("foundry_locks.{index}.reason"),
                "empty_lock_reason",
                "Lock reason must not be empty when present.",
            );
        }
    }
    let mut override_ids = BTreeSet::new();
    for (index, override_row) in document.local_recipe_overrides.iter().enumerate() {
        validate_identifier(
            &mut report,
            format!("local_recipe_overrides.{index}.id"),
            &override_row.id.0,
        );
        if !override_ids.insert(override_row.id.0.as_str()) {
            report.push(
                format!("local_recipe_overrides.{index}.id"),
                "duplicate_local_override_id",
                "Local override IDs must be unique within one document.",
            );
        }
        if override_row.edit_program.operations.is_empty() {
            report.push(
                format!("local_recipe_overrides.{index}.edit_program.operations"),
                "empty_local_override_program",
                "Local overrides must contain at least one edit operation.",
            );
        }
        if override_row.touched_targets.is_empty() {
            report.push(
                format!("local_recipe_overrides.{index}.touched_targets"),
                "missing_override_touched_targets",
                "Local overrides must declare touched semantic targets.",
            );
        }
    }
    if let Some(lock) = &document.catalog_lock {
        let expected_refs = document_catalog_refs(document);
        for (key, expected) in &expected_refs {
            validate_catalog_lock_ref(
                &mut report,
                lock.exact_refs.get(key),
                &format!("catalog_lock.exact_refs.{key}"),
                expected,
            );
        }
        for key in lock.exact_refs.keys() {
            if !expected_refs.contains_key(key) {
                report.push(
                    format!("catalog_lock.exact_refs.{key}"),
                    "extra_catalog_lock_ref",
                    "Catalog lock contains an unused exact content reference.",
                );
            }
        }
    }
    report
}

/// Validate a whole-model customizer profile.
#[must_use]
pub fn validate_customizer_profile(profile: &CustomizerProfile) -> FoundryValidationReport {
    let mut report = FoundryValidationReport::default();
    if profile.schema_version != CUSTOMIZER_PROFILE_SCHEMA_VERSION {
        report.push(
            "schema_version",
            "unsupported_customizer_profile_schema",
            "Customizer profile schema version is not supported.",
        );
    }
    validate_identifier(&mut report, "family_id", &profile.family_id);
    if let Some(style_id) = &profile.style_id {
        validate_identifier(&mut report, "style_id", style_id);
    }

    let mut section_ids = BTreeSet::new();
    for (index, section) in profile.sections.iter().enumerate() {
        validate_identifier(&mut report, format!("sections.{index}.id"), &section.id);
        validate_non_empty(
            &mut report,
            format!("sections.{index}.label"),
            &section.label,
        );
        if !section_ids.insert(section.id.as_str()) {
            report.push(
                format!("sections.{index}.id"),
                "duplicate_customizer_section",
                "Customizer section IDs must be unique.",
            );
        }
    }

    let mut control_ids = BTreeSet::new();
    let mut slot_owners = BTreeMap::<String, String>::new();
    let mut provider_role_owners = BTreeMap::<String, String>::new();
    let mut primary_count = 0_u32;
    for (index, control) in profile.controls.iter().enumerate() {
        validate_control(
            &mut report,
            index,
            control,
            &section_ids,
            &mut slot_owners,
            &mut provider_role_owners,
        );
        if !control_ids.insert(control.id.as_str()) {
            report.push(
                format!("controls.{index}.id"),
                "duplicate_customizer_control",
                "Customizer control IDs must be unique.",
            );
        }
        if control.primary {
            primary_count = primary_count.saturating_add(1);
        }
    }
    if primary_count > profile.maximum_primary_controls {
        report.push(
            "controls",
            "too_many_primary_controls",
            "Customizer profile exceeds its maximum primary control count.",
        );
    }

    for (index, strategy) in profile.candidate_strategies.iter().enumerate() {
        validate_identifier(
            &mut report,
            format!("candidate_strategies.{index}.id"),
            &strategy.id,
        );
        validate_non_empty(
            &mut report,
            format!("candidate_strategies.{index}.label"),
            &strategy.label,
        );
        for (control_index, control_id) in strategy.control_ids.iter().enumerate() {
            if !control_ids.contains(control_id.as_str()) {
                report.push(
                    format!("candidate_strategies.{index}.control_ids.{control_index}"),
                    "unknown_strategy_control",
                    "Candidate strategy references an unknown control.",
                );
            }
        }
    }
    report
}

/// Validate a command against an optional document/profile context.
#[must_use]
pub fn validate_foundry_command(
    command: &FoundryCommand,
    document: Option<&FoundryAssetDocument>,
    profile: Option<&CustomizerProfile>,
) -> FoundryValidationReport {
    let mut report = FoundryValidationReport::default();
    match command {
        FoundryCommand::SetControl { control_id, value } => {
            validate_identifier(&mut report, "set_control.control_id", control_id);
            validate_control_value(&mut report, "set_control.value", value);
            if let Some(profile) = profile {
                if let Some(control) = profile
                    .controls
                    .iter()
                    .find(|control| control.id == *control_id)
                {
                    validate_set_control_value(&mut report, control, value);
                } else {
                    report.push(
                        "set_control.control_id",
                        "unknown_command_control",
                        "SetControl references an unknown control.",
                    );
                }
            }
        }
        FoundryCommand::ResetControl { control_id } => {
            validate_identifier(&mut report, "reset_control.control_id", control_id);
            if let Some(profile) = profile
                && !profile
                    .controls
                    .iter()
                    .any(|control| control.id == *control_id)
            {
                report.push(
                    "reset_control.control_id",
                    "unknown_command_control",
                    "ResetControl references an unknown control.",
                );
            }
        }
        FoundryCommand::SelectProvider { role, provider_ref } => {
            validate_identifier(&mut report, "select_provider.role", role);
            validate_ref(&mut report, "select_provider.provider_ref", provider_ref);
        }
        FoundryCommand::SetRolePresence { role, .. } => {
            validate_identifier(&mut report, "set_role_presence.role", role);
        }
        FoundryCommand::SetStyle {
            style_content_ref,
            style_implementation_ref,
        } => {
            validate_ref(
                &mut report,
                "set_style.style_content_ref",
                style_content_ref,
            );
            validate_ref(
                &mut report,
                "set_style.style_implementation_ref",
                style_implementation_ref,
            );
        }
        FoundryCommand::SetVariationIntent { intent } => {
            validate_variation_intent(&mut report, "set_variation_intent.intent", intent);
        }
        FoundryCommand::SetVariationScope { scope } => {
            validate_variation_scope(&mut report, "set_variation_scope.scope", scope);
        }
        FoundryCommand::SetVariationChannels { channels } => {
            if channels.is_empty() {
                report.push(
                    "set_variation_channels.channels",
                    "empty_variation_channels",
                    "Variation channels must include at least one channel.",
                );
            }
            for (index, channel) in channels.iter().enumerate() {
                validate_variation_channel(
                    &mut report,
                    &format!("set_variation_channels.channels.{index}"),
                    channel,
                );
            }
        }
        FoundryCommand::ClearVariationFocus | FoundryCommand::ClearFocusPartGroup => {}
        FoundryCommand::SetFocusPartGroup { group_id } => {
            validate_identifier(&mut report, "set_focus_part_group.group_id", group_id);
        }
        FoundryCommand::GenerateFocusedPartCandidates {
            group_id,
            channels,
            mode,
        } => {
            validate_identifier(
                &mut report,
                "generate_focused_part_candidates.group_id",
                group_id,
            );
            if channels.is_empty() {
                report.push(
                    "generate_focused_part_candidates.channels",
                    "empty_variation_channels",
                    "Focused candidates need at least one variation channel.",
                );
            }
            for (index, channel) in channels.iter().enumerate() {
                validate_variation_channel(
                    &mut report,
                    &format!("generate_focused_part_candidates.channels.{index}"),
                    channel,
                );
            }
            if mode.trim().is_empty() {
                report.push(
                    "generate_focused_part_candidates.mode",
                    "empty_generation_mode",
                    "Focused candidates need a generation mode.",
                );
            }
        }
        FoundryCommand::GenerateCandidates(request) => {
            if request.count == 0 {
                report.push(
                    "generate_candidates.count",
                    "empty_candidate_request",
                    "GenerateCandidates must request at least one candidate.",
                );
            }
            if let (Some(profile), Some(strategy_id)) = (profile, &request.strategy_id)
                && !profile
                    .candidate_strategies
                    .iter()
                    .any(|strategy| strategy.id == *strategy_id)
            {
                report.push(
                    "generate_candidates.strategy_id",
                    "unknown_candidate_strategy",
                    "GenerateCandidates references an unknown strategy.",
                );
            }
        }
        FoundryCommand::AcceptCandidate { candidate_id }
        | FoundryCommand::RejectCandidate { candidate_id } => {
            validate_identifier(&mut report, "candidate_id", &candidate_id.0);
        }
        FoundryCommand::SwitchRevision { .. } | FoundryCommand::Undo => {}
        FoundryCommand::SetLock { lock } => {
            if matches!(lock.reason.as_deref(), Some("")) {
                report.push(
                    "set_lock.reason",
                    "empty_lock_reason",
                    "Lock reason must not be empty when present.",
                );
            }
        }
        FoundryCommand::ClearLock { .. } => {}
        FoundryCommand::Export { profile, .. } => {
            validate_identifier(&mut report, "export.profile", profile);
        }
        FoundryCommand::AddCurrentToPack { pack_id, member_id } => {
            validate_identifier(&mut report, "add_current_to_pack.pack_id", pack_id);
            validate_identifier(&mut report, "add_current_to_pack.member_id", member_id);
        }
    }
    if let Some(document) = document {
        report.extend_prefixed("document", validate_foundry_document(document));
    }
    report
}

/// Validate a foundry pack document contract.
#[must_use]
pub fn validate_foundry_pack(pack: &FoundryPackDocument) -> FoundryValidationReport {
    let mut report = FoundryValidationReport::default();
    if pack.schema_version != FOUNDRY_PACK_DOCUMENT_SCHEMA_VERSION {
        report.push(
            "schema_version",
            "unsupported_foundry_pack_schema",
            "Foundry pack schema version is not supported.",
        );
    }
    validate_identifier(&mut report, "pack_id", &pack.pack_id);
    validate_ref(&mut report, "shared_family_ref", &pack.shared_family_ref);
    validate_ref(&mut report, "shared_style_ref", &pack.shared_style_ref);
    validate_identifier(
        &mut report,
        "export_profile.profile",
        &pack.export_profile.profile,
    );
    for (control_id, value) in &pack.shared_controls {
        validate_identifier(
            &mut report,
            format!("shared_controls.{control_id}.key"),
            control_id,
        );
        validate_control_value(
            &mut report,
            format!("shared_controls.{control_id}.value"),
            value,
        );
    }
    if pack.members.is_empty() {
        report.push(
            "members",
            "empty_foundry_pack",
            "Foundry packs must contain at least one member document.",
        );
    }
    let mut shared_lock_targets = BTreeSet::new();
    for (index, lock) in pack.shared_locks.iter().enumerate() {
        if !shared_lock_targets.insert(&lock.target) {
            report.push(
                format!("shared_locks.{index}.target"),
                "duplicate_shared_lock_target",
                "Pack shared locks must target unique subjects.",
            );
        }
        if matches!(lock.reason.as_deref(), Some("")) {
            report.push(
                format!("shared_locks.{index}.reason"),
                "empty_lock_reason",
                "Lock reason must not be empty when present.",
            );
        }
    }
    if let SharedProviderPolicy::SharedExact(providers) = &pack.shared_provider_policy {
        for (role, provider_ref) in providers {
            validate_identifier(
                &mut report,
                format!("shared_provider_policy.{role}.key"),
                role,
            );
            validate_ref(
                &mut report,
                format!("shared_provider_policy.{role}.provider_ref"),
                provider_ref,
            );
        }
    }
    if let Some(lock) = &pack.catalog_lock {
        let shared_family_ref = if pack.coherence_policy == PackCoherencePolicy::ExactFamilyAndStyle
        {
            Some(&pack.shared_family_ref)
        } else {
            shared_member_catalog_ref(pack, CATALOG_LOCK_KEY_FAMILY)
        };
        validate_pack_catalog_lock_ref(
            &mut report,
            lock.exact_refs.get(CATALOG_LOCK_KEY_FAMILY),
            "catalog_lock.exact_refs.family",
            shared_family_ref,
            true,
        );
        if pack.coherence_policy == PackCoherencePolicy::ExactFamilyAndStyle {
            validate_pack_catalog_lock_ref(
                &mut report,
                lock.exact_refs.get(CATALOG_LOCK_KEY_STYLE),
                "catalog_lock.exact_refs.style",
                Some(&pack.shared_style_ref),
                true,
            );
        } else {
            validate_pack_catalog_lock_ref(
                &mut report,
                lock.exact_refs.get(CATALOG_LOCK_KEY_STYLE),
                "catalog_lock.exact_refs.style",
                shared_member_catalog_ref(pack, CATALOG_LOCK_KEY_STYLE),
                false,
            );
        }
    }
    for (member_id, document) in &pack.members {
        validate_identifier(&mut report, format!("members.{member_id}.key"), member_id);
        report.extend_prefixed(
            &format!("members.{member_id}"),
            validate_foundry_document(document),
        );
        if pack.coherence_policy == PackCoherencePolicy::ExactFamilyAndStyle {
            if document.family_content_ref != pack.shared_family_ref {
                report.push(
                    format!("members.{member_id}.family_content_ref"),
                    "pack_member_family_mismatch",
                    "Pack member family ref must match the shared family ref.",
                );
            }
            if document.style_content_ref != pack.shared_style_ref {
                report.push(
                    format!("members.{member_id}.style_content_ref"),
                    "pack_member_style_mismatch",
                    "Pack member style ref must match the shared style ref.",
                );
            }
        }
        if let SharedProviderPolicy::SharedExact(providers) = &pack.shared_provider_policy {
            for (role, expected_ref) in providers {
                if let Some(actual) = document.provider_overrides.get(role)
                    && actual.provider_ref != *expected_ref
                {
                    report.push(
                        format!("members.{member_id}.provider_overrides.{role}"),
                        "pack_member_provider_conflict",
                        "Pack member provider override conflicts with shared provider policy.",
                    );
                }
            }
        }
        for (lock_index, lock) in pack.shared_locks.iter().enumerate() {
            if let FoundryLockTarget::Provider(role) = &lock.target
                && document.provider_overrides.contains_key(role)
            {
                report.push(
                    format!("members.{member_id}.provider_overrides.{role}"),
                    "pack_shared_lock_conflict",
                    format!(
                        "Pack shared provider lock at index {lock_index} forbids this member override."
                    ),
                );
            }
        }
    }
    report
}

fn validate_control(
    report: &mut FoundryValidationReport,
    index: usize,
    control: &CustomizerControl,
    section_ids: &BTreeSet<&str>,
    slot_owners: &mut BTreeMap<String, String>,
    provider_role_owners: &mut BTreeMap<String, String>,
) {
    validate_identifier(report, format!("controls.{index}.id"), &control.id);
    validate_non_empty(report, format!("controls.{index}.label"), &control.label);
    if let Some(section) = &control.section {
        validate_identifier(report, format!("controls.{index}.section"), section);
        if !section_ids.contains(section.as_str()) {
            report.push(
                format!("controls.{index}.section"),
                "unknown_control_section",
                "Control references an unknown section.",
            );
        }
    }
    if control.bindings.is_empty() && !matches!(control.kind, ControlKind::ProviderGallery { .. }) {
        report.push(
            format!("controls.{index}.bindings"),
            "control_without_bindings",
            "Controls must own at least one family slot binding.",
        );
    }
    for (binding_index, binding) in control.bindings.iter().enumerate() {
        validate_control_binding(report, index, binding_index, control.visible, binding);
        if let Some(previous) = slot_owners.insert(binding.slot.clone(), control.id.clone()) {
            report.push(
                format!("controls.{index}.bindings.{binding_index}.slot"),
                "conflicting_control_ownership",
                format!(
                    "Family slot `{}` is already owned by control `{previous}`.",
                    binding.slot
                ),
            );
        }
    }
    match &control.kind {
        ControlKind::ContinuousAxis { default } => {
            if !default.is_finite() {
                report.push(
                    format!("controls.{index}.kind.default"),
                    "non_finite_control_default",
                    "Continuous control defaults must be finite.",
                );
            }
            if control.topology_behavior == ControlTopologyBehavior::TopologyChanging {
                report.push(
                    format!("controls.{index}.topology_behavior"),
                    "topology_changing_live_control",
                    "Topology-changing controls must be represented as discrete values.",
                );
            }
            if control.domain.certification != DomainCertification::CertifiedContinuous {
                report.push(
                    format!("controls.{index}.domain.certification"),
                    "uncertified_continuous_control",
                    "ContinuousAxis controls require a certified continuous domain.",
                );
            }
            if control.domain.continuous_intervals.is_empty() {
                report.push(
                    format!("controls.{index}.domain.continuous_intervals"),
                    "missing_continuous_control_interval",
                    "ContinuousAxis controls require at least one continuous interval.",
                );
            }
        }
        ControlKind::IntegerStepper { .. } | ControlKind::Toggle { .. } => {
            if !control.domain.continuous_intervals.is_empty() {
                report.push(
                    format!("controls.{index}.domain.continuous_intervals"),
                    "continuous_domain_for_discrete_control",
                    "Discrete controls must expose discrete values rather than continuous intervals.",
                );
            }
        }
        ControlKind::ChoiceGallery { options } => {
            validate_choice_options(report, index, options);
            validate_choice_domain(report, index, control, options);
        }
        ControlKind::ProviderGallery { role, options } => {
            validate_identifier(report, format!("controls.{index}.kind.role"), role);
            if let Some(first_control_id) =
                provider_role_owners.insert(role.clone(), control.id.clone())
            {
                report.push(
                    format!("controls.{index}.kind.role"),
                    "conflicting_provider_role_owner",
                    format!(
                        "Provider role `{role}` is already controlled by `{first_control_id}`."
                    ),
                );
            }
            validate_provider_options(report, index, options);
            validate_provider_domain(report, index, control, options);
        }
    }
    if control.topology_behavior == ControlTopologyBehavior::TopologyChanging
        && !control.domain.continuous_intervals.is_empty()
    {
        report.push(
            format!("controls.{index}.domain.continuous_intervals"),
            "topology_changing_continuous_domain",
            "Topology-changing controls must be represented by discrete values only.",
        );
    }
    validate_domain(report, index, control);
}

fn validate_control_binding(
    report: &mut FoundryValidationReport,
    control_index: usize,
    binding_index: usize,
    visible: bool,
    binding: &ControlSlotBinding,
) {
    validate_identifier(
        report,
        format!("controls.{control_index}.bindings.{binding_index}.slot"),
        &binding.slot,
    );
    if visible && binding.slot_policy != ParameterExecutionPolicy::RequiredBinding {
        report.push(
            format!("controls.{control_index}.bindings.{binding_index}.slot_policy"),
            "visible_control_non_required_slot",
            "Visible customizer controls should own RequiredBinding slots; advisory/runtime slots belong in metadata until a runtime adapter consumes them.",
        );
    }
    validate_response_curve(
        report,
        format!("controls.{control_index}.bindings.{binding_index}.response"),
        &binding.response,
    );
}

fn validate_domain(
    report: &mut FoundryValidationReport,
    control_index: usize,
    control: &CustomizerControl,
) {
    let available_discrete_count = control
        .domain
        .discrete_values
        .iter()
        .filter(|value| {
            !control
                .domain
                .unavailable_options
                .contains_key(&control_value_option_key(value))
        })
        .count();
    if control.domain.continuous_intervals.is_empty() && available_discrete_count == 0 {
        report.push(
            format!("controls.{control_index}.domain"),
            "empty_feasible_control_domain",
            "A control domain must expose at least one available value.",
        );
    }
    for (index, interval) in control.domain.continuous_intervals.iter().enumerate() {
        validate_interval(
            report,
            format!("controls.{control_index}.domain.continuous_intervals.{index}"),
            *interval,
        );
    }
    for (index, value) in control.domain.discrete_values.iter().enumerate() {
        validate_control_value(
            report,
            format!("controls.{control_index}.domain.discrete_values.{index}"),
            value,
        );
        if !control_value_matches_kind(&control.kind, value) {
            report.push(
                format!("controls.{control_index}.domain.discrete_values.{index}"),
                "control_domain_value_kind_mismatch",
                "Control domain value type must match the control kind.",
            );
        }
    }
    for (option, reason) in &control.domain.unavailable_options {
        validate_identifier(
            report,
            format!("controls.{control_index}.domain.unavailable_options.{option}"),
            option,
        );
        validate_non_empty(
            report,
            format!("controls.{control_index}.domain.unavailable_options.{option}.reason"),
            reason,
        );
    }
}

fn validate_interval(
    report: &mut FoundryValidationReport,
    subject: String,
    interval: ClosedInterval,
) {
    if !interval.minimum.is_finite() || !interval.maximum.is_finite() {
        report.push(
            subject,
            "non_finite_control_interval",
            "Continuous control intervals must be finite.",
        );
    } else if interval.minimum > interval.maximum {
        report.push(
            subject,
            "inverted_control_interval",
            "Continuous control interval minimum must not exceed maximum.",
        );
    }
}

fn validate_choice_options(
    report: &mut FoundryValidationReport,
    control_index: usize,
    options: &[ChoiceOption],
) {
    if options.is_empty() {
        report.push(
            format!("controls.{control_index}.kind.options"),
            "empty_choice_gallery",
            "ChoiceGallery controls must contain at least one option.",
        );
    }
    let mut seen = BTreeSet::new();
    for (index, option) in options.iter().enumerate() {
        validate_identifier(
            report,
            format!("controls.{control_index}.kind.options.{index}.value"),
            &option.value,
        );
        validate_non_empty(
            report,
            format!("controls.{control_index}.kind.options.{index}.label"),
            &option.label,
        );
        validate_non_empty(
            report,
            format!("controls.{control_index}.kind.options.{index}.preview.preview_id"),
            &option.preview.preview_id,
        );
        if !seen.insert(option.value.as_str()) {
            report.push(
                format!("controls.{control_index}.kind.options.{index}.value"),
                "duplicate_choice_option",
                "Choice option values must be unique.",
            );
        }
    }
}

fn validate_provider_options(
    report: &mut FoundryValidationReport,
    control_index: usize,
    options: &[ProviderOption],
) {
    if options.is_empty() {
        report.push(
            format!("controls.{control_index}.kind.options"),
            "empty_provider_gallery",
            "ProviderGallery controls must contain at least one option.",
        );
    }
    let mut seen = BTreeSet::new();
    for (index, option) in options.iter().enumerate() {
        validate_identifier(
            report,
            format!("controls.{control_index}.kind.options.{index}.provider_id"),
            &option.provider_id,
        );
        validate_non_empty(
            report,
            format!("controls.{control_index}.kind.options.{index}.label"),
            &option.label,
        );
        validate_non_empty(
            report,
            format!("controls.{control_index}.kind.options.{index}.preview.preview_id"),
            &option.preview.preview_id,
        );
        if !seen.insert(option.provider_id.as_str()) {
            report.push(
                format!("controls.{control_index}.kind.options.{index}.provider_id"),
                "duplicate_provider_option",
                "Provider option IDs must be unique.",
            );
        }
    }
}

fn validate_choice_domain(
    report: &mut FoundryValidationReport,
    control_index: usize,
    control: &CustomizerControl,
    options: &[ChoiceOption],
) {
    let option_ids = options
        .iter()
        .map(|option| option.value.as_str())
        .collect::<BTreeSet<_>>();
    validate_symbolic_domain(
        report,
        control_index,
        control,
        &option_ids,
        "choice",
        |value| match value {
            ControlValue::Choice(value) => Some(value.as_str()),
            _ => None,
        },
    );
}

fn validate_provider_domain(
    report: &mut FoundryValidationReport,
    control_index: usize,
    control: &CustomizerControl,
    options: &[ProviderOption],
) {
    let option_ids = options
        .iter()
        .map(|option| option.provider_id.as_str())
        .collect::<BTreeSet<_>>();
    validate_symbolic_domain(
        report,
        control_index,
        control,
        &option_ids,
        "provider",
        |value| match value {
            ControlValue::Provider(value) => Some(value.as_str()),
            _ => None,
        },
    );
}

fn validate_symbolic_domain(
    report: &mut FoundryValidationReport,
    control_index: usize,
    control: &CustomizerControl,
    option_ids: &BTreeSet<&str>,
    option_kind: &str,
    value_key: impl Fn(&ControlValue) -> Option<&str>,
) {
    let mut covered_options = BTreeSet::new();
    for (value_index, value) in control.domain.discrete_values.iter().enumerate() {
        let Some(key) = value_key(value) else {
            continue;
        };
        if !option_ids.contains(key) {
            report.push(
                format!("controls.{control_index}.domain.discrete_values.{value_index}"),
                format!("unknown_{option_kind}_domain_value"),
                format!("Control domain references an unknown {option_kind} option."),
            );
        }
        covered_options.insert(key);
    }
    for option in control.domain.unavailable_options.keys() {
        if !option_ids.contains(option.as_str()) {
            report.push(
                format!("controls.{control_index}.domain.unavailable_options.{option}"),
                format!("unknown_unavailable_{option_kind}_option"),
                format!("Unavailable option references an unknown {option_kind} option."),
            );
        }
        covered_options.insert(option.as_str());
    }
    for option in option_ids {
        if !covered_options.contains(option) {
            report.push(
                format!("controls.{control_index}.domain"),
                format!("missing_{option_kind}_option_domain"),
                format!("Every {option_kind} option must be marked available or unavailable."),
            );
        }
    }
}

fn validate_response_curve(
    report: &mut FoundryValidationReport,
    subject: String,
    curve: &ResponseCurve,
) {
    match curve {
        ResponseCurve::Linear => {}
        ResponseCurve::Piecewise { points, monotonic } => {
            if points.len() < 2 {
                report.push(
                    subject,
                    "too_few_response_curve_points",
                    "Piecewise response curves must contain at least two points.",
                );
                return;
            }
            let mut previous_input = f32::NEG_INFINITY;
            let mut previous_output = f32::NEG_INFINITY;
            for (index, [input, output]) in points.iter().copied().enumerate() {
                if !input.is_finite() || !output.is_finite() {
                    report.push(
                        format!("{subject}.points.{index}"),
                        "non_finite_response_curve_point",
                        "Response curve points must be finite.",
                    );
                }
                if input <= previous_input {
                    report.push(
                        format!("{subject}.points.{index}"),
                        "non_increasing_response_curve_input",
                        "Response curve input points must be strictly increasing.",
                    );
                }
                if index > 0 && !(output - previous_output).is_finite() {
                    report.push(
                        format!("{subject}.points.{index}"),
                        "non_finite_response_curve_output",
                        "Response curve interpolation must not produce non-finite output.",
                    );
                }
                if *monotonic && output < previous_output {
                    report.push(
                        format!("{subject}.points.{index}"),
                        "non_monotonic_response_curve",
                        "Response curve output must be monotonic when required.",
                    );
                }
                previous_input = input;
                previous_output = output;
            }
        }
    }
}

fn validate_ref(
    report: &mut FoundryValidationReport,
    subject: impl Into<String>,
    content_ref: &CatalogContentRef,
) {
    let subject = subject.into();
    validate_identifier(
        report,
        format!("{subject}.stable_id"),
        &content_ref.stable_id,
    );
    if content_ref.schema_version == 0 {
        report.push(
            format!("{subject}.schema_version"),
            "invalid_catalog_ref_schema",
            "Catalog content reference schema version must be greater than zero.",
        );
    }
}

fn validate_catalog_lock_ref(
    report: &mut FoundryValidationReport,
    actual: Option<&CatalogContentRef>,
    subject: &str,
    expected: &CatalogContentRef,
) {
    match actual {
        Some(actual) if actual == expected => {}
        Some(_) => report.push(
            subject,
            "catalog_lock_ref_mismatch",
            "Catalog lock reference must match the document's exact content reference.",
        ),
        None => report.push(
            subject,
            "missing_catalog_lock_ref",
            "Catalog lock is missing a required exact content reference.",
        ),
    }
}

fn validate_pack_catalog_lock_ref(
    report: &mut FoundryValidationReport,
    actual: Option<&CatalogContentRef>,
    subject: &str,
    expected: Option<&CatalogContentRef>,
    required: bool,
) {
    match (actual, expected) {
        (Some(actual), Some(expected)) if actual == expected => {}
        (Some(_), _) => report.push(
            subject,
            "catalog_lock_ref_mismatch",
            "Catalog lock reference must match a shared pack content reference.",
        ),
        (None, _) if required => report.push(
            subject,
            "missing_catalog_lock_ref",
            "Catalog lock is missing a required exact content reference.",
        ),
        (None, _) => {}
    }
}

fn shared_member_catalog_ref<'a>(
    pack: &'a FoundryPackDocument,
    key: &str,
) -> Option<&'a CatalogContentRef> {
    let mut documents = pack.members.values();
    let first_ref = documents
        .next()
        .and_then(|document| member_catalog_ref(document, key))?;
    documents
        .all(|document| member_catalog_ref(document, key) == Some(first_ref))
        .then_some(first_ref)
}

fn member_catalog_ref<'a>(
    document: &'a FoundryAssetDocument,
    key: &str,
) -> Option<&'a CatalogContentRef> {
    match key {
        CATALOG_LOCK_KEY_FAMILY => Some(&document.family_content_ref),
        CATALOG_LOCK_KEY_STYLE => Some(&document.style_content_ref),
        _ => None,
    }
}

fn validate_set_control_value(
    report: &mut FoundryValidationReport,
    control: &CustomizerControl,
    value: &ControlValue,
) {
    if !control_value_matches_kind(&control.kind, value) {
        report.push(
            "set_control.value",
            "control_value_kind_mismatch",
            "SetControl value type does not match the referenced control kind.",
        );
        return;
    }
    match (&control.kind, value) {
        (ControlKind::ChoiceGallery { options }, ControlValue::Choice(value))
            if !options.iter().any(|option| option.value == *value) =>
        {
            report.push(
                "set_control.value",
                "unknown_choice_option",
                "SetControl references an unknown choice option.",
            );
        }
        (ControlKind::ProviderGallery { options, .. }, ControlValue::Provider(value))
            if !options.iter().any(|option| option.provider_id == *value) =>
        {
            report.push(
                "set_control.value",
                "unknown_provider_option",
                "SetControl references an unknown provider option.",
            );
        }
        _ => {}
    }
    let option_key = control_value_option_key(value);
    if let Some(reason) = control.domain.unavailable_options.get(&option_key) {
        report.push(
            "set_control.value",
            "unavailable_control_option",
            format!("SetControl selected unavailable option `{option_key}`: {reason}"),
        );
    }
    if !control_value_in_domain(&control.domain, value) {
        report.push(
            "set_control.value",
            "control_value_outside_domain",
            "SetControl value is outside the control's feasible domain.",
        );
    }
}

fn validate_control_value(
    report: &mut FoundryValidationReport,
    subject: impl Into<String>,
    value: &ControlValue,
) {
    if let ControlValue::Scalar(value) = value
        && !value.is_finite()
    {
        report.push(
            subject,
            "non_finite_control_value",
            "Control scalar values must be finite.",
        );
    }
}

fn validate_variation_intent(
    report: &mut FoundryValidationReport,
    subject: &str,
    intent: &VariationIntent,
) {
    validate_variation_scope(report, &format!("{subject}.scope"), &intent.scope);
    if intent.channels.is_empty() {
        report.push(
            format!("{subject}.channels"),
            "empty_variation_channels",
            "Variation intent must include at least one channel.",
        );
    }
    for (index, channel) in intent.channels.iter().enumerate() {
        validate_variation_channel(report, &format!("{subject}.channels.{index}"), channel);
    }
    if intent.human_label.trim().is_empty() {
        report.push(
            format!("{subject}.human_label"),
            "empty_variation_label",
            "Variation intent label must not be empty.",
        );
    }
    if intent.human_summary.trim().is_empty() {
        report.push(
            format!("{subject}.human_summary"),
            "empty_variation_summary",
            "Variation intent summary must not be empty.",
        );
    }
}

fn validate_variation_scope(
    report: &mut FoundryValidationReport,
    subject: &str,
    scope: &VariationScope,
) {
    match scope {
        VariationScope::WholeAsset => {}
        VariationScope::SemanticPartGroup {
            group_id,
            display_name,
        } => validate_variation_scoped_label(report, subject, "group_id", group_id, display_name),
        VariationScope::MaterialSlot {
            slot_id,
            display_name,
        } => validate_variation_scoped_label(report, subject, "slot_id", slot_id, display_name),
        VariationScope::DetailZone {
            zone_id,
            display_name,
        } => validate_variation_scoped_label(report, subject, "zone_id", zone_id, display_name),
        VariationScope::RigRegion {
            region_id,
            display_name,
        } => validate_variation_scoped_label(report, subject, "region_id", region_id, display_name),
        VariationScope::MotionSet {
            motion_set_id,
            display_name,
        } => validate_variation_scoped_label(
            report,
            subject,
            "motion_set_id",
            motion_set_id,
            display_name,
        ),
        VariationScope::Custom {
            scope_id,
            display_name,
        } => validate_variation_scoped_label(report, subject, "scope_id", scope_id, display_name),
    }
}

fn validate_variation_channel(
    report: &mut FoundryValidationReport,
    subject: &str,
    channel: &VariationChannel,
) {
    if let VariationChannel::Custom {
        channel_id,
        display_name,
    } = channel
    {
        validate_identifier(report, format!("{subject}.channel_id"), channel_id);
        if display_name.trim().is_empty() {
            report.push(
                format!("{subject}.display_name"),
                "empty_variation_display_name",
                "Variation channel display name must not be empty.",
            );
        }
    }
}

fn validate_variation_scoped_label(
    report: &mut FoundryValidationReport,
    subject: &str,
    id_field: &str,
    id: &str,
    display_name: &str,
) {
    validate_identifier(report, format!("{subject}.{id_field}"), id);
    if display_name.trim().is_empty() {
        report.push(
            format!("{subject}.display_name"),
            "empty_variation_display_name",
            "Variation scope display name must not be empty.",
        );
    }
}

fn control_value_matches_kind(kind: &ControlKind, value: &ControlValue) -> bool {
    matches!(
        (kind, value),
        (ControlKind::ContinuousAxis { .. }, ControlValue::Scalar(_))
            | (ControlKind::IntegerStepper { .. }, ControlValue::Integer(_))
            | (ControlKind::Toggle { .. }, ControlValue::Toggle(_))
            | (ControlKind::ChoiceGallery { .. }, ControlValue::Choice(_))
            | (
                ControlKind::ProviderGallery { .. },
                ControlValue::Provider(_)
            )
    )
}

fn control_value_in_domain(domain: &crate::FeasibleControlDomain, value: &ControlValue) -> bool {
    match value {
        ControlValue::Scalar(value) => {
            domain
                .continuous_intervals
                .iter()
                .any(|interval| interval.minimum <= *value && *value <= interval.maximum)
                || domain.discrete_values.iter().any(|candidate| {
                    matches!(candidate, ControlValue::Scalar(candidate) if candidate == value)
                })
        }
        ControlValue::Integer(value) => domain
            .discrete_values
            .iter()
            .any(|candidate| matches!(candidate, ControlValue::Integer(candidate) if candidate == value)),
        ControlValue::Toggle(value) => domain
            .discrete_values
            .iter()
            .any(|candidate| matches!(candidate, ControlValue::Toggle(candidate) if candidate == value)),
        ControlValue::Choice(value) => domain
            .discrete_values
            .iter()
            .any(|candidate| matches!(candidate, ControlValue::Choice(candidate) if candidate == value)),
        ControlValue::Provider(value) => domain
            .discrete_values
            .iter()
            .any(|candidate| matches!(candidate, ControlValue::Provider(candidate) if candidate == value)),
    }
}

fn control_value_option_key(value: &ControlValue) -> String {
    match value {
        ControlValue::Scalar(value) => value.to_string(),
        ControlValue::Integer(value) => value.to_string(),
        ControlValue::Toggle(value) => value.to_string(),
        ControlValue::Choice(value) | ControlValue::Provider(value) => value.clone(),
    }
}

fn validate_identifier(
    report: &mut FoundryValidationReport,
    subject: impl Into<String>,
    value: &str,
) {
    let subject = subject.into();
    if value.is_empty() {
        report.push(subject, "empty_identifier", "Identifier must not be empty.");
        return;
    }
    if !value
        .chars()
        .all(|character| character.is_ascii_alphanumeric() || matches!(character, '_' | '-' | '.'))
    {
        report.push(
            subject,
            "invalid_identifier",
            "Identifier must contain only ASCII letters, digits, dashes, underscores, or dots.",
        );
    }
}

fn validate_non_empty(
    report: &mut FoundryValidationReport,
    subject: impl Into<String>,
    value: &str,
) {
    if value.is_empty() {
        report.push(subject, "empty_text", "Text field must not be empty.");
    }
}
