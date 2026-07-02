
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
