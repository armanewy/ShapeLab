use shape_caesar_assets::style_kits::roman_timber_engineering_style_kit;
use shape_family::{
    ASSET_FAMILY_SCHEMA_VERSION, AllowedOperationKind, AssetFamilySchema, AttachmentRule,
    ConstraintKind, ExportRequirement, FamilyParameterKind, FamilyParameterSlot,
    GeometricConstraint, ParameterRange, PartRole, RoleMultiplicity, RuntimeMetadataRequirement,
    VariantMode, VariantRule, validate_family_style_compatibility, validate_style_kit,
};

fn generic_bridge_family() -> AssetFamilySchema {
    AssetFamilySchema {
        schema_version: ASSET_FAMILY_SCHEMA_VERSION,
        id: "bridge".to_owned(),
        display_name: "Bridge".to_owned(),
        summary: "Theme-neutral crossing structure.".to_owned(),
        part_roles: vec![
            role("support", RoleMultiplicity::Repeated),
            role("span", RoleMultiplicity::Repeated),
            role("deck", RoleMultiplicity::Repeated),
            role("brace", RoleMultiplicity::Optional),
            role("connector", RoleMultiplicity::Repeated),
        ],
        attachment_rules: vec![AttachmentRule {
            id: "support_span".to_owned(),
            from_role: "support".to_owned(),
            to_role: "span".to_owned(),
            anchor_role: Some("connector".to_owned()),
            compatibility_tags: vec!["load_path".to_owned()],
            required: true,
        }],
        allowed_operations: vec![
            AllowedOperationKind::Primitive,
            AllowedOperationKind::Array,
            AllowedOperationKind::Transform,
            AllowedOperationKind::Lathe,
            AllowedOperationKind::Bevel,
        ],
        parameter_slots: vec![FamilyParameterSlot {
            id: "span_length".to_owned(),
            label: "Span Length".to_owned(),
            target_role: Some("span".to_owned()),
            kind: FamilyParameterKind::Length,
            range: Some(ParameterRange {
                minimum: 0.5,
                maximum: 6.0,
                step: 0.25,
            }),
            topology_changing: false,
        }],
        constraints: vec![GeometricConstraint {
            id: "deck_supported".to_owned(),
            roles: vec!["deck".to_owned(), "support".to_owned()],
            kind: ConstraintKind::MustSupport,
        }],
        variant_rules: vec![VariantRule {
            id: "support_rhythm".to_owned(),
            label: "Support rhythm".to_owned(),
            mode: VariantMode::Repetition,
            editable_roles: vec!["support".to_owned()],
            locked_by_tags: Vec::new(),
        }],
        export_requirements: vec![ExportRequirement {
            profile: "game-runtime".to_owned(),
            required_metadata: vec![
                RuntimeMetadataRequirement::Pivot,
                RuntimeMetadataRequirement::SnapAnchors,
                RuntimeMetadataRequirement::WalkableSurfaces,
            ],
            triangle_budget_hint: Some(3_000),
        }],
        compatible_style_kits: vec!["roman_timber_engineering".to_owned()],
        tags: vec!["crossing".to_owned()],
    }
}

fn role(id: &str, multiplicity: RoleMultiplicity) -> PartRole {
    PartRole {
        id: id.to_owned(),
        display_name: id.replace('_', " "),
        required: !matches!(multiplicity, RoleMultiplicity::Optional),
        multiplicity,
        semantic_tags: vec![id.to_owned()],
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
fn roman_timber_style_kit_is_valid_pack_specific_data() {
    let kit = roman_timber_engineering_style_kit();

    assert!(validate_style_kit(&kit).is_valid());
    assert!(kit.tags.iter().any(|tag| tag == "roman"));
}

#[test]
fn roman_timber_style_kit_is_compatible_with_generic_bridge_family() {
    let family = generic_bridge_family();
    let kit = roman_timber_engineering_style_kit();

    let report = validate_family_style_compatibility(&family, &kit);

    assert!(
        report.is_valid(),
        "expected compatibility, got {:?}",
        issue_codes(&report)
    );
}
