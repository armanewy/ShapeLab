
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
