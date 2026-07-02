
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
