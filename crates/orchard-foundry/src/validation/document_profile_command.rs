
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
