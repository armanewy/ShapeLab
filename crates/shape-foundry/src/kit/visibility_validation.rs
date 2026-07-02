
/// Validate a complete Foundry kit package.
#[must_use]
pub fn validate_foundry_kit_package(package: &FoundryKitPackage) -> FoundryKitValidationReport {
    let mut report = FoundryKitValidationReport::default();
    validate_schema_versions(package, &mut report);
    validate_cross_references(package, &mut report);
    validate_family_provider_style_compatibility(package, &mut report);
    validate_control_profile(package, &mut report);
    validate_quality_and_visibility(package, &mut report);
    validate_catalog_manifest(package, &mut report);
    report
}

/// Return a catalog visibility decision for one kit and review manifest.
#[must_use]
pub fn foundry_kit_visibility_decision(
    kit: &FoundryKit,
    review: &KitReviewManifest,
    developer_preview_enabled: bool,
) -> FoundryKitVisibilityDecision {
    let hidden = |reason: String| FoundryKitVisibilityDecision {
        visible: false,
        showcase_badge_allowed: false,
        reason: Some(reason),
    };

    match kit.quality_tier {
        FoundryKitQualityTier::Draft => {
            return hidden("Draft kits stay hidden from the default catalog.".to_owned());
        }
        FoundryKitQualityTier::Prototype if !developer_preview_enabled => {
            return hidden("Prototype kits require preview catalog mode.".to_owned());
        }
        FoundryKitQualityTier::Prototype => {
            return FoundryKitVisibilityDecision {
                visible: kit.catalog_visibility_policy.developer_preview_catalog,
                showcase_badge_allowed: false,
                reason: kit
                    .catalog_visibility_policy
                    .hidden_reason
                    .clone()
                    .filter(|_| !kit.catalog_visibility_policy.developer_preview_catalog),
            };
        }
        FoundryKitQualityTier::Usable => {
            if !kit.catalog_visibility_policy.default_novice_catalog {
                return hidden(
                    kit.catalog_visibility_policy
                        .hidden_reason
                        .clone()
                        .unwrap_or_else(|| {
                            "This kit is not enabled for the default catalog.".to_owned()
                        }),
                );
            }
            if !usable_review_evidence_passes(review) {
                return hidden("Usable kits require completed review evidence.".to_owned());
            }
        }
        FoundryKitQualityTier::Showcase => {
            if !kit.catalog_visibility_policy.default_novice_catalog
                && !kit.catalog_visibility_policy.showcase_badge_allowed
            {
                return hidden(
                    kit.catalog_visibility_policy
                        .hidden_reason
                        .clone()
                        .unwrap_or_else(|| {
                            "This kit is not enabled for the default catalog.".to_owned()
                        }),
                );
            }
            if !showcase_review_evidence_passes(review) {
                return hidden(
                    "Showcase kits require human approval and adversarial review.".to_owned(),
                );
            }
        }
    }

    if kit.catalog_visibility_policy.default_novice_catalog {
        FoundryKitVisibilityDecision {
            visible: true,
            showcase_badge_allowed: kit.quality_tier == FoundryKitQualityTier::Showcase
                && kit.catalog_visibility_policy.showcase_badge_allowed
                && showcase_review_evidence_passes(review),
            reason: None,
        }
    } else {
        hidden(
            kit.catalog_visibility_policy
                .hidden_reason
                .clone()
                .unwrap_or_else(|| "This kit is not enabled for the default catalog.".to_owned()),
        )
    }
}

fn validate_schema_versions(package: &FoundryKitPackage, report: &mut FoundryKitValidationReport) {
    let versions = [
        (
            package.schema_version,
            FOUNDRY_KIT_PACKAGE_SCHEMA_VERSION,
            "schema_version",
            "unsupported_foundry_kit_package_schema",
        ),
        (
            package.kit.schema_version,
            FOUNDRY_KIT_SCHEMA_VERSION,
            "kit.schema_version",
            "unsupported_foundry_kit_schema",
        ),
        (
            package.family_blueprint.schema_version,
            FAMILY_BLUEPRINT_SCHEMA_VERSION,
            "family_blueprint.schema_version",
            "unsupported_family_blueprint_schema",
        ),
        (
            package.provider_pack.schema_version,
            PROVIDER_PACK_SCHEMA_VERSION,
            "provider_pack.schema_version",
            "unsupported_provider_pack_schema",
        ),
        (
            package.style_pack.schema_version,
            STYLE_PACK_SCHEMA_VERSION,
            "style_pack.schema_version",
            "unsupported_style_pack_schema",
        ),
        (
            package.control_profile.schema_version,
            CONTROL_PROFILE_SCHEMA_VERSION,
            "control_profile.schema_version",
            "unsupported_control_profile_schema",
        ),
        (
            package.candidate_strategy_pack.schema_version,
            CANDIDATE_STRATEGY_PACK_SCHEMA_VERSION,
            "candidate_strategy_pack.schema_version",
            "unsupported_candidate_strategy_pack_schema",
        ),
        (
            package.quality_gate_profile.schema_version,
            QUALITY_GATE_PROFILE_SCHEMA_VERSION,
            "quality_gate_profile.schema_version",
            "unsupported_quality_gate_profile_schema",
        ),
        (
            package.compatibility_matrix.schema_version,
            KIT_COMPATIBILITY_MATRIX_SCHEMA_VERSION,
            "compatibility_matrix.schema_version",
            "unsupported_compatibility_matrix_schema",
        ),
        (
            package.review_manifest.schema_version,
            KIT_REVIEW_MANIFEST_SCHEMA_VERSION,
            "review_manifest.schema_version",
            "unsupported_review_manifest_schema",
        ),
        (
            package.catalog_manifest.schema_version,
            KIT_CATALOG_MANIFEST_SCHEMA_VERSION,
            "catalog_manifest.schema_version",
            "unsupported_kit_catalog_manifest_schema",
        ),
    ];
    for (actual, expected, subject, code) in versions {
        if actual != expected {
            report.push(
                subject,
                code,
                format!("Expected schema version {expected}, found {actual}."),
            );
        }
    }
}

fn validate_cross_references(package: &FoundryKitPackage, report: &mut FoundryKitValidationReport) {
    validate_ref_eq(
        report,
        "kit.family_blueprint_id",
        &package.kit.family_blueprint_id,
        &package.family_blueprint.family_id,
        "missing_family_blueprint_ref",
        "Kit family blueprint reference must match the embedded family blueprint.",
    );
    validate_ref_eq(
        report,
        "kit.provider_pack_id",
        &package.kit.provider_pack_id,
        &package.provider_pack.pack_id,
        "missing_provider_pack_ref",
        "Kit provider pack reference must match the embedded provider pack.",
    );
    validate_ref_eq(
        report,
        "kit.style_pack_id",
        &package.kit.style_pack_id,
        &package.style_pack.style_id,
        "missing_style_pack_ref",
        "Kit style pack reference must match the embedded style pack.",
    );
    validate_ref_eq(
        report,
        "kit.control_profile_id",
        &package.kit.control_profile_id,
        &package.control_profile.profile_id,
        "missing_control_profile_ref",
        "Kit control profile reference must match the embedded control profile.",
    );
    validate_ref_eq(
        report,
        "kit.candidate_strategy_pack_id",
        &package.kit.candidate_strategy_pack_id,
        &package.candidate_strategy_pack.pack_id,
        "missing_candidate_strategy_pack_ref",
        "Kit candidate strategy reference must match the embedded strategy pack.",
    );
    validate_ref_eq(
        report,
        "kit.quality_gate_profile_id",
        &package.kit.quality_gate_profile_id,
        &package.quality_gate_profile.profile_id,
        "missing_quality_gate_profile_ref",
        "Kit quality gate reference must match the embedded quality gate profile.",
    );
    validate_ref_eq(
        report,
        "kit.compatibility_matrix_id",
        &package.kit.compatibility_matrix_id,
        &package.compatibility_matrix.matrix_id,
        "missing_compatibility_matrix_ref",
        "Kit compatibility matrix reference must match the embedded matrix.",
    );
    validate_ref_eq(
        report,
        "kit.review_manifest_id",
        &package.kit.review_manifest_id,
        &package.review_manifest.manifest_id,
        "missing_review_manifest_ref",
        "Kit review manifest reference must match the embedded review manifest.",
    );
    validate_ref_eq(
        report,
        "kit.catalog_manifest_id",
        &package.kit.catalog_manifest_id,
        &package.catalog_manifest.catalog_id,
        "missing_catalog_manifest_ref",
        "Kit catalog manifest reference must match the embedded catalog manifest.",
    );
}

fn validate_ref_eq(
    report: &mut FoundryKitValidationReport,
    subject: &str,
    actual: &str,
    expected: &str,
    code: &str,
    message: &str,
) {
    if actual != expected {
        report.push(subject, code, message);
    }
}

fn validate_family_provider_style_compatibility(
    package: &FoundryKitPackage,
    report: &mut FoundryKitValidationReport,
) {
    let family_id = &package.family_blueprint.family_id;
    if package.control_profile.family_id != *family_id {
        report.push(
            "control_profile.family_id",
            "control_profile_family_mismatch",
            "Control profile must target the kit family blueprint.",
        );
    }
    if !package.provider_pack.supports_family(family_id) {
        report.push(
            "provider_pack.compatible_family_ids",
            "provider_pack_family_incompatible",
            "Provider pack must support the kit family.",
        );
    }
    if !package.style_pack.supports_family(family_id) {
        report.push(
            "style_pack.compatible_family_ids",
            "style_pack_family_incompatible",
            "Style pack must support the kit family.",
        );
    }
    if !package
        .style_pack
        .allows_provider_pack(&package.provider_pack.pack_id)
    {
        report.push(
            "style_pack.compatible_provider_packs",
            "style_provider_pack_incompatible",
            "Style pack must allow the selected provider pack.",
        );
    }
    if let Some(incompatibility) = package.compatibility_matrix.style_provider_incompatibility(
        &package.style_pack.style_id,
        &package.provider_pack.pack_id,
    ) {
        report.push(
            "compatibility_matrix.incompatible_style_provider_pairs",
            "style_provider_pair_incompatible",
            format!(
                "Incompatible style/provider pair is hidden from novice catalog: {}",
                incompatibility.hidden_reason
            ),
        );
    }
    let provider_tags = provider_pack_tags(&package.provider_pack);
    if let Some(tag) = package
        .style_pack
        .forbidden_provider_tags
        .iter()
        .find(|tag| provider_tags.contains(tag.as_str()))
    {
        report.push(
            "style_pack.forbidden_provider_tags",
            "style_forbidden_provider_tag",
            format!("Provider pack carries forbidden style tag '{tag}'."),
        );
    }
    if !package.style_pack.allowed_provider_tags.is_empty()
        && !package
            .style_pack
            .allowed_provider_tags
            .iter()
            .any(|tag| provider_tags.contains(tag.as_str()))
    {
        report.push(
            "style_pack.allowed_provider_tags",
            "style_missing_allowed_provider_tag",
            "Provider pack must carry at least one tag allowed by the style pack.",
        );
    }

    let role_ids = package
        .family_blueprint
        .semantic_roles
        .iter()
        .map(|role| role.role_id.as_str())
        .collect::<BTreeSet<_>>();
    for required_role in &package.family_blueprint.required_roles {
        if !role_ids.contains(required_role.as_str()) {
            report.push(
                format!("family_blueprint.required_roles.{required_role}"),
                "unknown_required_role",
                "Required role must exist in the family role inventory.",
            );
        }
        if !package
            .provider_pack
            .semantic_role_coverage
            .iter()
            .any(|role| role == required_role)
        {
            report.push(
                format!("provider_pack.semantic_role_coverage.{required_role}"),
                "missing_required_role_coverage",
                "Provider pack must cover every required family role.",
            );
        }
    }
    for optional_role in &package.family_blueprint.optional_roles {
        if !role_ids.contains(optional_role.as_str()) {
            report.push(
                format!("family_blueprint.optional_roles.{optional_role}"),
                "unknown_optional_role",
                "Optional role must exist in the family role inventory.",
            );
        }
    }

    let supplied_slots = package
        .provider_pack
        .provider_slots_supplied
        .iter()
        .map(String::as_str)
        .collect::<BTreeSet<_>>();
    let declared_slots = package
        .family_blueprint
        .provider_slots
        .iter()
        .map(|slot| slot.slot_id.as_str())
        .collect::<BTreeSet<_>>();
    let slot_expected_roles = package
        .family_blueprint
        .provider_slots
        .iter()
        .map(|slot| (slot.slot_id.as_str(), slot.role_id.as_str()))
        .collect::<BTreeMap<_, _>>();
    for supplied_slot in &package.provider_pack.provider_slots_supplied {
        if !declared_slots.contains(supplied_slot.as_str()) {
            report.push(
                format!("provider_pack.provider_slots_supplied.{supplied_slot}"),
                "unknown_provider_slot_supplied",
                "Provider pack supplies a slot not declared by the family blueprint.",
            );
        }
    }
    for slot in &package.family_blueprint.provider_slots {
        if slot.required && !supplied_slots.contains(slot.slot_id.as_str()) {
            report.push(
                format!("provider_pack.provider_slots_supplied.{}", slot.slot_id),
                "missing_required_provider_slot",
                "Provider pack must supply every required family provider slot.",
            );
        }
    }
    let mut option_ids = BTreeSet::new();
    for (index, option) in package.provider_pack.provider_options.iter().enumerate() {
        if !option_ids.insert(option.option_id.as_str()) {
            report.push(
                format!("provider_pack.provider_options.{index}.option_id"),
                "duplicate_provider_option_id",
                "Provider option IDs must be unique within a provider pack.",
            );
        }
        if !declared_slots.contains(option.slot_id.as_str()) {
            report.push(
                format!("provider_pack.provider_options.{index}.slot_id"),
                "provider_option_unknown_slot",
                "Provider option slot must be declared by the family blueprint.",
            );
        }
        if !supplied_slots.contains(option.slot_id.as_str()) {
            report.push(
                format!("provider_pack.provider_options.{index}.slot_id"),
                "provider_option_unsupplied_slot",
                "Provider option slot must be included in provider_slots_supplied.",
            );
        }
        if let Some(expected_role) = slot_expected_roles.get(option.slot_id.as_str())
            && !option
                .semantic_roles
                .iter()
                .any(|role| role.as_str() == *expected_role)
        {
            report.push(
                format!("provider_pack.provider_options.{index}.semantic_roles"),
                "provider_option_missing_slot_role",
                "Provider option semantic roles must include the role expected by its slot.",
            );
        }
        for role in &option.semantic_roles {
            if !role_ids.contains(role.as_str()) {
                report.push(
                    format!("provider_pack.provider_options.{index}.semantic_roles.{role}"),
                    "provider_option_unknown_role",
                    "Provider option semantic role must exist in the family blueprint.",
                );
            }
        }
    }
    for covered_role in &package.provider_pack.semantic_role_coverage {
        if !role_ids.contains(covered_role.as_str()) {
            report.push(
                format!("provider_pack.semantic_role_coverage.{covered_role}"),
                "provider_pack_unknown_covered_role",
                "Provider pack role coverage must reference a known family role.",
            );
        }
        if !package.provider_pack.provider_options.iter().any(|option| {
            option
                .semantic_roles
                .iter()
                .any(|role| role == covered_role)
        }) {
            report.push(
                format!("provider_pack.semantic_role_coverage.{covered_role}"),
                "provider_role_coverage_not_backed_by_option",
                "Provider pack role coverage must be backed by at least one provider option.",
            );
        }
    }
}
