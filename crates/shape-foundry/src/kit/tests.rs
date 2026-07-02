
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn foundry_kit_schema_roundtrips() {
        let package = valid_package(FoundryKitQualityTier::Prototype);
        let json = serde_json::to_string_pretty(&package).expect("serialize kit package");
        let decoded: FoundryKitPackage =
            serde_json::from_str(&json).expect("deserialize kit package");
        assert_eq!(decoded, package);
        assert!(validate_foundry_kit_package(&decoded).is_valid());
    }

    #[test]
    fn provider_pack_schema_roundtrips() {
        let package = valid_package(FoundryKitQualityTier::Prototype);
        let json = serde_json::to_string(&package.provider_pack).expect("serialize provider pack");
        let decoded: ProviderPack = serde_json::from_str(&json).expect("deserialize provider pack");
        assert_eq!(decoded, package.provider_pack);
    }

    #[test]
    fn style_pack_compatibility_rejects_incompatible_provider() {
        let mut package = valid_package(FoundryKitQualityTier::Prototype);
        package
            .style_pack
            .incompatible_provider_packs
            .push(package.provider_pack.pack_id.clone());
        let report = validate_foundry_kit_package(&package);
        assert!(has_issue(&report, "style_provider_pack_incompatible"));
    }

    #[test]
    fn incompatible_provider_style_pair_is_not_novice_visible() {
        let mut package = reviewed_usable_package();
        package
            .compatibility_matrix
            .incompatible_style_provider_pairs
            .push(StyleProviderIncompatibility {
                style_id: package.style_pack.style_id.clone(),
                provider_pack_id: package.provider_pack.pack_id.clone(),
                hidden_reason: "Style and kit do not produce coherent output.".to_owned(),
            });
        let report = validate_foundry_kit_package(&package);
        assert!(has_issue(&report, "style_provider_pair_incompatible"));
    }

    #[test]
    fn draft_and_prototype_are_hidden_by_default() {
        let draft = valid_package(FoundryKitQualityTier::Draft);
        let prototype = valid_package(FoundryKitQualityTier::Prototype);
        assert!(
            !foundry_kit_visibility_decision(&draft.kit, &draft.review_manifest, false).visible
        );
        assert!(
            !foundry_kit_visibility_decision(&prototype.kit, &prototype.review_manifest, false)
                .visible
        );
        assert!(
            foundry_kit_visibility_decision(&prototype.kit, &prototype.review_manifest, true)
                .visible
        );
    }

    #[test]
    fn usable_visibility_requires_review_evidence() {
        let mut package = reviewed_usable_package();
        assert!(validate_foundry_kit_package(&package).is_valid());
        assert!(
            foundry_kit_visibility_decision(&package.kit, &package.review_manifest, false).visible
        );
        package.review_manifest.human_approval_marker = false;
        let report = validate_foundry_kit_package(&package);
        assert!(has_issue(&report, "usable_visible_without_review_evidence"));
        assert!(
            !foundry_kit_visibility_decision(&package.kit, &package.review_manifest, false).visible
        );
    }

    #[test]
    fn showcase_visibility_requires_human_and_adversarial_review() {
        let mut package = reviewed_usable_package();
        package.kit.quality_tier = FoundryKitQualityTier::Showcase;
        package.kit.catalog_visibility_policy.showcase_badge_allowed = true;
        package.review_manifest.tier_requested = FoundryKitQualityTier::Showcase;
        package.review_manifest.tier_achieved = FoundryKitQualityTier::Showcase;
        let report = validate_foundry_kit_package(&package);
        assert!(has_issue(&report, "showcase_missing_adversarial_review"));
        package.review_manifest.adversarial_review_marker = true;
        assert!(validate_foundry_kit_package(&package).is_valid());
        let decision =
            foundry_kit_visibility_decision(&package.kit, &package.review_manifest, false);
        assert!(decision.visible);
        assert!(decision.showcase_badge_allowed);
    }

    #[test]
    fn primary_controls_are_limited_to_seven_by_default() {
        let mut package = valid_package(FoundryKitQualityTier::Prototype);
        for index in 1..=8 {
            package.control_profile.controls.push(control(
                &format!("control-{index}"),
                &format!("slot-{index}"),
            ));
        }
        let report = validate_foundry_kit_package(&package);
        assert!(has_issue(&report, "too_many_primary_controls"));
    }

    #[test]
    fn duplicate_visible_control_ownership_is_rejected() {
        let mut package = valid_package(FoundryKitQualityTier::Prototype);
        package.control_profile.controls =
            vec![control("width-a", "width"), control("width-b", "width")];
        let report = validate_foundry_kit_package(&package);
        assert!(has_issue(&report, "duplicate_visible_control_ownership"));
    }

    #[test]
    fn missing_required_provider_slot_is_rejected() {
        let mut package = valid_package(FoundryKitQualityTier::Prototype);
        package.provider_pack.provider_slots_supplied.clear();
        let report = validate_foundry_kit_package(&package);
        assert!(has_issue(&report, "missing_required_provider_slot"));
    }

    #[test]
    fn forbidden_provider_tags_are_rejected() {
        let mut package = valid_package(FoundryKitQualityTier::Prototype);
        package
            .style_pack
            .forbidden_provider_tags
            .push("plain".to_owned());
        let report = validate_foundry_kit_package(&package);
        assert!(has_issue(&report, "style_forbidden_provider_tag"));
    }

    #[test]
    fn allowed_provider_tags_must_overlap_when_declared() {
        let mut package = valid_package(FoundryKitQualityTier::Prototype);
        package.style_pack.allowed_provider_tags = vec!["metal".to_owned()];
        let report = validate_foundry_kit_package(&package);
        assert!(has_issue(&report, "style_missing_allowed_provider_tag"));
    }

    #[test]
    fn provider_options_must_reference_known_slots_and_roles() {
        let mut package = valid_package(FoundryKitQualityTier::Prototype);
        package.provider_pack.provider_options[0].slot_id = "missing-slot".to_owned();
        package.provider_pack.provider_options[0].semantic_roles = vec!["missing-role".to_owned()];
        let report = validate_foundry_kit_package(&package);
        assert!(has_issue(&report, "provider_option_unknown_slot"));
        assert!(has_issue(&report, "provider_option_unsupplied_slot"));
        assert!(has_issue(&report, "provider_option_unknown_role"));
    }

    #[test]
    fn provider_options_must_cover_their_slot_role() {
        let mut package = valid_package(FoundryKitQualityTier::Prototype);
        package.provider_pack.provider_options[0].semantic_roles = vec!["detail".to_owned()];
        let report = validate_foundry_kit_package(&package);
        assert!(has_issue(&report, "provider_option_missing_slot_role"));
    }

    #[test]
    fn candidate_strategy_controls_must_exist() {
        let mut package = valid_package(FoundryKitQualityTier::Prototype);
        package
            .candidate_strategy_pack
            .allowed_controls
            .push("missing-control".to_owned());
        package.candidate_strategy_pack.strategies[0]
            .allowed_controls
            .push("missing-control".to_owned());
        let report = validate_foundry_kit_package(&package);
        assert!(has_issue(&report, "candidate_strategy_unknown_control"));
    }

    #[test]
    fn catalog_manifest_matches_visibility_policy_and_known_ids() {
        let mut package = valid_package(FoundryKitQualityTier::Prototype);
        package
            .catalog_manifest
            .default_visible_kit_ids
            .push(package.kit.kit_id.clone());
        package
            .catalog_manifest
            .default_visible_kit_ids
            .push("missing-kit".to_owned());
        package.catalog_manifest.developer_preview_kit_ids.clear();
        package
            .catalog_manifest
            .hidden_kit_reasons
            .insert("missing-kit".to_owned(), String::new());
        let report = validate_foundry_kit_package(&package);
        assert!(has_issue(
            &report,
            "catalog_manifest_unknown_default_visible_kit"
        ));
        assert!(has_issue(&report, "catalog_manifest_visibility_mismatch"));
        assert!(has_issue(
            &report,
            "catalog_manifest_developer_preview_mismatch"
        ));
        assert!(has_issue(&report, "catalog_manifest_unknown_hidden_kit"));
        assert!(has_issue(&report, "catalog_manifest_empty_hidden_reason"));
    }

    fn reviewed_usable_package() -> FoundryKitPackage {
        let mut package = valid_package(FoundryKitQualityTier::Usable);
        package.kit.catalog_visibility_policy = CatalogVisibilityPolicy::novice_visible();
        package.review_manifest.tier_requested = FoundryKitQualityTier::Usable;
        package.review_manifest.tier_achieved = FoundryKitQualityTier::Usable;
        package.review_manifest.human_approval_marker = true;
        package.review_manifest.contact_sheet_paths =
            vec!["target/hq/contact-sheet.png".to_owned()];
        package.review_manifest.benchmark_refs = vec!["target/hq/quality-report.json".to_owned()];
        package.catalog_manifest.default_visible_kit_ids = vec![package.kit.kit_id.clone()];
        package.catalog_manifest.hidden_kit_reasons.clear();
        package
    }

    fn valid_package(tier: FoundryKitQualityTier) -> FoundryKitPackage {
        let visibility = match tier {
            FoundryKitQualityTier::Draft => CatalogVisibilityPolicy {
                default_novice_catalog: false,
                developer_preview_catalog: false,
                showcase_badge_allowed: false,
                hidden_reason: Some("Draft content is hidden.".to_owned()),
            },
            FoundryKitQualityTier::Prototype => {
                CatalogVisibilityPolicy::hidden("Prototype content requires preview catalog mode.")
            }
            FoundryKitQualityTier::Usable | FoundryKitQualityTier::Showcase => {
                CatalogVisibilityPolicy::hidden("Review evidence is pending.")
            }
        };
        let review_tier = if tier >= FoundryKitQualityTier::Usable {
            FoundryKitQualityTier::Usable
        } else {
            tier
        };
        FoundryKitPackage {
            schema_version: FOUNDRY_KIT_PACKAGE_SCHEMA_VERSION,
            kit: FoundryKit {
                schema_version: FOUNDRY_KIT_SCHEMA_VERSION,
                kit_id: "box-kit".to_owned(),
                display_name: "Box Kit".to_owned(),
                family_blueprint_id: "box".to_owned(),
                provider_pack_id: "box-providers".to_owned(),
                style_pack_id: "plain-style".to_owned(),
                control_profile_id: "box-controls".to_owned(),
                candidate_strategy_pack_id: "box-strategies".to_owned(),
                quality_gate_profile_id: "usable-gates".to_owned(),
                compatibility_matrix_id: "box-compatibility".to_owned(),
                review_manifest_id: "box-review".to_owned(),
                catalog_manifest_id: "box-catalog".to_owned(),
                preview_camera_policy: PreviewCameraPolicy {
                    policy_id: "clay-review".to_owned(),
                    required_views: vec!["front".to_owned(), "three-quarter".to_owned()],
                    clay_preview_required: true,
                    contact_sheet_required: tier >= FoundryKitQualityTier::Usable,
                },
                quality_tier: tier,
                catalog_visibility_policy: visibility,
                source_profile_slug: Some("box-primitive".to_owned()),
                category_chips: vec!["Box".to_owned()],
            },
            family_blueprint: FamilyBlueprint {
                schema_version: FAMILY_BLUEPRINT_SCHEMA_VERSION,
                family_id: "box".to_owned(),
                display_name: "Box".to_owned(),
                semantic_roles: vec![
                    FamilyBlueprintRole {
                        role_id: "body".to_owned(),
                        label: "Body".to_owned(),
                        required: true,
                        tags: vec!["structure".to_owned()],
                    },
                    FamilyBlueprintRole {
                        role_id: "detail".to_owned(),
                        label: "Detail".to_owned(),
                        required: false,
                        tags: vec!["detail".to_owned()],
                    },
                ],
                required_roles: vec!["body".to_owned()],
                optional_roles: vec!["detail".to_owned()],
                provider_slots: vec![ProviderSlotExpectation {
                    slot_id: "body-slot".to_owned(),
                    role_id: "body".to_owned(),
                    required: true,
                    attachment_tags: vec!["support".to_owned()],
                }],
                attachment_expectations: Vec::new(),
                scale_policy: HighLevelScalePolicy {
                    label: "Prop scale".to_owned(),
                    allowed_range: Some("normalized".to_owned()),
                },
                export_part_naming_policy: ExportPartNamingPolicy {
                    strategy: "role-labels".to_owned(),
                    required_part_names: vec!["body".to_owned()],
                },
            },
            provider_pack: ProviderPack {
                schema_version: PROVIDER_PACK_SCHEMA_VERSION,
                pack_id: "box-providers".to_owned(),
                family_id: Some("box".to_owned()),
                compatible_family_ids: vec!["box".to_owned()],
                provider_slots_supplied: vec!["body-slot".to_owned()],
                provider_options: vec![ProviderPackOption {
                    option_id: "body-basic".to_owned(),
                    slot_id: "body-slot".to_owned(),
                    label: "Straight Body".to_owned(),
                    semantic_roles: vec!["body".to_owned()],
                    compatibility_tags: vec!["plain".to_owned()],
                    detail_density_tags: vec!["medium".to_owned()],
                    triangle_budget_estimate: Some(1200),
                }],
                semantic_role_coverage: vec!["body".to_owned()],
                socket_attachment_tags: vec!["support".to_owned()],
                detail_density_tags: vec!["medium".to_owned()],
                triangle_budget_estimates: BTreeMap::from([("body-basic".to_owned(), 1200)]),
                compatibility_tags: vec!["plain".to_owned()],
            },
            style_pack: StylePack {
                schema_version: STYLE_PACK_SCHEMA_VERSION,
                style_id: "plain-style".to_owned(),
                display_name: "Plain".to_owned(),
                compatible_family_ids: vec!["box".to_owned()],
                bevel_language: "soft structural edges".to_owned(),
                proportion_language: "sturdy broad spans".to_owned(),
                detail_density_policy: "medium detail".to_owned(),
                silhouette_exaggeration_policy: "readable from three-quarter view".to_owned(),
                symmetry_asymmetry_policy: "mostly symmetric with optional wear".to_owned(),
                allowed_provider_tags: vec!["plain".to_owned()],
                forbidden_provider_tags: vec!["other-style".to_owned()],
                compatible_provider_packs: vec!["box-providers".to_owned()],
                incompatible_provider_packs: Vec::new(),
                future_material_vocabulary: Some(FutureMaterialVocabulary {
                    label: "Future material notes".to_owned(),
                    tags: vec!["clay".to_owned()],
                }),
            },
            control_profile: ControlProfile {
                schema_version: CONTROL_PROFILE_SCHEMA_VERSION,
                profile_id: "box-controls".to_owned(),
                family_id: "box".to_owned(),
                style_id: Some("plain-style".to_owned()),
                maximum_primary_controls: DEFAULT_MAX_PRIMARY_NOVICE_CONTROLS,
                controls: vec![control("body-width", "width")],
            },
            candidate_strategy_pack: CandidateStrategyPack {
                schema_version: CANDIDATE_STRATEGY_PACK_SCHEMA_VERSION,
                pack_id: "box-strategies".to_owned(),
                strategies: vec![KitCandidateStrategy {
                    strategy_id: "refine".to_owned(),
                    name: "Refine".to_owned(),
                    allowed_controls: vec!["body-width".to_owned()],
                    explanation_templates: vec!["Adjusted whole-model proportions.".to_owned()],
                }],
                allowed_controls: vec!["body-width".to_owned()],
                allowed_provider_choices: BTreeMap::new(),
                diversity_goals: vec!["six coherent options".to_owned()],
                invalid_state_rejection_policy: "Reject invalid compile states.".to_owned(),
                lock_respect_policy: "Preserve locked controls.".to_owned(),
            },
            quality_gate_profile: QualityGateProfile {
                schema_version: QUALITY_GATE_PROFILE_SCHEMA_VERSION,
                profile_id: "usable-gates".to_owned(),
                required_tier: review_tier,
                mesh_gates: vec!["valid mesh".to_owned()],
                candidate_gates: vec!["six candidates".to_owned()],
                contact_sheet_gates: vec!["clay contact sheet".to_owned()],
                export_gates: vec!["package reopen".to_owned()],
                manual_review_gates: vec!["human review".to_owned()],
            },
            compatibility_matrix: KitCompatibilityMatrix {
                schema_version: KIT_COMPATIBILITY_MATRIX_SCHEMA_VERSION,
                matrix_id: "box-compatibility".to_owned(),
                compatible_style_provider_pairs: vec![StyleProviderCompatibility {
                    style_id: "plain-style".to_owned(),
                    provider_pack_id: "box-providers".to_owned(),
                    reason: "Coherent plain box style.".to_owned(),
                }],
                incompatible_style_provider_pairs: Vec::new(),
            },
            review_manifest: KitReviewManifest {
                schema_version: KIT_REVIEW_MANIFEST_SCHEMA_VERSION,
                manifest_id: "box-review".to_owned(),
                tier_requested: tier,
                tier_achieved: review_tier,
                reviewer: None,
                human_approval_marker: false,
                adversarial_review_marker: false,
                visual_review_notes: vec!["Review pending.".to_owned()],
                contact_sheet_paths: if tier >= FoundryKitQualityTier::Usable {
                    vec!["target/hq/contact-sheet.png".to_owned()]
                } else {
                    Vec::new()
                },
                benchmark_refs: Vec::new(),
                known_limitations: Vec::new(),
                blocked_reasons: Vec::new(),
            },
            catalog_manifest: KitCatalogManifest {
                schema_version: KIT_CATALOG_MANIFEST_SCHEMA_VERSION,
                catalog_id: "box-catalog".to_owned(),
                kit_ids: vec!["box-kit".to_owned()],
                default_visible_kit_ids: Vec::new(),
                developer_preview_kit_ids: if tier == FoundryKitQualityTier::Draft {
                    Vec::new()
                } else {
                    vec!["box-kit".to_owned()]
                },
                hidden_kit_reasons: BTreeMap::from([(
                    "box-kit".to_owned(),
                    "Review pending.".to_owned(),
                )]),
            },
        }
    }

    fn control(control_id: &str, slot: &str) -> ControlProfileControl {
        ControlProfileControl {
            control_id: control_id.to_owned(),
            label: control_id.replace('-', " "),
            description: "Changes the whole-model result.".to_owned(),
            kind: ControlProfileControlKind::Continuous,
            owned_family_slots: vec![slot.to_owned()],
            owned_provider_slots: Vec::new(),
            visible_effect_expectation: "Visible whole-model difference.".to_owned(),
            topology_behavior: ControlProfileTopologyBehavior::TopologyPreserving,
            option_visibility: ControlOptionVisibility {
                hide_invalid_from_novices: true,
                show_plain_language_reasons: true,
            },
            default_locked: false,
            primary: true,
            visible: true,
        }
    }

    fn has_issue(report: &FoundryKitValidationReport, code: &str) -> bool {
        report.issues.iter().any(|issue| issue.code == code)
    }
}
