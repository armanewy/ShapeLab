
#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        CANDIDATE_STRATEGY_PACK_SCHEMA_VERSION, CONTROL_PROFILE_SCHEMA_VERSION,
        CandidateStrategyPack, CatalogVisibilityPolicy, ControlOptionVisibility, ControlProfile,
        ExportPartNamingPolicy, FAMILY_BLUEPRINT_SCHEMA_VERSION,
        FOUNDRY_KIT_PACKAGE_SCHEMA_VERSION, FOUNDRY_KIT_SCHEMA_VERSION, FamilyBlueprint,
        FamilyBlueprintRole, FoundryKit, FoundryKitQualityTier, FutureMaterialVocabulary,
        HighLevelScalePolicy, KIT_CATALOG_MANIFEST_SCHEMA_VERSION,
        KIT_COMPATIBILITY_MATRIX_SCHEMA_VERSION, KIT_REVIEW_MANIFEST_SCHEMA_VERSION,
        KitCandidateStrategy, KitCatalogManifest, KitCompatibilityMatrix, KitReviewManifest,
        PROVIDER_PACK_SCHEMA_VERSION, PreviewCameraPolicy, ProviderPack, ProviderPackOption,
        ProviderSlotExpectation, QUALITY_GATE_PROFILE_SCHEMA_VERSION, QualityGateProfile,
        STYLE_PACK_SCHEMA_VERSION, StylePack,
    };

    #[test]
    fn author_studio_is_gated_from_default_release() {
        let shell = foundry_author_studio_shell(FoundryAuthorStudioGate::default_release());
        assert!(!shell.available);
        assert!(shell.steps.is_empty());
        assert!(
            shell
                .unavailable_reason
                .as_deref()
                .is_some_and(|reason| reason.contains("developer"))
        );
    }

    #[test]
    fn author_studio_developer_gate_exposes_workflow_steps() {
        let shell = foundry_author_studio_shell(FoundryAuthorStudioGate::developer_enabled());
        assert!(shell.available);
        assert_eq!(shell.steps.len(), 9);
        assert_eq!(shell.steps[0].label, "Kit Overview");
        assert_eq!(shell.steps[8].label, "Review & Package");
    }

    #[test]
    fn role_labeling_descriptors_validate_inventory() {
        let roles = sample_roles();
        assert!(validate_author_roles(&roles).is_valid());
        let mut duplicate = roles.clone();
        duplicate.push(roles[0].clone());
        assert!(
            validate_author_roles(&duplicate)
                .issues
                .iter()
                .any(|issue| issue.code == "duplicate_role_id")
        );
    }

    #[test]
    fn socket_port_descriptor_validation_catches_missing_and_dangling_refs() {
        let provider = ProviderDescriptor {
            provider_id: "provider_a".to_owned(),
            display_name: "Provider A".to_owned(),
            semantic_role: "missing".to_owned(),
            provider_slot: "body_slot".to_owned(),
            tags: vec!["clean".to_owned()],
            compatibility_tags: vec!["plain".to_owned()],
            approximate_triangle_budget: Some(512),
            preview_available: false,
            descriptor_only: true,
            socket_requirements: vec![
                SocketPortDescriptor {
                    socket_id: "attach".to_owned(),
                    port_id: String::new(),
                    target_role: "detail".to_owned(),
                    compatibility_tags: Vec::new(),
                    allowed_attachment_modes: vec!["snap".to_owned()],
                    required: true,
                    author_notes: "Required detail attachment.".to_owned(),
                },
                SocketPortDescriptor {
                    socket_id: "attach".to_owned(),
                    port_id: "other".to_owned(),
                    target_role: "missing".to_owned(),
                    compatibility_tags: vec!["plain".to_owned()],
                    allowed_attachment_modes: vec!["snap".to_owned()],
                    required: true,
                    author_notes: "Duplicate socket.".to_owned(),
                },
            ],
        };
        let report = validate_provider_descriptor(&provider, &sample_roles());
        let codes = issue_codes(&report);
        assert!(codes.contains("dangling_provider_role"));
        assert!(codes.contains("missing_required_socket_metadata"));
        assert!(codes.contains("missing_required_socket_compatibility_tags"));
        assert!(codes.contains("duplicate_socket_id"));
        assert!(codes.contains("dangling_socket_role"));
    }

    #[test]
    fn provider_descriptors_reject_unsupported_import_claims_and_sparse_sockets() {
        let provider = ProviderDescriptor {
            provider_id: "provider_a".to_owned(),
            display_name: "Provider A".to_owned(),
            semantic_role: "body".to_owned(),
            provider_slot: "body_slot".to_owned(),
            tags: vec!["clean".to_owned()],
            compatibility_tags: vec!["plain".to_owned()],
            approximate_triangle_budget: Some(512),
            preview_available: false,
            descriptor_only: false,
            socket_requirements: vec![SocketPortDescriptor {
                socket_id: "attach".to_owned(),
                port_id: "attach_port".to_owned(),
                target_role: "detail".to_owned(),
                compatibility_tags: vec!["plain".to_owned()],
                allowed_attachment_modes: Vec::new(),
                required: true,
                author_notes: String::new(),
            }],
        };
        let report = validate_provider_descriptor(&provider, &sample_roles());
        let codes = issue_codes(&report);
        assert!(codes.contains("provider_import_must_be_descriptor_only"));
        assert!(codes.contains("missing_required_socket_attachment_modes"));
        assert!(codes.contains("missing_required_socket_author_notes"));
    }

    #[test]
    fn style_compatibility_validation_rejects_conflicting_tags() {
        let descriptor = StyleCompatibilityDescriptor {
            compatible_style_packs: vec!["plain".to_owned()],
            incompatible_style_packs: BTreeMap::from([("plain".to_owned(), String::new())]),
            allowed_provider_tags: vec!["decorative".to_owned()],
            forbidden_provider_tags: vec!["decorative".to_owned()],
            detail_density_policy: "Readable at thumbnail size.".to_owned(),
            bevel_language_notes: "Broad bevels.".to_owned(),
            proportion_language_notes: "Plain proportions.".to_owned(),
            symmetry_asymmetry_policy: "Mostly symmetric.".to_owned(),
        };
        let report = validate_style_compatibility(&descriptor);
        let codes = issue_codes(&report);
        assert!(codes.contains("style_tag_both_allowed_and_forbidden"));
        assert!(codes.contains("style_pack_both_compatible_and_incompatible"));
        assert!(codes.contains("missing_incompatibility_reason"));
    }

    #[test]
    fn control_mapping_rejects_duplicate_ownership_and_bad_topology_controls() {
        let controls = vec![
            control(
                "shape",
                "Shape",
                "body_slot",
                ControlProfileControlKind::Continuous,
            ),
            ControlMappingDescriptor {
                control_id: "shape_alt".to_owned(),
                label: "Shape Alt".to_owned(),
                description: "Changes the body.".to_owned(),
                kind: ControlProfileControlKind::Continuous,
                primary: true,
                visible: true,
                owned_family_slots: Vec::new(),
                owned_provider_slots: vec!["body_slot".to_owned()],
                response_curve_descriptor: "linear".to_owned(),
                discrete_options: Vec::new(),
                provider_slot_binding: Some("body_slot".to_owned()),
                topology_behavior: ControlProfileTopologyBehavior::TopologyChanging,
                disabled_reason_policy: "Requires a body provider.".to_owned(),
            },
        ];
        let report = validate_control_mappings(&controls);
        let codes = issue_codes(&report);
        assert!(codes.contains("duplicate_visible_slot_ownership"));
        assert!(codes.contains("topology_changing_control_must_be_discrete"));
    }

    #[test]
    fn control_mapping_rejects_empty_choice_options_and_unowned_bindings() {
        let controls = vec![ControlMappingDescriptor {
            control_id: "body_shape".to_owned(),
            label: "Body Shape".to_owned(),
            description: "Changes the body.".to_owned(),
            kind: ControlProfileControlKind::Choice,
            primary: true,
            visible: true,
            owned_family_slots: Vec::new(),
            owned_provider_slots: vec!["body_slot".to_owned()],
            response_curve_descriptor: "discrete".to_owned(),
            discrete_options: Vec::new(),
            provider_slot_binding: Some("detail_slot".to_owned()),
            topology_behavior: ControlProfileTopologyBehavior::TopologyChanging,
            disabled_reason_policy: "Requires a body provider.".to_owned(),
        }];
        let report = validate_control_mappings(&controls);
        let codes = issue_codes(&report);
        assert!(codes.contains("choice_control_missing_options"));
        assert!(codes.contains("topology_changing_control_missing_options"));
        assert!(codes.contains("provider_slot_binding_not_owned"));
    }

    #[test]
    fn primary_controls_are_limited_to_seven_by_default() {
        let controls = (0..8)
            .map(|index| {
                control(
                    &format!("control_{index}"),
                    &format!("Control {index}"),
                    &format!("slot_{index}"),
                    ControlProfileControlKind::Choice,
                )
            })
            .collect::<Vec<_>>();
        assert!(
            validate_control_mappings(&controls)
                .issues
                .iter()
                .any(|issue| issue.code == "too_many_primary_controls")
        );
    }

    #[test]
    fn candidate_strategy_validation_requires_user_facing_labels() {
        let controls = vec![control(
            "armor_mass",
            "Armor Mass",
            "armor_slot",
            ControlProfileControlKind::Choice,
        )];
        let strategies = vec![CandidateStrategyDescriptor {
            strategy_id: "heavy".to_owned(),
            name: "Heavy".to_owned(),
            explanation: "Heavier silhouette.".to_owned(),
            allowed_controls: vec!["armor_mass".to_owned()],
            allowed_provider_changes: vec!["armor_slot".to_owned()],
            intensity_policy: "medium".to_owned(),
            diversity_policy: "avoid duplicates".to_owned(),
            lock_respect_policy: "Respect locked controls.".to_owned(),
            rejection_policy: "Reject invalid output.".to_owned(),
            explanation_template: "Changes scalar::armor.mass".to_owned(),
        }];
        let report = validate_candidate_strategy_descriptors(&strategies, &controls);
        let codes = issue_codes(&report);
        assert!(codes.contains("candidate_strategy_uses_raw_recipe_surface"));
        assert!(codes.contains("candidate_explanation_missing_user_facing_label"));

        let fixed = vec![CandidateStrategyDescriptor {
            explanation_template: "Armor Mass becomes heavier.".to_owned(),
            ..strategies[0].clone()
        }];
        assert!(validate_candidate_strategy_descriptors(&fixed, &controls).is_valid());
    }

    #[test]
    fn candidate_strategy_validation_rejects_unknown_provider_changes_and_empty_policies() {
        let controls = vec![control(
            "armor_mass",
            "Armor Mass",
            "armor_slot",
            ControlProfileControlKind::Choice,
        )];
        let strategies = vec![CandidateStrategyDescriptor {
            strategy_id: String::new(),
            name: "Heavy".to_owned(),
            explanation: "Heavier silhouette.".to_owned(),
            allowed_controls: vec!["armor_mass".to_owned()],
            allowed_provider_changes: vec!["missing_slot".to_owned()],
            intensity_policy: String::new(),
            diversity_policy: String::new(),
            lock_respect_policy: String::new(),
            rejection_policy: String::new(),
            explanation_template: String::new(),
        }];
        let report = validate_candidate_strategy_descriptors(&strategies, &controls);
        let codes = issue_codes(&report);
        assert!(codes.contains("missing_strategy_id"));
        assert!(codes.contains("missing_intensity_policy"));
        assert!(codes.contains("missing_diversity_policy"));
        assert!(codes.contains("missing_lock_respect_policy"));
        assert!(codes.contains("missing_rejection_policy"));
        assert!(codes.contains("missing_explanation_template"));
        assert!(codes.contains("candidate_strategy_unknown_provider_slot"));
    }

    #[test]
    fn preview_camera_policy_validates_gallery_consistency() {
        let policy = PreviewCameraPolicyDescriptor {
            default_camera: camera("default", "three-quarter"),
            direction_board_camera: camera("direction", "three-quarter"),
            option_gallery_camera: camera("option", "three-quarter"),
            contact_sheet_cameras: vec![
                camera("front", "front"),
                camera("side", "side"),
                camera("back", "back"),
                camera("three-quarter", "three-quarter"),
            ],
            option_gallery_policies: vec![OptionGalleryCameraPolicy {
                control_id: "support_style".to_owned(),
                option_camera_ids: vec!["option".to_owned(), "front".to_owned()],
                option_fitted_scale_policies: vec!["fit_model".to_owned(), "fit_part".to_owned()],
            }],
        };
        let report = validate_preview_camera_policy(&policy);
        assert!(
            report
                .issues
                .iter()
                .any(|issue| issue.code == "option_gallery_camera_not_consistent")
        );
    }

    #[test]
    fn preview_camera_policy_rejects_missing_identity_and_scale_policies() {
        let policy = PreviewCameraPolicyDescriptor {
            default_camera: PreviewCameraDescriptor {
                camera_id: String::new(),
                label: String::new(),
                view: String::new(),
                fitted_scale_policy: "fit_model".to_owned(),
                lighting_policy: "clay_reference".to_owned(),
                supported: true,
                unsupported_reason: None,
            },
            direction_board_camera: camera("direction", "three-quarter"),
            option_gallery_camera: camera("option", "three-quarter"),
            contact_sheet_cameras: vec![
                camera("front", "front"),
                camera("side", "side"),
                camera("back", "back"),
                camera("three-quarter", "three-quarter"),
            ],
            option_gallery_policies: vec![OptionGalleryCameraPolicy {
                control_id: "support_style".to_owned(),
                option_camera_ids: vec!["option".to_owned()],
                option_fitted_scale_policies: Vec::new(),
            }],
        };
        let report = validate_preview_camera_policy(&policy);
        let codes = issue_codes(&report);
        assert!(codes.contains("missing_camera_identity"));
        assert!(codes.contains("missing_option_gallery_fitted_scale"));
        assert!(codes.contains("option_gallery_policy_length_mismatch"));
    }

    #[test]
    fn quality_gate_runner_emits_honest_unsupported_states() {
        let mut package = sample_package();
        package.kit.source_profile_slug = None;
        let launches = author_quality_gate_launches(
            &package,
            &AuthorQualityArtifactRefs {
                package_manifest_ref: None,
                verified_built_in_backing: false,
                out_dir: "target/author".to_owned(),
                quality_report_ref: None,
                review_manifest_ref: None,
                contact_sheet_refs: Vec::new(),
            },
        );
        assert!(launches.iter().any(|launch| {
            launch.task == AuthorQualityGateTask::ValidateKit && !launch.supported
        }));
        assert!(launches.iter().any(|launch| {
            launch.task == AuthorQualityGateTask::BoxPrimitiveGate
                && !launch.supported
                && launch.unsupported_reason.is_some()
        }));
        assert!(launches.iter().any(|launch| {
            launch.task == AuthorQualityGateTask::ProduceReviewManifest && !launch.supported
        }));
    }

    #[test]
    fn quality_gate_runner_uses_package_ref_for_authored_packages() {
        let mut package = sample_package();
        package.kit.source_profile_slug = Some("sample".to_owned());
        let launches = author_quality_gate_launches(
            &package,
            &AuthorQualityArtifactRefs {
                package_manifest_ref: Some("target/author/foundry-kit-package.json".to_owned()),
                verified_built_in_backing: false,
                out_dir: "target/author".to_owned(),
                quality_report_ref: Some("target/author/quality-report.json".to_owned()),
                review_manifest_ref: None,
                contact_sheet_refs: Vec::new(),
            },
        );
        let validate = launches
            .iter()
            .find(|launch| launch.task == AuthorQualityGateTask::ValidateKit)
            .expect("validate launch row");
        assert!(validate.supported);
        assert!(validate.invocation.as_deref().is_some_and(|invocation| {
            invocation.contains("target/author/foundry-kit-package.json")
        }));
        assert!(launches.iter().any(|launch| {
            launch.task == AuthorQualityGateTask::PackageKit
                && !launch.supported
                && launch
                    .unsupported_reason
                    .as_deref()
                    .is_some_and(|reason| reason.contains("verified canonical"))
        }));
        assert!(launches.iter().any(|launch| {
            launch.task == AuthorQualityGateTask::ProduceReviewManifest && !launch.supported
        }));
    }

    #[test]
    fn quality_gate_runner_launches_verified_builtin_rows() {
        let package = sample_package();
        let launches = author_quality_gate_launches(
            &package,
            &AuthorQualityArtifactRefs {
                package_manifest_ref: None,
                verified_built_in_backing: true,
                out_dir: "target/author".to_owned(),
                quality_report_ref: Some("target/author/quality-report.json".to_owned()),
                review_manifest_ref: None,
                contact_sheet_refs: Vec::new(),
            },
        );
        assert!(launches.iter().all(|launch| launch.supported));
        assert!(launches.iter().any(|launch| {
            launch.task == AuthorQualityGateTask::BoxPrimitiveGate
                && launch
                    .invocation
                    .as_deref()
                    .is_some_and(|invocation| invocation.contains("box_primitive"))
        }));
    }

    #[test]
    fn package_export_manifest_includes_review_manifest() {
        let package = sample_package();
        let manifest = author_package_export_manifest(
            &package,
            &AuthorQualityArtifactRefs {
                package_manifest_ref: None,
                verified_built_in_backing: true,
                out_dir: "target/author".to_owned(),
                quality_report_ref: Some("quality-report.json".to_owned()),
                review_manifest_ref: Some("review-manifest.json".to_owned()),
                contact_sheet_refs: vec!["contact-sheet.png".to_owned()],
            },
        );
        assert_eq!(manifest.review_manifest_ref, "review-manifest.json");
        assert_eq!(manifest.kit_manifest_ref, "kit-manifest.json");
        assert_eq!(manifest.provider_pack_refs, vec!["provider-pack.json"]);
        assert_eq!(manifest.style_pack_refs, vec!["style-pack.json"]);
        assert_eq!(manifest.control_profile_ref, "control-profile.json");
        assert_eq!(
            manifest.candidate_strategy_pack_ref,
            "candidate-strategy-pack.json"
        );
        assert_eq!(
            manifest.quality_gate_profile_ref,
            "quality-gate-profile.json"
        );
        assert_eq!(manifest.contact_sheet_refs, vec!["contact-sheet.png"]);
    }

    fn issue_codes(report: &AuthorStudioValidationReport) -> BTreeSet<&str> {
        report
            .issues
            .iter()
            .map(|issue| issue.code.as_str())
            .collect()
    }

    fn sample_roles() -> Vec<AuthorRoleDescriptor> {
        vec![
            AuthorRoleDescriptor {
                role_id: "body".to_owned(),
                display_name: "Body".to_owned(),
                description: "Main box body.".to_owned(),
                required: true,
                repeated: false,
                default_visibility: true,
                export_part_name: "Body".to_owned(),
            },
            AuthorRoleDescriptor {
                role_id: "detail".to_owned(),
                display_name: "Detail".to_owned(),
                description: "Optional box detail.".to_owned(),
                required: true,
                repeated: false,
                default_visibility: true,
                export_part_name: "Detail".to_owned(),
            },
        ]
    }

    fn control(
        id: &str,
        label: &str,
        slot: &str,
        kind: ControlProfileControlKind,
    ) -> ControlMappingDescriptor {
        ControlMappingDescriptor {
            control_id: id.to_owned(),
            label: label.to_owned(),
            description: format!("Adjusts {label}."),
            kind,
            primary: true,
            visible: true,
            owned_family_slots: Vec::new(),
            owned_provider_slots: vec![slot.to_owned()],
            response_curve_descriptor: "linear".to_owned(),
            discrete_options: vec!["A".to_owned(), "B".to_owned()],
            provider_slot_binding: Some(slot.to_owned()),
            topology_behavior: if matches!(kind, ControlProfileControlKind::Choice) {
                ControlProfileTopologyBehavior::TopologyChanging
            } else {
                ControlProfileTopologyBehavior::TopologyPreserving
            },
            disabled_reason_policy: "Requires a compatible provider option.".to_owned(),
        }
    }

    fn camera(id: &str, view: &str) -> PreviewCameraDescriptor {
        PreviewCameraDescriptor {
            camera_id: id.to_owned(),
            label: id.to_owned(),
            view: view.to_owned(),
            fitted_scale_policy: "fit_model".to_owned(),
            lighting_policy: "clay_reference".to_owned(),
            supported: true,
            unsupported_reason: None,
        }
    }

    fn sample_package() -> FoundryKitPackage {
        FoundryKitPackage {
            schema_version: FOUNDRY_KIT_PACKAGE_SCHEMA_VERSION,
            kit: FoundryKit {
                schema_version: FOUNDRY_KIT_SCHEMA_VERSION,
                kit_id: "sample-kit".to_owned(),
                display_name: "Sample Kit".to_owned(),
                family_blueprint_id: "sample-family".to_owned(),
                provider_pack_id: "sample-provider".to_owned(),
                style_pack_id: "sample-style".to_owned(),
                control_profile_id: "sample-controls".to_owned(),
                candidate_strategy_pack_id: "sample-strategies".to_owned(),
                quality_gate_profile_id: "sample-quality".to_owned(),
                compatibility_matrix_id: "sample-compatibility".to_owned(),
                review_manifest_id: "sample-review".to_owned(),
                catalog_manifest_id: "sample-catalog".to_owned(),
                preview_camera_policy: PreviewCameraPolicy {
                    policy_id: "sample-preview".to_owned(),
                    required_views: vec!["front".to_owned(), "three-quarter".to_owned()],
                    clay_preview_required: true,
                    contact_sheet_required: false,
                },
                quality_tier: FoundryKitQualityTier::Draft,
                catalog_visibility_policy: CatalogVisibilityPolicy::hidden(
                    "Draft kits stay hidden.",
                ),
                source_profile_slug: Some("sample".to_owned()),
                category_chips: vec!["Author".to_owned()],
            },
            family_blueprint: FamilyBlueprint {
                schema_version: FAMILY_BLUEPRINT_SCHEMA_VERSION,
                family_id: "sample-family".to_owned(),
                display_name: "Sample Family".to_owned(),
                semantic_roles: vec![FamilyBlueprintRole {
                    role_id: "body".to_owned(),
                    label: "Body".to_owned(),
                    required: true,
                    tags: vec!["box".to_owned()],
                }],
                required_roles: vec!["body".to_owned()],
                optional_roles: Vec::new(),
                provider_slots: vec![ProviderSlotExpectation {
                    slot_id: "body_slot".to_owned(),
                    role_id: "body".to_owned(),
                    required: true,
                    attachment_tags: vec!["center".to_owned()],
                }],
                attachment_expectations: Vec::new(),
                scale_policy: HighLevelScalePolicy {
                    label: "Box scale".to_owned(),
                    allowed_range: Some("Authored box scale.".to_owned()),
                },
                export_part_naming_policy: ExportPartNamingPolicy {
                    strategy: "role labels".to_owned(),
                    required_part_names: vec!["Body".to_owned()],
                },
            },
            provider_pack: ProviderPack {
                schema_version: PROVIDER_PACK_SCHEMA_VERSION,
                pack_id: "sample-provider".to_owned(),
                family_id: Some("sample-family".to_owned()),
                compatible_family_ids: vec!["sample-family".to_owned()],
                provider_slots_supplied: vec!["body_slot".to_owned()],
                provider_options: vec![ProviderPackOption {
                    option_id: "simple_body".to_owned(),
                    slot_id: "body_slot".to_owned(),
                    label: "Simple Body".to_owned(),
                    semantic_roles: vec!["body".to_owned()],
                    compatibility_tags: vec!["plain".to_owned()],
                    detail_density_tags: vec!["clean".to_owned()],
                    triangle_budget_estimate: Some(512),
                }],
                semantic_role_coverage: vec!["body".to_owned()],
                socket_attachment_tags: vec!["center".to_owned()],
                detail_density_tags: vec!["clean".to_owned()],
                triangle_budget_estimates: BTreeMap::from([("simple_body".to_owned(), 512)]),
                compatibility_tags: vec!["plain".to_owned()],
            },
            style_pack: StylePack {
                schema_version: STYLE_PACK_SCHEMA_VERSION,
                style_id: "sample-style".to_owned(),
                display_name: "Sample Style".to_owned(),
                compatible_family_ids: vec!["sample-family".to_owned()],
                bevel_language: "Readable bevels.".to_owned(),
                proportion_language: "Plain proportions.".to_owned(),
                detail_density_policy: "Moderate details.".to_owned(),
                silhouette_exaggeration_policy: "Readable outline.".to_owned(),
                symmetry_asymmetry_policy: "Mostly symmetric.".to_owned(),
                allowed_provider_tags: vec!["plain".to_owned()],
                forbidden_provider_tags: Vec::new(),
                compatible_provider_packs: vec!["sample-provider".to_owned()],
                incompatible_provider_packs: Vec::new(),
                future_material_vocabulary: Some(FutureMaterialVocabulary {
                    label: "Reserved only".to_owned(),
                    tags: vec!["metal".to_owned()],
                }),
            },
            control_profile: ControlProfile {
                schema_version: CONTROL_PROFILE_SCHEMA_VERSION,
                profile_id: "sample-controls".to_owned(),
                family_id: "sample-family".to_owned(),
                style_id: Some("sample-style".to_owned()),
                maximum_primary_controls: DEFAULT_MAX_PRIMARY_NOVICE_CONTROLS,
                controls: vec![crate::ControlProfileControl {
                    control_id: "body_shape".to_owned(),
                    label: "Body Shape".to_owned(),
                    description: "Choose the body silhouette.".to_owned(),
                    kind: ControlProfileControlKind::Choice,
                    owned_family_slots: Vec::new(),
                    owned_provider_slots: vec!["body_slot".to_owned()],
                    visible_effect_expectation: "Body silhouette changes.".to_owned(),
                    topology_behavior: ControlProfileTopologyBehavior::TopologyChanging,
                    option_visibility: ControlOptionVisibility {
                        hide_invalid_from_novices: true,
                        show_plain_language_reasons: true,
                    },
                    default_locked: false,
                    primary: true,
                    visible: true,
                }],
            },
            candidate_strategy_pack: CandidateStrategyPack {
                schema_version: CANDIDATE_STRATEGY_PACK_SCHEMA_VERSION,
                pack_id: "sample-strategies".to_owned(),
                strategies: vec![KitCandidateStrategy {
                    strategy_id: "plain".to_owned(),
                    name: "Plain".to_owned(),
                    allowed_controls: vec!["body_shape".to_owned()],
                    explanation_templates: vec!["Body Shape becomes broader.".to_owned()],
                }],
                allowed_controls: vec!["body_shape".to_owned()],
                allowed_provider_choices: BTreeMap::from([(
                    "body_slot".to_owned(),
                    vec!["simple_body".to_owned()],
                )]),
                diversity_goals: vec!["shape".to_owned()],
                invalid_state_rejection_policy: "Reject invalid output.".to_owned(),
                lock_respect_policy: "Respect locked controls.".to_owned(),
            },
            quality_gate_profile: QualityGateProfile {
                schema_version: QUALITY_GATE_PROFILE_SCHEMA_VERSION,
                profile_id: "sample-quality".to_owned(),
                required_tier: FoundryKitQualityTier::Draft,
                mesh_gates: vec!["model validates".to_owned()],
                candidate_gates: vec!["six candidates".to_owned()],
                contact_sheet_gates: Vec::new(),
                export_gates: vec!["package export".to_owned()],
                manual_review_gates: vec!["manual review".to_owned()],
            },
            compatibility_matrix: KitCompatibilityMatrix {
                schema_version: KIT_COMPATIBILITY_MATRIX_SCHEMA_VERSION,
                matrix_id: "sample-compatibility".to_owned(),
                compatible_style_provider_pairs: Vec::new(),
                incompatible_style_provider_pairs: Vec::new(),
            },
            review_manifest: KitReviewManifest {
                schema_version: KIT_REVIEW_MANIFEST_SCHEMA_VERSION,
                manifest_id: "sample-review".to_owned(),
                tier_requested: FoundryKitQualityTier::Draft,
                tier_achieved: FoundryKitQualityTier::Draft,
                reviewer: None,
                human_approval_marker: false,
                adversarial_review_marker: false,
                visual_review_notes: Vec::new(),
                contact_sheet_paths: Vec::new(),
                benchmark_refs: Vec::new(),
                known_limitations: vec!["Descriptor only.".to_owned()],
                blocked_reasons: vec!["Manual review pending.".to_owned()],
            },
            catalog_manifest: KitCatalogManifest {
                schema_version: KIT_CATALOG_MANIFEST_SCHEMA_VERSION,
                catalog_id: "sample-catalog".to_owned(),
                kit_ids: vec!["sample-kit".to_owned()],
                default_visible_kit_ids: Vec::new(),
                developer_preview_kit_ids: vec!["sample-kit".to_owned()],
                hidden_kit_reasons: BTreeMap::from([(
                    "sample-kit".to_owned(),
                    "Draft kits stay hidden.".to_owned(),
                )]),
            },
        }
    }
}
