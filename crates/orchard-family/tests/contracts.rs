use std::collections::BTreeMap;

use orchard_family::{
    ASSET_FAMILY_SCHEMA_VERSION, AllowedOperationKind, AssetFamilySchema, AttachmentRule,
    BevelPolicy, ConstraintKind, DetailModule, ExaggerationPolicy, ExportRequirement,
    FamilyDefaultValue, FamilyParameterKind, FamilyParameterSlot, FamilyRuleExecutionPolicy,
    FamilyStyleFacet, FamilyStylePolicyOverrides, GeometricConstraint, LengthUnit, LengthValue,
    NormalizedBevelProfile, ParameterExecutionPolicy, ParameterRange, PartPrototype, PartRole,
    ProfileLanguage, ReadabilityThreshold, RepetitionPolicy, RoleMultiplicity, RoleProportion,
    RoleProvision, RuntimeMetadataRequirement, STYLE_KIT_SCHEMA_VERSION, StyleKit, SymmetryPolicy,
    VariantMode, VariantRule, validate_asset_family_schema, validate_family_style_compatibility,
    validate_family_style_completeness, validate_style_kit,
};

fn box_family(style_kit: &str) -> AssetFamilySchema {
    AssetFamilySchema {
        schema_version: ASSET_FAMILY_SCHEMA_VERSION,
        id: "box_primitive".to_owned(),
        display_name: "Box Primitive".to_owned(),
        summary:
            "Theme-neutral closed box volume with a body, sides, top, edges, and contact base."
                .to_owned(),
        part_roles: vec![
            role("body", RoleMultiplicity::Single, true, &["body"]),
            role(
                "side",
                RoleMultiplicity::Range { min: 1, max: 8 },
                true,
                &["wall"],
            ),
            role("top", RoleMultiplicity::Repeated, true, &["surface"]),
            role(
                "corner_edge",
                RoleMultiplicity::Optional,
                false,
                &["detail", "edge"],
            ),
            role(
                "base_contact",
                RoleMultiplicity::Repeated,
                false,
                &["contact"],
            ),
        ],
        attachment_rules: vec![
            AttachmentRule {
                id: "body_side".to_owned(),
                from_role: "body".to_owned(),
                to_role: "side".to_owned(),
                anchor_role: Some("base_contact".to_owned()),
                compatibility_tags: vec!["box_structure".to_owned()],
                required: true,
                execution_policy: FamilyRuleExecutionPolicy::Required,
            },
            AttachmentRule {
                id: "top_side".to_owned(),
                from_role: "top".to_owned(),
                to_role: "side".to_owned(),
                anchor_role: None,
                compatibility_tags: vec!["surface".to_owned()],
                required: true,
                execution_policy: FamilyRuleExecutionPolicy::Required,
            },
        ],
        allowed_operations: vec![
            AllowedOperationKind::Primitive,
            AllowedOperationKind::Array,
            AllowedOperationKind::Transform,
            AllowedOperationKind::Cut,
            AllowedOperationKind::Bevel,
        ],
        parameter_slots: vec![
            FamilyParameterSlot {
                id: "side_width".to_owned(),
                label: "Side Width".to_owned(),
                target_role: Some("side".to_owned()),
                kind: FamilyParameterKind::Length {
                    unit: LengthUnit::Meters,
                },
                range: Some(ParameterRange {
                    minimum: 0.5,
                    maximum: 8.0,
                    step: 0.25,
                }),
                default_value: Some(FamilyDefaultValue::Scalar(3.0)),
                execution_policy: ParameterExecutionPolicy::RequiredBinding,
                topology_changing: false,
            },
            FamilyParameterSlot {
                id: "side_count".to_owned(),
                label: "Side Count".to_owned(),
                target_role: Some("side".to_owned()),
                kind: FamilyParameterKind::Count,
                range: Some(ParameterRange {
                    minimum: 2.0,
                    maximum: 16.0,
                    step: 1.0,
                }),
                default_value: Some(FamilyDefaultValue::Integer(4)),
                execution_policy: ParameterExecutionPolicy::RequiredBinding,
                topology_changing: true,
            },
        ],
        constraints: vec![
            GeometricConstraint {
                id: "sides_touch_body".to_owned(),
                roles: vec!["side".to_owned(), "body".to_owned()],
                kind: ConstraintKind::MustSupport,
                execution_policy: FamilyRuleExecutionPolicy::Required,
            },
            GeometricConstraint {
                id: "top_has_clearance".to_owned(),
                roles: vec!["top".to_owned(), "base_contact".to_owned()],
                kind: ConstraintKind::Clearance,
                execution_policy: FamilyRuleExecutionPolicy::Advisory,
            },
        ],
        variant_rules: vec![
            VariantRule {
                id: "box_proportions".to_owned(),
                label: "Box proportions".to_owned(),
                mode: VariantMode::Proportion,
                editable_roles: vec!["side".to_owned(), "top".to_owned()],
                locked_by_tags: vec!["locked_box".to_owned()],
            },
            VariantRule {
                id: "edge_density".to_owned(),
                label: "Edge density".to_owned(),
                mode: VariantMode::Repetition,
                editable_roles: vec!["corner_edge".to_owned()],
                locked_by_tags: Vec::new(),
            },
        ],
        export_requirements: vec![ExportRequirement {
            profile: "asset-pack".to_owned(),
            required_metadata: vec![
                RuntimeMetadataRequirement::Pivot,
                RuntimeMetadataRequirement::CollisionProxies,
                RuntimeMetadataRequirement::Previews,
            ],
            triangle_budget_hint: Some(8_000),
        }],
        compatible_style_kits: vec![style_kit.to_owned()],
        tags: vec!["box".to_owned(), "clay".to_owned()],
    }
}

#[test]
fn legacy_attachment_rules_derive_execution_policy_from_required_flag() {
    let required: AttachmentRule = serde_json::from_str(
        r#"{
            "id":"body_side",
            "from_role":"body",
            "to_role":"side",
            "anchor_role":null,
            "compatibility_tags":["box_structure"],
            "required":true
        }"#,
    )
    .expect("legacy required rule parses");
    let optional: AttachmentRule = serde_json::from_str(
        r#"{
            "id":"edge_body",
            "from_role":"corner_edge",
            "to_role":"body",
            "anchor_role":null,
            "compatibility_tags":[],
            "required":false
        }"#,
    )
    .expect("legacy optional rule parses");

    assert_eq!(
        required.execution_policy,
        FamilyRuleExecutionPolicy::Required
    );
    assert_eq!(
        optional.execution_policy,
        FamilyRuleExecutionPolicy::Advisory
    );
}

#[test]
fn attachment_required_flag_and_execution_policy_must_agree() {
    let mut family = box_family("plain_box");
    family.attachment_rules[0].execution_policy = FamilyRuleExecutionPolicy::Advisory;
    let report = validate_asset_family_schema(&family);

    assert!(
        report
            .issues
            .iter()
            .any(|issue| issue.code == "required_attachment_policy_mismatch")
    );

    let mut family = box_family("plain_box");
    family.attachment_rules[0].required = false;
    let report = validate_asset_family_schema(&family);
    assert!(
        report
            .issues
            .iter()
            .any(|issue| issue.code == "optional_attachment_policy_mismatch")
    );
}

fn role(
    id: &str,
    multiplicity: RoleMultiplicity,
    required: bool,
    semantic_tags: &[&str],
) -> PartRole {
    PartRole {
        id: id.to_owned(),
        display_name: id.replace('_', " "),
        required,
        multiplicity,
        provision: if required {
            RoleProvision::StyleRequired
        } else {
            RoleProvision::FamilyOrStyle
        },
        semantic_tags: semantic_tags.iter().map(|tag| (*tag).to_owned()).collect(),
    }
}

fn box_style_kit() -> StyleKit {
    let proportions = vec![
        RoleProportion {
            role: "body".to_owned(),
            preferred_scale: [
                LengthValue::FamilyUnits(2.2),
                LengthValue::FamilyUnits(1.6),
                LengthValue::FamilyUnits(1.2),
            ],
            taper: 0.0,
        },
        RoleProportion {
            role: "side".to_owned(),
            preferred_scale: [
                LengthValue::FamilyUnits(2.0),
                LengthValue::FamilyUnits(0.12),
                LengthValue::FamilyUnits(1.0),
            ],
            taper: 0.0,
        },
    ];
    let part_prototypes = vec![
        PartPrototype {
            id: "box_body".to_owned(),
            display_name: "Box body".to_owned(),
            role: "body".to_owned(),
            operation_tags: vec![AllowedOperationKind::Primitive, AllowedOperationKind::Bevel],
            style_tags: vec!["body".to_owned()],
        },
        PartPrototype {
            id: "box_top".to_owned(),
            display_name: "Box top".to_owned(),
            role: "top".to_owned(),
            operation_tags: vec![AllowedOperationKind::Primitive, AllowedOperationKind::Cut],
            style_tags: vec!["surface".to_owned()],
        },
        PartPrototype {
            id: "box_side".to_owned(),
            display_name: "Box side".to_owned(),
            role: "side".to_owned(),
            operation_tags: vec![AllowedOperationKind::Primitive],
            style_tags: vec!["wall".to_owned()],
        },
    ];
    let detail_modules = vec![DetailModule {
        id: "edge_line".to_owned(),
        display_name: "Edge line".to_owned(),
        target_roles: vec!["top".to_owned(), "base_contact".to_owned()],
        minimum_readability: ReadabilityThreshold {
            pixels: 24,
            camera_profile: "oblique".to_owned(),
        },
        tags: vec!["edge_detail".to_owned()],
    }];
    StyleKit {
        schema_version: STYLE_KIT_SCHEMA_VERSION,
        id: "plain_box".to_owned(),
        display_name: "Plain Box".to_owned(),
        compatible_families: vec!["box_primitive".to_owned()],
        bevel_policy: BevelPolicy {
            width: LengthValue::FamilyUnits(0.04),
            segments: 2,
            profile: NormalizedBevelProfile { normalized: 0.5 },
        },
        profile_language: ProfileLanguage {
            curve_family: "straight".to_owned(),
            allowed_profiles: vec!["box".to_owned(), "tube".to_owned(), "channel".to_owned()],
            allow_asymmetry: false,
        },
        repetition: RepetitionPolicy {
            density: 0.65,
            preferred_spacing: LengthValue::FamilyUnits(0.25),
            maximum_default_count: 12,
        },
        symmetry: SymmetryPolicy {
            prefer_mirrors: true,
            allowed_axes: vec!["x".to_owned(), "y".to_owned()],
        },
        exaggeration: ExaggerationPolicy {
            silhouette: 0.2,
            detail: 0.45,
        },
        family_facets: BTreeMap::from([(
            "box_primitive".to_owned(),
            FamilyStyleFacet {
                family_id: "box_primitive".to_owned(),
                proportions,
                part_prototypes,
                detail_modules,
                policy_overrides: FamilyStylePolicyOverrides::default(),
            },
        )]),
        tags: vec!["clay".to_owned()],
    }
}

fn issue_codes(report: &orchard_family::FamilyValidationReport) -> Vec<&str> {
    report
        .issues
        .iter()
        .map(|issue| issue.code.as_str())
        .collect()
}

#[test]
fn family_and_style_kit_serde_round_trip() {
    let family = box_family("plain_box");
    let kit = box_style_kit();

    assert!(validate_asset_family_schema(&family).is_valid());
    assert!(validate_style_kit(&kit).is_valid());
    assert!(validate_family_style_compatibility(&family, &kit).is_valid());

    let family_json = serde_json::to_string_pretty(&family).expect("family should serialize");
    let kit_json = serde_json::to_string_pretty(&kit).expect("kit should serialize");
    let family_round_trip: AssetFamilySchema =
        serde_json::from_str(&family_json).expect("family should deserialize");
    let kit_round_trip: StyleKit = serde_json::from_str(&kit_json).expect("kit should deserialize");

    assert_eq!(family, family_round_trip);
    assert_eq!(kit, kit_round_trip);
}

#[test]
fn duplicate_role_id_is_rejected() {
    let mut family = box_family("plain_box");
    family.part_roles[1].id = family.part_roles[0].id.clone();

    let report = validate_asset_family_schema(&family);

    assert!(issue_codes(&report).contains(&"duplicate_part_role_id"));
}

#[test]
fn unknown_attachment_role_is_rejected() {
    let mut family = box_family("plain_box");
    family.attachment_rules[0].to_role = "missing".to_owned();

    let report = validate_asset_family_schema(&family);

    assert!(issue_codes(&report).contains(&"unknown_attachment_to_role"));
}

#[test]
fn invalid_parameter_range_is_rejected() {
    let mut family = box_family("plain_box");
    family.parameter_slots[0].range = Some(ParameterRange {
        minimum: 8.0,
        maximum: 2.0,
        step: 0.25,
    });

    let report = validate_asset_family_schema(&family);

    assert!(issue_codes(&report).contains(&"invalid_parameter_range"));
}

#[test]
fn parameter_kind_semantics_are_rejected() {
    let mut family = box_family("plain_box");
    family.parameter_slots[1].range = Some(ParameterRange {
        minimum: 2.5,
        maximum: 16.0,
        step: 1.0,
    });
    family.parameter_slots.push(FamilyParameterSlot {
        id: "has_edge_line".to_owned(),
        label: "Has Edge Line".to_owned(),
        target_role: Some("top".to_owned()),
        kind: FamilyParameterKind::Toggle,
        range: Some(ParameterRange {
            minimum: 0.0,
            maximum: 1.0,
            step: 1.0,
        }),
        default_value: Some(FamilyDefaultValue::Toggle(true)),
        execution_policy: ParameterExecutionPolicy::RequiredBinding,
        topology_changing: true,
    });

    let report = validate_asset_family_schema(&family);
    let codes = issue_codes(&report);

    assert!(codes.contains(&"non_integral_count_parameter_range"));
    assert!(codes.contains(&"toggle_parameter_has_range"));
}

#[test]
fn parameter_default_semantics_are_rejected() {
    let mut family = box_family("plain_box");
    family.parameter_slots[0].default_value = Some(FamilyDefaultValue::Scalar(99.0));
    family.parameter_slots[1].default_value = Some(FamilyDefaultValue::Choice("many".to_owned()));
    family.parameter_slots.push(FamilyParameterSlot {
        id: "mystery".to_owned(),
        label: "Mystery".to_owned(),
        target_role: None,
        kind: FamilyParameterKind::Custom("mystery".to_owned()),
        range: None,
        default_value: Some(FamilyDefaultValue::Toggle(true)),
        execution_policy: ParameterExecutionPolicy::RequiredBinding,
        topology_changing: false,
    });
    family.parameter_slots.push(FamilyParameterSlot {
        id: "has_edge_line".to_owned(),
        label: "Has Edge Line".to_owned(),
        target_role: Some("top".to_owned()),
        kind: FamilyParameterKind::Toggle,
        range: None,
        default_value: None,
        execution_policy: ParameterExecutionPolicy::RequiredBinding,
        topology_changing: true,
    });

    let report = validate_asset_family_schema(&family);
    let codes = issue_codes(&report);

    assert!(codes.contains(&"default_parameter_out_of_range"));
    assert!(codes.contains(&"default_parameter_type_mismatch"));
    assert!(codes.contains(&"missing_parameter_default"));
}

#[test]
fn semantic_parameter_ranges_and_relative_roles_are_rejected() {
    let mut family = box_family("plain_box");
    family.parameter_slots[0].kind = FamilyParameterKind::Length {
        unit: LengthUnit::RelativeToRole {
            role: "missing".to_owned(),
        },
    };
    family.parameter_slots[0].range = Some(ParameterRange {
        minimum: -1.0,
        maximum: 0.0,
        step: 0.25,
    });
    family.parameter_slots[1].range = Some(ParameterRange {
        minimum: -2.0,
        maximum: 4.0,
        step: 1.0,
    });

    let report = validate_asset_family_schema(&family);
    let codes = issue_codes(&report);

    assert!(codes.contains(&"unknown_relative_length_unit_role"));
    assert!(codes.contains(&"non_positive_length_parameter_range"));
    assert!(codes.contains(&"non_positive_count_parameter_range"));
}

#[test]
fn contradictory_requiredness_is_rejected() {
    let mut family = box_family("plain_box");
    family.part_roles[3].required = true;

    let report = validate_asset_family_schema(&family);

    assert!(issue_codes(&report).contains(&"required_optional_role"));
}

#[test]
fn unstable_identifier_format_is_rejected() {
    let mut family = box_family("plain_box");
    family.part_roles[0].id = "..".to_owned();

    let report = validate_asset_family_schema(&family);

    assert!(issue_codes(&report).contains(&"invalid_part_role_id"));
}

#[test]
fn duplicate_set_like_fields_are_rejected() {
    let mut family = box_family("plain_box");
    family
        .allowed_operations
        .push(AllowedOperationKind::Primitive);
    family.part_roles[0].semantic_tags.push("body".to_owned());
    family.tags.push("box".to_owned());
    family.constraints[0].roles.push("side".to_owned());
    family.variant_rules[0]
        .editable_roles
        .push("side".to_owned());

    let report = validate_asset_family_schema(&family);
    let codes = issue_codes(&report);

    assert!(codes.contains(&"duplicate_allowed_operation"));
    assert!(codes.contains(&"duplicate_part_role_tag"));
    assert!(codes.contains(&"duplicate_family_tag"));
    assert!(codes.contains(&"duplicate_constraint_role"));
    assert!(codes.contains(&"duplicate_variant_editable_role"));
}

#[test]
fn compatibility_and_provider_completeness_are_separate_reports() {
    let family = box_family("plain_box");
    let mut kit = box_style_kit();
    kit.family_facets
        .get_mut("box_primitive")
        .expect("box_primitive facet")
        .part_prototypes
        .retain(|prototype| prototype.role != "top");

    let compatibility = validate_family_style_compatibility(&family, &kit);
    let completeness = validate_family_style_completeness(&family, &kit);

    assert!(compatibility.is_valid());
    assert!(issue_codes(&completeness).contains(&"missing_style_required_role_provider"));
}

#[test]
fn style_facets_scope_role_references_to_the_selected_family() {
    let family = box_family("plain_box");
    let mut kit = box_style_kit();
    kit.compatible_families.push("foreign_family".to_owned());
    kit.family_facets.insert(
        "foreign_family".to_owned(),
        FamilyStyleFacet {
            family_id: "foreign_family".to_owned(),
            proportions: Vec::new(),
            part_prototypes: vec![PartPrototype {
                id: "foreign_body".to_owned(),
                display_name: "Foreign Body".to_owned(),
                role: "foreign_body".to_owned(),
                operation_tags: vec![AllowedOperationKind::Primitive],
                style_tags: vec!["foreign".to_owned()],
            }],
            detail_modules: Vec::new(),
            policy_overrides: FamilyStylePolicyOverrides::default(),
        },
    );

    let box_report = validate_family_style_compatibility(&family, &kit);
    assert!(
        box_report.is_valid(),
        "foreign-role role references should not poison box_primitive compatibility"
    );

    kit.family_facets
        .get_mut("box_primitive")
        .expect("box_primitive facet")
        .part_prototypes
        .push(PartPrototype {
            id: "bad_unknown_box_part".to_owned(),
            display_name: "Bad Unknown Box Part".to_owned(),
            role: "unknown_body".to_owned(),
            operation_tags: vec![AllowedOperationKind::Primitive],
            style_tags: vec!["invalid".to_owned()],
        });
    let bad_box_report = validate_family_style_compatibility(&family, &kit);
    assert!(issue_codes(&bad_box_report).contains(&"unknown_style_prototype_role"));
}

#[test]
fn global_style_policies_cannot_reference_family_roles() {
    let mut kit = box_style_kit();
    kit.bevel_policy.width = LengthValue::RelativeToRole {
        role: "side".to_owned(),
        ratio: 0.05,
    };
    kit.repetition.preferred_spacing = LengthValue::RelativeToRole {
        role: "top".to_owned(),
        ratio: 0.2,
    };

    let report = validate_style_kit(&kit);
    let codes = issue_codes(&report);

    assert_eq!(
        codes
            .iter()
            .filter(|code| **code == "global_style_policy_relative_to_role")
            .count(),
        2
    );
}

#[test]
fn older_style_kit_payloads_deserialize_before_schema_validation() {
    let mut value = serde_json::to_value(box_style_kit()).expect("style kit json");
    value["schema_version"] = serde_json::json!(STYLE_KIT_SCHEMA_VERSION - 2);
    value
        .as_object_mut()
        .expect("style kit object")
        .remove("family_facets");

    let kit: StyleKit = serde_json::from_value(value).expect("old payload should deserialize");
    assert!(kit.family_facets.is_empty());

    let report = validate_style_kit(&kit);
    assert!(issue_codes(&report).contains(&"unsupported_style_kit_schema"));
}

#[test]
fn schema_v3_style_kit_migrates_global_role_data_into_single_family_facet() {
    let mut value = serde_json::to_value(box_style_kit()).expect("style kit json");
    let object = value.as_object_mut().expect("style kit object");
    let box_facet = object
        .get_mut("family_facets")
        .and_then(|facets| facets.get_mut("box_primitive"))
        .and_then(|facet| facet.as_object_mut())
        .expect("box_primitive facet");
    let proportions = box_facet.remove("proportions").expect("proportions");
    let prototypes = box_facet
        .remove("part_prototypes")
        .expect("part prototypes");
    let details = box_facet.remove("detail_modules").expect("details");
    object.insert(
        "schema_version".to_owned(),
        serde_json::json!(STYLE_KIT_SCHEMA_VERSION - 1),
    );
    object.insert("proportions".to_owned(), proportions);
    object.insert("part_prototypes".to_owned(), prototypes);
    object.insert("detail_modules".to_owned(), details);
    object.remove("family_facets");

    let kit: StyleKit = serde_json::from_value(value).expect("legacy v3 kit should migrate");
    assert_eq!(kit.schema_version, STYLE_KIT_SCHEMA_VERSION);
    assert!(
        kit.family_facets
            .get("box_primitive")
            .expect("box_primitive facet")
            .part_prototypes
            .iter()
            .any(|prototype| prototype.id == "box_body")
    );
}

#[test]
fn schema_v3_style_kit_migrates_partial_global_role_data_without_false_facet_conflicts() {
    let mut value = serde_json::to_value(box_style_kit()).expect("style kit json");
    let object = value.as_object_mut().expect("style kit object");
    let box_facet = object
        .get_mut("family_facets")
        .and_then(|facets| facets.get_mut("box_primitive"))
        .and_then(|facet| facet.as_object_mut())
        .expect("box_primitive facet");
    let prototypes = box_facet
        .remove("part_prototypes")
        .expect("part prototypes");
    object.insert(
        "schema_version".to_owned(),
        serde_json::json!(STYLE_KIT_SCHEMA_VERSION - 1),
    );
    object.insert("part_prototypes".to_owned(), prototypes);

    let kit: StyleKit =
        serde_json::from_value(value).expect("partial legacy v3 kit should migrate");
    let facet = kit
        .family_facets
        .get("box_primitive")
        .expect("box_primitive facet");
    assert!(!facet.proportions.is_empty());
    assert!(
        facet
            .part_prototypes
            .iter()
            .any(|prototype| prototype.id == "box_body")
    );
}

#[test]
fn schema_v4_rejects_legacy_global_role_data() {
    let mut value = serde_json::to_value(box_style_kit()).expect("style kit json");
    let object = value.as_object_mut().expect("style kit object");
    object.insert(
        "proportions".to_owned(),
        serde_json::json!([{
            "role": "body",
            "preferred_scale": [
                {"FamilyUnits": 1.0},
                {"FamilyUnits": 1.0},
                {"FamilyUnits": 1.0}
            ],
            "taper": 0.0
        }]),
    );

    let result = serde_json::from_value::<StyleKit>(value);

    assert!(result.is_err());
}

#[test]
fn schema_v4_rejects_empty_legacy_global_role_fields() {
    let mut value = serde_json::to_value(box_style_kit()).expect("style kit json");
    value
        .as_object_mut()
        .expect("style kit object")
        .insert("part_prototypes".to_owned(), serde_json::json!([]));

    let result = serde_json::from_value::<StyleKit>(value);

    assert!(result.is_err());
}

#[test]
fn family_style_facets_reject_removed_default_provider_field() {
    let mut value = serde_json::to_value(box_style_kit()).expect("style kit json");
    value["family_facets"]["box_primitive"]["default_role_providers"] = serde_json::json!({});

    let result = serde_json::from_value::<StyleKit>(value);

    assert!(result.is_err());
}

#[test]
fn facet_only_style_kits_deserialize_and_validate() {
    let mut value = serde_json::to_value(box_style_kit()).expect("style kit json");
    let object = value.as_object_mut().expect("style kit object");
    object.remove("proportions");
    object.remove("part_prototypes");
    object.remove("detail_modules");

    let kit: StyleKit = serde_json::from_value(value).expect("facet-only kit should deserialize");

    assert!(validate_style_kit(&kit).is_valid());
    assert!(validate_family_style_compatibility(&box_family("plain_box"), &kit).is_valid());
}

#[test]
fn incompatible_style_and_family_are_rejected() {
    let family = box_family("soft_clay");
    let kit = box_style_kit();

    let report = validate_family_style_compatibility(&family, &kit);
    let codes = issue_codes(&report);

    assert!(codes.contains(&"style_kit_not_accepted_by_family"));
}

#[test]
fn prototype_operations_must_be_allowed_by_family() {
    let family = box_family("plain_box");
    let mut kit = box_style_kit();
    kit.family_facets
        .get_mut("box_primitive")
        .expect("box_primitive facet")
        .part_prototypes[0]
        .operation_tags
        .push(AllowedOperationKind::Lathe);

    let report = validate_family_style_compatibility(&family, &kit);

    assert!(issue_codes(&report).contains(&"style_prototype_operation_not_allowed"));
}

#[test]
fn duplicate_style_prototype_and_detail_ids_are_rejected() {
    let mut kit = box_style_kit();
    let facet = kit
        .family_facets
        .get_mut("box_primitive")
        .expect("box_primitive facet");
    facet.part_prototypes[1].id = facet.part_prototypes[0].id.clone();
    facet.detail_modules.push(facet.detail_modules[0].clone());
    kit.tags.push("clay".to_owned());

    let report = validate_style_kit(&kit);
    let codes = issue_codes(&report);

    assert!(codes.contains(&"duplicate_part_prototype_id"));
    assert!(codes.contains(&"duplicate_detail_module_id"));
    assert!(codes.contains(&"duplicate_style_kit_tag"));
}

#[test]
fn generic_examples_contain_no_pack_specific_names() {
    let family_json =
        serde_json::to_string(&box_family("plain_box")).expect("family should serialize");
    let kit_json = serde_json::to_string(&box_style_kit()).expect("kit should serialize");
    let combined = format!("{family_json}\n{kit_json}").to_lowercase();

    for forbidden in [
        "legacy-project",
        "plain_clay",
        "industrial steel",
        "deck",
        "span",
        "brace",
        "connector",
        "crate",
        "hard_surface",
        "external-style",
        "external project",
    ] {
        assert!(
            !combined.contains(forbidden),
            "generic schema fixture leaked {forbidden}"
        );
    }
}
