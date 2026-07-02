
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
