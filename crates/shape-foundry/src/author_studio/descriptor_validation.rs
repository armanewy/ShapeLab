
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
