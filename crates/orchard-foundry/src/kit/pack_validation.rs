
fn validate_control_profile(package: &FoundryKitPackage, report: &mut FoundryKitValidationReport) {
    let maximum_primary = package
        .control_profile
        .maximum_primary_controls
        .min(DEFAULT_MAX_PRIMARY_NOVICE_CONTROLS);
    let primary_count = package
        .control_profile
        .controls
        .iter()
        .filter(|control| control.visible && control.primary)
        .count() as u32;
    if primary_count > maximum_primary {
        report.push(
            "control_profile.controls",
            "too_many_primary_controls",
            format!(
                "Default novice control profiles may expose at most {maximum_primary} primary controls."
            ),
        );
    }

    let family_slots = package
        .family_blueprint
        .provider_slots
        .iter()
        .map(|slot| slot.slot_id.as_str())
        .collect::<BTreeSet<_>>();
    let mut family_slot_owners = BTreeMap::<&str, &str>::new();
    let mut provider_slot_owners = BTreeMap::<&str, &str>::new();
    for control in package
        .control_profile
        .controls
        .iter()
        .filter(|control| control.visible)
    {
        for slot in &control.owned_family_slots {
            if let Some(previous) = family_slot_owners.insert(slot.as_str(), &control.control_id) {
                report.push(
                    format!(
                        "control_profile.controls.{}.owned_family_slots",
                        control.control_id
                    ),
                    "duplicate_visible_control_ownership",
                    format!(
                        "Visible controls '{}' and '{}' both own family slot '{}'.",
                        previous, control.control_id, slot
                    ),
                );
            }
        }
        for slot in &control.owned_provider_slots {
            if !family_slots.is_empty() && !family_slots.contains(slot.as_str()) {
                report.push(
                    format!(
                        "control_profile.controls.{}.owned_provider_slots",
                        control.control_id
                    ),
                    "unknown_provider_slot_ownership",
                    "Visible control owns a provider slot not declared by the family blueprint.",
                );
            }
            if let Some(previous) = provider_slot_owners.insert(slot.as_str(), &control.control_id)
            {
                report.push(
                    format!(
                        "control_profile.controls.{}.owned_provider_slots",
                        control.control_id
                    ),
                    "duplicate_visible_control_ownership",
                    format!(
                        "Visible controls '{}' and '{}' both own provider slot '{}'.",
                        previous, control.control_id, slot
                    ),
                );
            }
        }
    }
    validate_candidate_strategy_pack(package, report);
}

fn validate_candidate_strategy_pack(
    package: &FoundryKitPackage,
    report: &mut FoundryKitValidationReport,
) {
    let visible_control_ids = package
        .control_profile
        .controls
        .iter()
        .filter(|control| control.visible)
        .map(|control| control.control_id.as_str())
        .collect::<BTreeSet<_>>();
    let allowed_controls = package
        .candidate_strategy_pack
        .allowed_controls
        .iter()
        .map(String::as_str)
        .collect::<BTreeSet<_>>();
    for control_id in &package.candidate_strategy_pack.allowed_controls {
        if !visible_control_ids.contains(control_id.as_str()) {
            report.push(
                format!("candidate_strategy_pack.allowed_controls.{control_id}"),
                "candidate_strategy_unknown_control",
                "Candidate strategy allowed controls must exist in the visible control profile.",
            );
        }
    }
    for (index, strategy) in package
        .candidate_strategy_pack
        .strategies
        .iter()
        .enumerate()
    {
        for control_id in &strategy.allowed_controls {
            if !visible_control_ids.contains(control_id.as_str()) {
                report.push(
                    format!(
                        "candidate_strategy_pack.strategies.{index}.allowed_controls.{control_id}"
                    ),
                    "candidate_strategy_unknown_control",
                    "Candidate strategies must reference visible controls.",
                );
            }
            if !allowed_controls.contains(control_id.as_str()) {
                report.push(
                    format!(
                        "candidate_strategy_pack.strategies.{index}.allowed_controls.{control_id}"
                    ),
                    "candidate_strategy_control_not_allowed",
                    "Candidate strategy controls must be listed in the pack allowed_controls set.",
                );
            }
        }
    }

    let supplied_slots = package
        .provider_pack
        .provider_slots_supplied
        .iter()
        .map(String::as_str)
        .collect::<BTreeSet<_>>();
    for (slot, choices) in &package.candidate_strategy_pack.allowed_provider_choices {
        if !supplied_slots.contains(slot.as_str()) {
            report.push(
                format!("candidate_strategy_pack.allowed_provider_choices.{slot}"),
                "candidate_strategy_unknown_provider_slot",
                "Candidate strategy provider choices must reference supplied provider slots.",
            );
        }
        for choice in choices {
            if !package
                .provider_pack
                .provider_options
                .iter()
                .any(|option| option.slot_id == *slot && option.option_id == *choice)
            {
                report.push(
                    format!("candidate_strategy_pack.allowed_provider_choices.{slot}.{choice}"),
                    "candidate_strategy_unknown_provider_choice",
                    "Candidate strategy provider choices must reference provider options for the same slot.",
                );
            }
        }
    }
}

fn validate_quality_and_visibility(
    package: &FoundryKitPackage,
    report: &mut FoundryKitValidationReport,
) {
    let kit = &package.kit;
    let review = &package.review_manifest;
    if review.tier_achieved < package.quality_gate_profile.required_tier {
        report.push(
            "review_manifest.tier_achieved",
            "quality_gate_tier_not_met",
            "Review manifest must meet the quality gate profile required tier.",
        );
    }
    if kit.quality_tier >= FoundryKitQualityTier::Usable && review.contact_sheet_paths.is_empty() {
        report.push(
            "review_manifest.contact_sheet_paths",
            "missing_contact_sheet_evidence",
            "Usable and Showcase kit claims require contact-sheet evidence.",
        );
    }
    if kit.quality_tier == FoundryKitQualityTier::Showcase {
        if !review.human_approval_marker {
            report.push(
                "review_manifest.human_approval_marker",
                "showcase_missing_human_approval",
                "Showcase kits require human approval.",
            );
        }
        if !review.adversarial_review_marker {
            report.push(
                "review_manifest.adversarial_review_marker",
                "showcase_missing_adversarial_review",
                "Showcase kits require adversarial visual review.",
            );
        }
    }

    if matches!(
        kit.quality_tier,
        FoundryKitQualityTier::Draft | FoundryKitQualityTier::Prototype
    ) && kit.catalog_visibility_policy.default_novice_catalog
    {
        report.push(
            "kit.catalog_visibility_policy.default_novice_catalog",
            "draft_or_prototype_visible_by_default",
            "Draft and Prototype kits must stay hidden from the default novice catalog.",
        );
    }
    if kit.quality_tier == FoundryKitQualityTier::Usable
        && kit.catalog_visibility_policy.default_novice_catalog
        && !usable_review_evidence_passes(review)
    {
        report.push(
            "kit.catalog_visibility_policy.default_novice_catalog",
            "usable_visible_without_review_evidence",
            "Usable kits can be novice-visible only after required review evidence passes.",
        );
    }
    if kit.quality_tier == FoundryKitQualityTier::Showcase
        && (kit.catalog_visibility_policy.default_novice_catalog
            || kit.catalog_visibility_policy.showcase_badge_allowed)
        && !showcase_review_evidence_passes(review)
    {
        report.push(
            "kit.catalog_visibility_policy.showcase_badge_allowed",
            "showcase_visible_without_review_evidence",
            "Showcase visibility and badges require human approval and adversarial review.",
        );
    }
}

fn validate_catalog_manifest(package: &FoundryKitPackage, report: &mut FoundryKitValidationReport) {
    let kit_id = &package.kit.kit_id;
    let kit_ids = package
        .catalog_manifest
        .kit_ids
        .iter()
        .map(String::as_str)
        .collect::<BTreeSet<_>>();
    if !package.catalog_manifest.kit_ids.contains(kit_id) {
        report.push(
            "catalog_manifest.kit_ids",
            "catalog_manifest_missing_kit",
            "Catalog manifest must include the package kit ID.",
        );
    }
    for visible_kit_id in &package.catalog_manifest.default_visible_kit_ids {
        if !kit_ids.contains(visible_kit_id.as_str()) {
            report.push(
                format!("catalog_manifest.default_visible_kit_ids.{visible_kit_id}"),
                "catalog_manifest_unknown_default_visible_kit",
                "Default-visible catalog IDs must be listed in kit_ids.",
            );
        }
    }
    for preview_kit_id in &package.catalog_manifest.developer_preview_kit_ids {
        if !kit_ids.contains(preview_kit_id.as_str()) {
            report.push(
                format!("catalog_manifest.developer_preview_kit_ids.{preview_kit_id}"),
                "catalog_manifest_unknown_developer_preview_kit",
                "Developer-preview catalog IDs must be listed in kit_ids.",
            );
        }
    }
    for (hidden_kit_id, reason) in &package.catalog_manifest.hidden_kit_reasons {
        if !kit_ids.contains(hidden_kit_id.as_str()) {
            report.push(
                format!("catalog_manifest.hidden_kit_reasons.{hidden_kit_id}"),
                "catalog_manifest_unknown_hidden_kit",
                "Hidden kit reasons must reference IDs listed in kit_ids.",
            );
        }
        if reason.trim().is_empty() {
            report.push(
                format!("catalog_manifest.hidden_kit_reasons.{hidden_kit_id}"),
                "catalog_manifest_empty_hidden_reason",
                "Hidden kit reasons must use plain-language non-empty text.",
            );
        }
    }
    if package
        .catalog_manifest
        .default_visible_kit_ids
        .contains(kit_id)
        != package.kit.catalog_visibility_policy.default_novice_catalog
    {
        report.push(
            "catalog_manifest.default_visible_kit_ids",
            "catalog_manifest_visibility_mismatch",
            "Catalog manifest default visibility must match the kit visibility policy.",
        );
    }
    if package
        .catalog_manifest
        .developer_preview_kit_ids
        .contains(kit_id)
        != package
            .kit
            .catalog_visibility_policy
            .developer_preview_catalog
    {
        report.push(
            "catalog_manifest.developer_preview_kit_ids",
            "catalog_manifest_developer_preview_mismatch",
            "Catalog manifest developer-preview visibility must match the kit visibility policy.",
        );
    }
    if !package.kit.catalog_visibility_policy.default_novice_catalog {
        if package
            .kit
            .catalog_visibility_policy
            .hidden_reason
            .as_deref()
            .map(str::trim)
            .unwrap_or_default()
            .is_empty()
        {
            report.push(
                "kit.catalog_visibility_policy.hidden_reason",
                "kit_visibility_missing_hidden_reason",
                "Hidden kits must carry a plain-language reason.",
            );
        }
        if package
            .catalog_manifest
            .hidden_kit_reasons
            .get(kit_id)
            .map(String::as_str)
            .map(str::trim)
            .unwrap_or_default()
            .is_empty()
        {
            report.push(
                "catalog_manifest.hidden_kit_reasons",
                "catalog_manifest_missing_hidden_reason",
                "Catalog manifest must explain why hidden kits are not default-visible.",
            );
        }
    }
}

fn usable_review_evidence_passes(review: &KitReviewManifest) -> bool {
    review.tier_achieved >= FoundryKitQualityTier::Usable
        && review.human_approval_marker
        && !review.contact_sheet_paths.is_empty()
        && !review.benchmark_refs.is_empty()
        && review.blocked_reasons.is_empty()
}

fn showcase_review_evidence_passes(review: &KitReviewManifest) -> bool {
    usable_review_evidence_passes(review) && review.adversarial_review_marker
}

fn provider_pack_tags(provider_pack: &ProviderPack) -> BTreeSet<&str> {
    let mut tags = BTreeSet::new();
    tags.extend(provider_pack.compatibility_tags.iter().map(String::as_str));
    tags.extend(
        provider_pack
            .socket_attachment_tags
            .iter()
            .map(String::as_str),
    );
    tags.extend(provider_pack.detail_density_tags.iter().map(String::as_str));
    for option in &provider_pack.provider_options {
        tags.extend(option.compatibility_tags.iter().map(String::as_str));
        tags.extend(option.detail_density_tags.iter().map(String::as_str));
    }
    tags
}
