use shape_family::{
    ASSET_FAMILY_SCHEMA_VERSION, AllowedOperationKind, AssetFamilySchema, AttachmentRule,
    BevelPolicy, ConstraintKind, DetailModule, ExaggerationPolicy, ExportRequirement,
    FamilyDefaultValue, FamilyParameterKind, FamilyParameterSlot, GeometricConstraint, LengthUnit,
    LengthValue, NormalizedBevelProfile, ParameterRange, PartPrototype, PartRole, ProfileLanguage,
    ReadabilityThreshold, RepetitionPolicy, RoleMultiplicity, RoleProportion, RoleProvision,
    RuntimeMetadataRequirement, STYLE_KIT_SCHEMA_VERSION, StyleKit, SymmetryPolicy, VariantMode,
    VariantRule, validate_asset_family_schema, validate_family_style_compatibility,
    validate_family_style_completeness, validate_style_kit,
};

fn bridge_family(style_kit: &str) -> AssetFamilySchema {
    AssetFamilySchema {
        schema_version: ASSET_FAMILY_SCHEMA_VERSION,
        id: "bridge".to_owned(),
        display_name: "Bridge".to_owned(),
        summary:
            "Theme-neutral crossing structure with supports, spans, decks, braces, and connectors."
                .to_owned(),
        part_roles: vec![
            role("support", RoleMultiplicity::Repeated, true, &["support"]),
            role(
                "span",
                RoleMultiplicity::Range { min: 1, max: 8 },
                true,
                &["structure"],
            ),
            role("deck", RoleMultiplicity::Repeated, true, &["walkable"]),
            role(
                "brace",
                RoleMultiplicity::Optional,
                false,
                &["detail", "support"],
            ),
            role(
                "connector",
                RoleMultiplicity::Repeated,
                false,
                &["attachment"],
            ),
        ],
        attachment_rules: vec![
            AttachmentRule {
                id: "support_span".to_owned(),
                from_role: "support".to_owned(),
                to_role: "span".to_owned(),
                anchor_role: Some("connector".to_owned()),
                compatibility_tags: vec!["load_path".to_owned()],
                required: true,
            },
            AttachmentRule {
                id: "deck_span".to_owned(),
                from_role: "deck".to_owned(),
                to_role: "span".to_owned(),
                anchor_role: None,
                compatibility_tags: vec!["surface".to_owned()],
                required: true,
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
                id: "span_length".to_owned(),
                label: "Span Length".to_owned(),
                target_role: Some("span".to_owned()),
                kind: FamilyParameterKind::Length {
                    unit: LengthUnit::Meters,
                },
                range: Some(ParameterRange {
                    minimum: 0.5,
                    maximum: 8.0,
                    step: 0.25,
                }),
                default_value: Some(FamilyDefaultValue::Scalar(3.0)),
                topology_changing: false,
            },
            FamilyParameterSlot {
                id: "support_count".to_owned(),
                label: "Support Count".to_owned(),
                target_role: Some("support".to_owned()),
                kind: FamilyParameterKind::Count,
                range: Some(ParameterRange {
                    minimum: 2.0,
                    maximum: 16.0,
                    step: 1.0,
                }),
                default_value: Some(FamilyDefaultValue::Integer(4)),
                topology_changing: true,
            },
        ],
        constraints: vec![
            GeometricConstraint {
                id: "supports_touch_spans".to_owned(),
                roles: vec!["support".to_owned(), "span".to_owned()],
                kind: ConstraintKind::MustSupport,
            },
            GeometricConstraint {
                id: "deck_has_clearance".to_owned(),
                roles: vec!["deck".to_owned(), "connector".to_owned()],
                kind: ConstraintKind::Clearance,
            },
        ],
        variant_rules: vec![
            VariantRule {
                id: "span_proportions".to_owned(),
                label: "Span proportions".to_owned(),
                mode: VariantMode::Proportion,
                editable_roles: vec!["span".to_owned(), "deck".to_owned()],
                locked_by_tags: vec!["locked_structure".to_owned()],
            },
            VariantRule {
                id: "brace_density".to_owned(),
                label: "Brace density".to_owned(),
                mode: VariantMode::Repetition,
                editable_roles: vec!["brace".to_owned()],
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
        tags: vec!["modular".to_owned(), "hard_surface".to_owned()],
    }
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

fn industrial_style_kit() -> StyleKit {
    StyleKit {
        schema_version: STYLE_KIT_SCHEMA_VERSION,
        id: "industrial_steel".to_owned(),
        display_name: "Industrial Steel".to_owned(),
        compatible_families: vec!["bridge".to_owned(), "crate".to_owned()],
        proportions: vec![
            RoleProportion {
                role: "support".to_owned(),
                preferred_scale: [
                    LengthValue::FamilyUnits(0.35),
                    LengthValue::FamilyUnits(0.35),
                    LengthValue::FamilyUnits(1.8),
                ],
                taper: 0.0,
            },
            RoleProportion {
                role: "span".to_owned(),
                preferred_scale: [
                    LengthValue::FamilyUnits(3.0),
                    LengthValue::FamilyUnits(0.8),
                    LengthValue::FamilyUnits(0.25),
                ],
                taper: 0.0,
            },
        ],
        bevel_policy: BevelPolicy {
            width: LengthValue::RelativeToRole {
                role: "span".to_owned(),
                ratio: 0.04,
            },
            segments: 2,
            profile: NormalizedBevelProfile { normalized: 0.5 },
        },
        profile_language: ProfileLanguage {
            curve_family: "straight".to_owned(),
            allowed_profiles: vec!["box".to_owned(), "tube".to_owned(), "channel".to_owned()],
            allow_asymmetry: false,
        },
        part_prototypes: vec![
            PartPrototype {
                id: "box_support".to_owned(),
                display_name: "Box support".to_owned(),
                role: "support".to_owned(),
                operation_tags: vec![AllowedOperationKind::Primitive, AllowedOperationKind::Bevel],
                style_tags: vec!["structural".to_owned()],
            },
            PartPrototype {
                id: "deck_plate".to_owned(),
                display_name: "Deck plate".to_owned(),
                role: "deck".to_owned(),
                operation_tags: vec![AllowedOperationKind::Primitive, AllowedOperationKind::Cut],
                style_tags: vec!["surface".to_owned()],
            },
            PartPrototype {
                id: "box_span".to_owned(),
                display_name: "Box span".to_owned(),
                role: "span".to_owned(),
                operation_tags: vec![AllowedOperationKind::Primitive],
                style_tags: vec!["structural".to_owned()],
            },
        ],
        detail_modules: vec![DetailModule {
            id: "bolt_row".to_owned(),
            display_name: "Bolt row".to_owned(),
            target_roles: vec!["deck".to_owned(), "connector".to_owned()],
            minimum_readability: ReadabilityThreshold {
                pixels: 24,
                camera_profile: "oblique".to_owned(),
            },
            tags: vec!["fastener".to_owned()],
        }],
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
        tags: vec!["hard_surface".to_owned()],
    }
}

fn issue_codes(report: &shape_family::FamilyValidationReport) -> Vec<&str> {
    report
        .issues
        .iter()
        .map(|issue| issue.code.as_str())
        .collect()
}

#[test]
fn family_and_style_kit_serde_round_trip() {
    let family = bridge_family("industrial_steel");
    let kit = industrial_style_kit();

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
    let mut family = bridge_family("industrial_steel");
    family.part_roles[1].id = family.part_roles[0].id.clone();

    let report = validate_asset_family_schema(&family);

    assert!(issue_codes(&report).contains(&"duplicate_part_role_id"));
}

#[test]
fn unknown_attachment_role_is_rejected() {
    let mut family = bridge_family("industrial_steel");
    family.attachment_rules[0].to_role = "missing".to_owned();

    let report = validate_asset_family_schema(&family);

    assert!(issue_codes(&report).contains(&"unknown_attachment_to_role"));
}

#[test]
fn invalid_parameter_range_is_rejected() {
    let mut family = bridge_family("industrial_steel");
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
    let mut family = bridge_family("industrial_steel");
    family.parameter_slots[1].range = Some(ParameterRange {
        minimum: 2.5,
        maximum: 16.0,
        step: 1.0,
    });
    family.parameter_slots.push(FamilyParameterSlot {
        id: "has_rails".to_owned(),
        label: "Has Rails".to_owned(),
        target_role: Some("deck".to_owned()),
        kind: FamilyParameterKind::Toggle,
        range: Some(ParameterRange {
            minimum: 0.0,
            maximum: 1.0,
            step: 1.0,
        }),
        default_value: Some(FamilyDefaultValue::Toggle(true)),
        topology_changing: true,
    });

    let report = validate_asset_family_schema(&family);
    let codes = issue_codes(&report);

    assert!(codes.contains(&"non_integral_count_parameter_range"));
    assert!(codes.contains(&"toggle_parameter_has_range"));
}

#[test]
fn parameter_default_semantics_are_rejected() {
    let mut family = bridge_family("industrial_steel");
    family.parameter_slots[0].default_value = Some(FamilyDefaultValue::Scalar(99.0));
    family.parameter_slots[1].default_value = Some(FamilyDefaultValue::Choice("many".to_owned()));
    family.parameter_slots.push(FamilyParameterSlot {
        id: "mystery".to_owned(),
        label: "Mystery".to_owned(),
        target_role: None,
        kind: FamilyParameterKind::Custom("mystery".to_owned()),
        range: None,
        default_value: Some(FamilyDefaultValue::Toggle(true)),
        topology_changing: false,
    });
    family.parameter_slots.push(FamilyParameterSlot {
        id: "has_rails".to_owned(),
        label: "Has Rails".to_owned(),
        target_role: Some("deck".to_owned()),
        kind: FamilyParameterKind::Toggle,
        range: None,
        default_value: None,
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
    let mut family = bridge_family("industrial_steel");
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
    let mut family = bridge_family("industrial_steel");
    family.part_roles[3].required = true;

    let report = validate_asset_family_schema(&family);

    assert!(issue_codes(&report).contains(&"required_optional_role"));
}

#[test]
fn unstable_identifier_format_is_rejected() {
    let mut family = bridge_family("industrial_steel");
    family.part_roles[0].id = "..".to_owned();

    let report = validate_asset_family_schema(&family);

    assert!(issue_codes(&report).contains(&"invalid_part_role_id"));
}

#[test]
fn duplicate_set_like_fields_are_rejected() {
    let mut family = bridge_family("industrial_steel");
    family
        .allowed_operations
        .push(AllowedOperationKind::Primitive);
    family.part_roles[0]
        .semantic_tags
        .push("support".to_owned());
    family.tags.push("modular".to_owned());
    family.constraints[0].roles.push("support".to_owned());
    family.variant_rules[0]
        .editable_roles
        .push("span".to_owned());

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
    let family = bridge_family("industrial_steel");
    let mut kit = industrial_style_kit();
    kit.part_prototypes
        .retain(|prototype| prototype.role != "deck");

    let compatibility = validate_family_style_compatibility(&family, &kit);
    let completeness = validate_family_style_completeness(&family, &kit);

    assert!(compatibility.is_valid());
    assert!(issue_codes(&completeness).contains(&"missing_style_required_role_provider"));
}

#[test]
fn incompatible_style_and_family_are_rejected() {
    let family = bridge_family("stylized_wood");
    let kit = industrial_style_kit();

    let report = validate_family_style_compatibility(&family, &kit);
    let codes = issue_codes(&report);

    assert!(codes.contains(&"style_kit_not_accepted_by_family"));
}

#[test]
fn prototype_operations_must_be_allowed_by_family() {
    let family = bridge_family("industrial_steel");
    let mut kit = industrial_style_kit();
    kit.part_prototypes[0]
        .operation_tags
        .push(AllowedOperationKind::Lathe);

    let report = validate_family_style_compatibility(&family, &kit);

    assert!(issue_codes(&report).contains(&"style_prototype_operation_not_allowed"));
}

#[test]
fn duplicate_style_prototype_and_detail_ids_are_rejected() {
    let mut kit = industrial_style_kit();
    kit.part_prototypes[1].id = kit.part_prototypes[0].id.clone();
    kit.detail_modules.push(kit.detail_modules[0].clone());
    kit.tags.push("hard_surface".to_owned());

    let report = validate_style_kit(&kit);
    let codes = issue_codes(&report);

    assert!(codes.contains(&"duplicate_part_prototype_id"));
    assert!(codes.contains(&"duplicate_detail_module_id"));
    assert!(codes.contains(&"duplicate_style_kit_tag"));
}

#[test]
fn generic_examples_contain_no_pack_specific_names() {
    let family_json =
        serde_json::to_string(&bridge_family("industrial_steel")).expect("family should serialize");
    let kit_json = serde_json::to_string(&industrial_style_kit()).expect("kit should serialize");
    let combined = format!("{family_json}\n{kit_json}").to_lowercase();

    for forbidden in ["caesar", "roman", "gallic", "river bend"] {
        assert!(
            !combined.contains(forbidden),
            "generic schema fixture leaked {forbidden}"
        );
    }
}
