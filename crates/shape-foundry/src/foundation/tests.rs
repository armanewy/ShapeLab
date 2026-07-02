
#[cfg(test)]
mod tests {
    use super::*;
    use crate::{foundry_kit_visibility_decision, validate_foundry_kit_package};

    #[test]
    fn foundation_draft_schema_roundtrips() {
        let draft = foundation_draft_template("boxes", "box_primitive");
        let json = serde_json::to_string(&draft).expect("serialize");
        let roundtrip: FoundryFoundationDraft = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(roundtrip, draft);
        assert_eq!(
            roundtrip.schema_version,
            FOUNDRY_FOUNDATION_DRAFT_SCHEMA_VERSION
        );
    }

    #[test]
    fn new_draft_defaults_to_internal_reviewed_unpublished() {
        let draft = foundation_draft_template("boxes", "box_primitive");
        assert_eq!(
            draft.catalog_visibility,
            FoundationCatalogVisibility::InternalOnly
        );
        assert!(draft.human_review_required);
        assert!(!draft.publish_allowed);
    }

    #[test]
    fn forbidden_commands_cannot_deserialize_or_execute() {
        for command in forbidden_foundation_command_names() {
            let json = format!(r#"{{"command":"{command}","args":{{}}}}"#);
            assert!(parse_foundation_authoring_command_json(&json).is_err());
        }
        let extra_args = r#"{
            "command": "CreateFamilyBlueprint",
            "args": {
                "family_id": "box_primitive",
                "display_name": "Box Primitive",
                "raw_vertex_positions": [[0, 0, 0]]
            }
        }"#;
        assert!(parse_foundation_authoring_command_json(extra_args).is_err());
        let mut draft = foundation_draft_template("boxes", "box_primitive");
        draft
            .rejected_command_attempts
            .push("InjectMeshPayload".to_owned());
        draft
            .direct_geometry_payload_attempts
            .push("vertices:[0,0,0]".to_owned());
        let report = validate_foundation_draft(&draft);
        let codes = issue_codes(&report);
        assert!(codes.contains("forbidden_command_attempt"));
        assert!(codes.contains("direct_geometry_payload_attempt"));
    }

    #[test]
    fn foundation_draft_rejects_unknown_geometry_fields() {
        let draft = foundation_draft_template("boxes", "box_primitive");
        let mut value = serde_json::to_value(&draft).expect("to value");
        let object = value.as_object_mut().expect("object");
        object.insert(
            "mesh_payload".to_owned(),
            serde_json::json!({"vertices": [[0, 0, 0]]}),
        );
        object.insert(
            "raw_vertex_positions".to_owned(),
            serde_json::json!([[0, 0, 0]]),
        );
        assert!(serde_json::from_value::<FoundryFoundationDraft>(value).is_err());
    }

    #[test]
    fn allowed_commands_execute_in_structured_space() {
        let mut draft = foundation_draft_template("boxes", "box_primitive");
        execute_foundation_authoring_command(
            &mut draft,
            FoundationAuthoringCommand::AddRole {
                role_id: "edge_detail".to_owned(),
                label: "Edge Detail".to_owned(),
                required: false,
            },
        )
        .expect("execute");
        assert!(
            draft
                .family_blueprint
                .roles
                .iter()
                .any(|role| role.role_id == "edge_detail")
        );
    }

    #[test]
    fn draft_validation_rejects_primary_control_overload_and_technical_labels() {
        let mut draft = foundation_draft_template("boxes", "box_primitive");
        draft.control_profile.controls = (0..8)
            .map(|index| DraftControl {
                control_id: format!("control_{index}"),
                label: if index == 0 {
                    "scalar::body.path".to_owned()
                } else {
                    format!("Control {index}")
                },
                description: "Visible change.".to_owned(),
                kind: ControlProfileControlKind::Choice,
                primary: true,
                visible: true,
                owned_family_slots: Vec::new(),
                owned_provider_slots: vec![format!("slot_{index}")],
                topology_behavior: ControlProfileTopologyBehavior::TopologyChanging,
                visible_effect_expectation: "Visible change.".to_owned(),
            })
            .collect();
        let report = validate_foundation_draft(&draft);
        let codes = issue_codes(&report);
        assert!(codes.contains("too_many_primary_controls"));
        assert!(codes.contains("technical_term_in_novice_label"));
    }

    #[test]
    fn draft_validation_rejects_missing_quality_gate_and_contact_sheet_gap() {
        let mut draft = foundation_draft_template("boxes", "box_primitive");
        draft.quality_gate_profile = None;
        assert!(issue_codes(&validate_foundation_draft(&draft)).contains("missing_quality_gate"));

        let mut usable = foundation_draft_template("boxes", "box_primitive");
        usable.quality_target = FoundationQualityTarget::Usable;
        usable
            .quality_gate_profile
            .as_mut()
            .expect("quality gate")
            .contact_sheet_required = false;
        assert!(
            issue_codes(&validate_foundation_draft(&usable))
                .contains("usable_or_showcase_requires_contact_sheet")
        );
    }

    #[test]
    fn draft_validation_rejects_slot_and_candidate_incoherence() {
        let mut draft = foundation_draft_template("boxes", "box_primitive");
        draft.control_profile.controls.push(DraftControl {
            control_id: "conflict".to_owned(),
            label: "Conflict".to_owned(),
            description: "Conflicting control.".to_owned(),
            kind: ControlProfileControlKind::Choice,
            primary: true,
            visible: true,
            owned_family_slots: Vec::new(),
            owned_provider_slots: draft.control_profile.controls[0]
                .owned_provider_slots
                .clone(),
            topology_behavior: ControlProfileTopologyBehavior::TopologyChanging,
            visible_effect_expectation: "Visible change.".to_owned(),
        });
        draft.candidate_strategy_pack.strategies[0]
            .allowed_controls
            .push("missing_control".to_owned());
        draft.candidate_strategy_pack.strategies[0]
            .allowed_provider_changes
            .push("missing_slot".to_owned());
        let report = validate_foundation_draft(&draft);
        let codes = issue_codes(&report);
        assert!(codes.contains("duplicate_slot_ownership"));
        assert!(codes.contains("candidate_strategy_not_control_space"));
        assert!(codes.contains("candidate_strategy_unknown_provider_slot"));
    }

    #[test]
    fn draft_validation_rejects_unknown_control_provider_slots() {
        let mut draft = foundation_draft_template("boxes", "box_primitive");
        draft.control_profile.controls[0]
            .owned_provider_slots
            .push("missing_slot".to_owned());
        let report = validate_foundation_draft(&draft);
        assert!(issue_codes(&report).contains("control_unknown_provider_slot"));
    }

    #[test]
    fn draft_validation_rejects_unknown_compatibility_references() {
        let mut draft = foundation_draft_template("boxes", "box_primitive");
        draft
            .compatibility_matrix
            .rules
            .push(DraftCompatibilityRule {
                style_id: "unknown_style".to_owned(),
                provider_pack_id: "unknown_provider_pack".to_owned(),
                compatible: true,
                reason: "Invalid row used for validation coverage.".to_owned(),
            });
        let report = validate_foundation_draft(&draft);
        let codes = issue_codes(&report);
        assert!(codes.contains("compatibility_unknown_style"));
        assert!(codes.contains("compatibility_unknown_provider_pack"));

        let mut explicit_review_style = foundation_draft_template("boxes", "box_primitive");
        explicit_review_style
            .style_pack
            .compatibility_style_ids
            .push("review_style".to_owned());
        explicit_review_style
            .compatibility_matrix
            .rules
            .push(DraftCompatibilityRule {
                style_id: "review_style".to_owned(),
                provider_pack_id: explicit_review_style.provider_taxonomy.provider_packs[0]
                    .pack_id
                    .clone(),
                compatible: true,
                reason: "Explicitly listed review style is allowed.".to_owned(),
            });
        assert!(validate_foundation_draft(&explicit_review_style).is_valid());
    }

    #[test]
    fn draft_validation_rejects_publish_and_visibility_overclaims() {
        let mut draft = foundation_draft_template("boxes", "box_primitive");
        draft.human_review_required = false;
        draft.publish_allowed = true;
        draft.catalog_visibility = FoundationCatalogVisibility::NoviceCatalog;
        let report = validate_foundation_draft(&draft);
        let codes = issue_codes(&report);
        assert!(codes.contains("foundation_publish_not_allowed"));
        assert!(codes.contains("foundation_draft_must_remain_internal"));
        assert!(codes.contains("publish_requires_human_review"));
        assert!(codes.contains("draft_or_prototype_cannot_be_novice_visible"));
    }

    #[test]
    fn usable_or_showcase_drafts_still_cannot_publish_or_leave_internal_catalog() {
        let mut draft = foundation_draft_template("boxes", "box_primitive");
        draft.quality_target = FoundationQualityTarget::Usable;
        draft.publish_allowed = true;
        draft.catalog_visibility = FoundationCatalogVisibility::NoviceCatalog;
        draft
            .quality_gate_profile
            .as_mut()
            .expect("quality gate")
            .contact_sheet_required = true;
        let report = validate_foundation_draft(&draft);
        let codes = issue_codes(&report);
        assert!(codes.contains("foundation_publish_not_allowed"));
        assert!(codes.contains("foundation_draft_must_remain_internal"));
    }

    #[test]
    fn materialized_kit_remains_draft_until_reviewed() {
        let draft = foundation_draft_template("boxes", "box_primitive");
        let package = materialize_foundation_draft_package(&draft).expect("materialize");
        assert_eq!(package.kit.quality_tier, FoundryKitQualityTier::Draft);
        assert!(!package.kit.catalog_visibility_policy.default_novice_catalog);
        assert!(
            !package
                .kit
                .catalog_visibility_policy
                .developer_preview_catalog
        );
        assert!(package.catalog_manifest.default_visible_kit_ids.is_empty());
        assert!(
            package
                .catalog_manifest
                .developer_preview_kit_ids
                .is_empty()
        );
        assert!(validate_foundry_kit_package(&package).is_valid());
        assert!(
            package
                .candidate_strategy_pack
                .allowed_provider_choices
                .contains_key("body_slot")
        );
        let decision =
            foundry_kit_visibility_decision(&package.kit, &package.review_manifest, false);
        assert!(!decision.visible);
    }

    #[test]
    fn materialization_maps_invalid_package_report_back_to_foundation_errors() {
        let mut draft = foundation_draft_template("boxes", "box_primitive");
        draft.compatibility_matrix.rules = vec![DraftCompatibilityRule {
            style_id: draft.style_pack.style_id.clone(),
            provider_pack_id: draft.provider_taxonomy.provider_packs[0].pack_id.clone(),
            compatible: false,
            reason: "Conflicts with this provider pack.".to_owned(),
        }];
        assert!(validate_foundation_draft(&draft).is_valid());
        let report = materialize_foundation_draft_package(&draft)
            .expect_err("materialization should reject invalid kit package");
        assert!(issue_codes(&report).contains("materialized_style_provider_pair_incompatible"));
    }

    #[test]
    fn archetype_materializer_box_primitive_draft_is_internal_only() {
        for (family_id, style_id) in [
            ("box-primitive", "plain-clay"),
            ("wide-box", "plain-clay"),
            ("soft-box", "plain-clay"),
        ] {
            let draft =
                materialize_archetype_foundation_draft("box-primitive", family_id, style_id)
                    .expect("box primitive archetype materializes");
            assert!(validate_foundation_draft(&draft).is_valid());
            assert!(!draft.publish_allowed);
            assert_eq!(
                draft.catalog_visibility,
                FoundationCatalogVisibility::InternalOnly
            );
            assert!(draft.human_review_required);
            assert!(draft.direct_geometry_payload_attempts.is_empty());
            assert_eq!(draft.control_profile.controls.len(), 2);
            assert_eq!(draft.family_blueprint.required_roles, vec!["body"]);
            assert!(draft.family_blueprint.optional_roles.is_empty());
            assert!(draft.family_blueprint.sockets.is_empty());
        }
        let draft =
            materialize_archetype_foundation_draft("box-primitive", "box-primitive", "plain-clay")
                .expect("box primitive archetype materializes");
        assert!(validate_foundation_draft(&draft).is_valid());
        assert_eq!(draft.family_blueprint.family_id, "box_primitive");
        assert_eq!(draft.style_pack.style_id, "plain_clay");
    }

    #[test]
    fn archetype_materializer_rejects_invalid_archetype_and_geometry_payloads() {
        assert!(
            materialize_archetype_foundation_draft(
                "unsupported-archetype",
                "box-primitive",
                "plain-clay",
            )
            .is_err()
        );
        let mut draft =
            materialize_archetype_foundation_draft("box-primitive", "box-primitive", "plain-clay")
                .expect("box primitive archetype materializes");
        draft
            .direct_geometry_payload_attempts
            .push("raw vertex payload".to_owned());
        let report = validate_foundation_draft(&draft);
        assert!(issue_codes(&report).contains("direct_geometry_payload_attempt"));
    }

    #[test]
    fn archetype_materializer_report_is_deterministic() {
        let first =
            materialize_archetype_foundation_draft("box-primitive", "box-primitive", "plain-clay")
                .expect("first draft");
        let second =
            materialize_archetype_foundation_draft("box-primitive", "box-primitive", "plain-clay")
                .expect("second draft");
        assert_eq!(first, second);
        let files = vec![
            "family-blueprint-draft.json".to_owned(),
            "materialization-report.json".to_owned(),
        ];
        let first_report = archetype_draft_materialization_report(&first, files.clone());
        let second_report = archetype_draft_materialization_report(&second, files);
        assert_eq!(first_report, second_report);
        assert!(!first_report.publish_allowed);
        assert!(!first_report.novice_visible);
        assert!(first_report.human_review_required);
        assert!(!first_report.missing_taste_bearing_providers.is_empty());
    }

    #[test]
    fn fixture_drafts_are_internal_draft_only() {
        let fixtures = foundation_draft_fixtures();
        assert_eq!(fixtures.len(), 1);
        for draft in fixtures {
            assert_eq!(draft.quality_target, FoundationQualityTarget::Draft);
            assert_eq!(
                draft.catalog_visibility,
                FoundationCatalogVisibility::InternalOnly
            );
            assert!(!draft.publish_allowed);
            assert!(validate_foundation_draft(&draft).is_valid());
        }
    }

    #[test]
    fn adversarial_report_is_deterministic_and_complete() {
        let draft = foundation_draft_template("boxes", "box_primitive");
        let first = foundation_adversarial_report(&draft);
        let second = foundation_adversarial_report(&draft);
        assert_eq!(first, second);
        assert_eq!(first.questions.len(), 11);
        assert!(
            first
                .questions
                .iter()
                .any(|question| question.question.contains("procedural filler"))
        );
    }

    fn issue_codes(report: &FoundationDraftValidationReport) -> BTreeSet<&str> {
        report
            .issues
            .iter()
            .map(|issue| issue.code.as_str())
            .collect()
    }
}
