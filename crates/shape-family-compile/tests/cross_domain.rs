use std::collections::{BTreeMap, BTreeSet};

use shape_asset::{
    AssetId, AssetRecipe, Frame3, GeometryRecipe, GeometrySource, PartDefinition, PartDefinitionId,
    PartInstance, PartInstanceId, Transform3, definition_scalar_path, get_scalar,
    validate_asset_recipe,
};
use shape_caesar_assets::style_kits::roman_timber_engineering_style_kit;
use shape_compile::export::write_grouped_obj_export;
use shape_family::{
    ASSET_FAMILY_SCHEMA_VERSION, AllowedOperationKind, AssetFamilySchema, AttachmentRule,
    BevelPolicy, ConstraintKind, ExaggerationPolicy, ExportRequirement, FamilyParameterKind,
    FamilyParameterSlot, GeometricConstraint, LengthUnit, LengthValue, NormalizedBevelProfile,
    ParameterRange, PartPrototype, PartRole, ProfileLanguage, RepetitionPolicy, RoleMultiplicity,
    RoleProvision, RuntimeMetadataRequirement, STYLE_KIT_SCHEMA_VERSION, StyleKit, SymmetryPolicy,
    VariantMode, VariantRule,
};
use shape_family_compile::{
    FamilyCompileError, FamilyImplementation, FamilyInstantiationRequest, FamilyValue,
    ParameterBinding, RecipeFragment, ScalarTransform, StyleImplementation, instantiate_family,
    scalar_parameter,
};

const LOCAL_DEFINITION: PartDefinitionId = PartDefinitionId(90);
const LOCAL_INSTANCE: PartInstanceId = PartInstanceId(91);

#[test]
fn bridge_roman_crate_scifi_and_lamp_furniture_instantiate_compile_and_export() {
    let bridge = bridge_family(&["roman_timber_engineering"]);
    let roman_kit = roman_timber_engineering_style_kit();
    let bridge_impl = bridge_implementation();
    let roman_impl = roman_timber_style_implementation();
    let bridge_output = instantiate_family(
        &bridge,
        &roman_kit,
        &bridge_impl,
        &roman_impl,
        &request(
            "bridge",
            "roman_timber_engineering",
            [("span_length", FamilyValue::Scalar(3.4))],
        ),
    )
    .expect("bridge should instantiate");
    assert_concrete_exportable(&bridge_output);

    let crate_family = crate_family();
    let scifi_kit = scifi_industrial_style_kit();
    let crate_impl = crate_implementation();
    let scifi_impl = scifi_industrial_style_implementation();
    let crate_output = instantiate_family(
        &crate_family,
        &scifi_kit,
        &crate_impl,
        &scifi_impl,
        &request(
            "crate",
            "sci_fi_industrial",
            [
                ("body_width", FamilyValue::Scalar(1.35)),
                ("has_handle", FamilyValue::Toggle(true)),
                ("bolt_segments", FamilyValue::Integer(20)),
            ],
        ),
    )
    .expect("crate should instantiate");
    assert_concrete_exportable(&crate_output);
    assert!(
        crate_output
            .recipe
            .instances
            .values()
            .any(|instance| { instance.enabled && instance.name.contains("handle") })
    );

    let lamp_family = lamp_family();
    let furniture_kit = stylized_furniture_style_kit();
    let lamp_impl = lamp_implementation();
    let furniture_impl = stylized_furniture_style_implementation();
    let lamp_output = instantiate_family(
        &lamp_family,
        &furniture_kit,
        &lamp_impl,
        &furniture_impl,
        &request(
            "lamp",
            "stylized_furniture",
            [
                ("shade_scale", FamilyValue::Scalar(0.75)),
                ("stem_height", FamilyValue::Scalar(1.2)),
            ],
        ),
    )
    .expect("lamp should instantiate");
    assert_concrete_exportable(&lamp_output);
}

#[test]
fn family_controls_modify_intended_recipe_fields() {
    let family = bridge_family(&["industrial_steel"]);
    let kit = industrial_bridge_style_kit();
    let implementation = industrial_bridge_implementation();
    let style = industrial_bridge_style_implementation();

    let short = instantiate_family(
        &family,
        &kit,
        &implementation,
        &style,
        &request(
            "bridge",
            "industrial_steel",
            [
                ("span_length", FamilyValue::Scalar(2.5)),
                ("support_shape", FamilyValue::Choice("box".to_owned())),
            ],
        ),
    )
    .expect("short bridge");
    let long = instantiate_family(
        &family,
        &kit,
        &implementation,
        &style,
        &request(
            "bridge",
            "industrial_steel",
            [
                ("span_length", FamilyValue::Scalar(4.25)),
                ("support_shape", FamilyValue::Choice("box".to_owned())),
            ],
        ),
    )
    .expect("long bridge");

    let span_application = long
        .report
        .parameter_applications
        .iter()
        .find(|application| application.slot == "span_length")
        .expect("span application should be reported");
    assert_eq!(span_application.role, "span");
    assert_eq!(
        get_scalar(&long.recipe, &span_application.target).expect("concrete span scalar"),
        4.25
    );
    assert_ne!(
        short.report.source_recipe_hash, long.report.source_recipe_hash,
        "changing a family control should change the concrete recipe"
    );
}

#[test]
fn same_family_with_two_styles_produces_different_geometry() {
    let family = bridge_family(&["roman_timber_engineering", "industrial_steel"]);
    let roman = instantiate_family(
        &family,
        &roman_timber_engineering_style_kit(),
        &bridge_implementation(),
        &roman_timber_style_implementation(),
        &request(
            "bridge",
            "roman_timber_engineering",
            [("span_length", FamilyValue::Scalar(3.0))],
        ),
    )
    .expect("roman bridge");
    let industrial = instantiate_family(
        &family,
        &industrial_bridge_style_kit(),
        &industrial_bridge_implementation(),
        &industrial_bridge_style_implementation(),
        &request(
            "bridge",
            "industrial_steel",
            [
                ("span_length", FamilyValue::Scalar(3.0)),
                ("support_shape", FamilyValue::Choice("box".to_owned())),
            ],
        ),
    )
    .expect("industrial bridge");

    let roman_obj =
        write_grouped_obj_export(&roman.artifact, Some(&roman.recipe)).expect("roman export");
    let industrial_obj = write_grouped_obj_export(&industrial.artifact, Some(&industrial.recipe))
        .expect("industrial export");

    assert_ne!(
        roman.report.source_recipe_hash,
        industrial.report.source_recipe_hash
    );
    assert_ne!(roman_obj.obj, industrial_obj.obj);
}

#[test]
fn choice_binding_selects_style_prototype() {
    let family = bridge_family(&["industrial_steel"]);
    let output = instantiate_family(
        &family,
        &industrial_bridge_style_kit(),
        &industrial_bridge_implementation(),
        &industrial_bridge_style_implementation(),
        &request(
            "bridge",
            "industrial_steel",
            [
                ("span_length", FamilyValue::Scalar(2.8)),
                ("support_shape", FamilyValue::Choice("round".to_owned())),
            ],
        ),
    )
    .expect("bridge should instantiate with chosen support");

    assert_eq!(
        output.report.selected_providers.get("support"),
        Some(&"round_support".to_owned())
    );
}

#[test]
fn required_role_coverage_is_verified_before_geometry() {
    let family = crate_family();
    let mut kit = scifi_industrial_style_kit();
    kit.part_prototypes
        .retain(|prototype| prototype.role != "panel");
    let error = instantiate_family(
        &family,
        &kit,
        &crate_implementation(),
        &scifi_industrial_style_implementation(),
        &request(
            "crate",
            "sci_fi_industrial",
            [("body_width", FamilyValue::Scalar(1.2))],
        ),
    )
    .expect_err("missing required panel provider should fail");

    let FamilyCompileError::SchemaValidationFailed(report) = error else {
        panic!("expected schema validation failure");
    };
    assert!(
        report
            .issues
            .iter()
            .any(|issue| { issue.code == "missing_style_required_role_provider" })
    );
}

#[test]
fn ids_are_deterministically_remapped_and_instantiation_repeats_identically() {
    let family = crate_family();
    let kit = scifi_industrial_style_kit();
    let implementation = crate_implementation();
    let style = scifi_industrial_style_implementation();
    let request = request(
        "crate",
        "sci_fi_industrial",
        [
            ("body_width", FamilyValue::Scalar(1.4)),
            ("has_handle", FamilyValue::Toggle(false)),
            ("bolt_segments", FamilyValue::Integer(18)),
        ],
    );

    let first =
        instantiate_family(&family, &kit, &implementation, &style, &request).expect("first");
    let second =
        instantiate_family(&family, &kit, &implementation, &style, &request).expect("second");

    assert_eq!(
        first.report.source_recipe_hash,
        second.report.source_recipe_hash
    );
    assert_eq!(
        serde_json::to_string(&first.recipe).expect("first recipe json"),
        serde_json::to_string(&second.recipe).expect("second recipe json")
    );
    assert!(!first.recipe.definitions.contains_key(&LOCAL_DEFINITION));
    assert!(!first.recipe.instances.contains_key(&LOCAL_INSTANCE));
    assert_eq!(
        first.recipe.definitions.keys().copied().collect::<Vec<_>>(),
        vec![
            PartDefinitionId(1),
            PartDefinitionId(2),
            PartDefinitionId(3)
        ]
    );
}

#[test]
fn incompatible_family_style_pair_is_rejected_before_geometry() {
    let family = crate_family();
    let error = instantiate_family(
        &family,
        &roman_timber_engineering_style_kit(),
        &crate_implementation(),
        &roman_timber_style_implementation(),
        &request("crate", "roman_timber_engineering", []),
    )
    .expect_err("incompatible family/style pair should fail");

    let FamilyCompileError::SchemaValidationFailed(report) = error else {
        panic!("expected schema validation failure");
    };
    assert!(report.issues.iter().any(|issue| {
        issue.code == "style_kit_not_accepted_by_family"
            || issue.code == "family_not_accepted_by_style_kit"
    }));
}

#[test]
fn request_values_are_validated_against_family_slots() {
    let error = instantiate_family(
        &crate_family(),
        &scifi_industrial_style_kit(),
        &crate_implementation(),
        &scifi_industrial_style_implementation(),
        &request(
            "crate",
            "sci_fi_industrial",
            [
                ("body_width", FamilyValue::Scalar(8.0)),
                ("unknown_slot", FamilyValue::Toggle(true)),
            ],
        ),
    )
    .expect_err("invalid request values should fail before compilation");

    let FamilyCompileError::RequestValidationFailed(report) = error else {
        panic!("expected request validation failure");
    };
    let codes = report
        .issues
        .iter()
        .map(|issue| issue.code.as_str())
        .collect::<Vec<_>>();
    assert!(codes.contains(&"request_parameter_out_of_range"));
    assert!(codes.contains(&"unknown_request_parameter"));
}

#[test]
fn every_declared_choice_mapping_must_resolve_in_the_current_style() {
    let mut implementation = industrial_bridge_implementation();
    let ParameterBinding::ChoiceToPrototype { choices, .. } = implementation
        .parameter_bindings
        .iter_mut()
        .find(|binding| matches!(binding, ParameterBinding::ChoiceToPrototype { .. }))
        .expect("choice binding")
    else {
        panic!("expected choice binding");
    };
    choices.insert("round".to_owned(), "missing_round_support".to_owned());

    let error = instantiate_family(
        &bridge_family(&["industrial_steel"]),
        &industrial_bridge_style_kit(),
        &implementation,
        &industrial_bridge_style_implementation(),
        &request(
            "bridge",
            "industrial_steel",
            [
                ("span_length", FamilyValue::Scalar(3.0)),
                ("support_shape", FamilyValue::Choice("box".to_owned())),
            ],
        ),
    )
    .expect_err("unselected broken choice mapping should fail during implementation validation");

    let FamilyCompileError::ImplementationValidationFailed(report) = error else {
        panic!("expected implementation validation failure");
    };
    assert!(
        report
            .issues
            .iter()
            .any(|issue| issue.code == "unknown_choice_binding_prototype")
    );
}

#[test]
fn required_roles_cannot_be_disabled_by_toggle_bindings() {
    let mut family = crate_family();
    family.parameter_slots.push(FamilyParameterSlot {
        id: "body_enabled".to_owned(),
        label: "Body Enabled".to_owned(),
        target_role: Some("body".to_owned()),
        kind: FamilyParameterKind::Toggle,
        range: None,
        topology_changing: true,
    });
    let mut implementation = crate_implementation();
    implementation
        .parameter_bindings
        .push(ParameterBinding::TogglePartPresence {
            slot: "body_enabled".to_owned(),
            role: "body".to_owned(),
        });

    let error = instantiate_family(
        &family,
        &scifi_industrial_style_kit(),
        &implementation,
        &scifi_industrial_style_implementation(),
        &request(
            "crate",
            "sci_fi_industrial",
            [
                ("body_width", FamilyValue::Scalar(1.2)),
                ("body_enabled", FamilyValue::Toggle(false)),
            ],
        ),
    )
    .expect_err("required body role should not be disableable");

    let FamilyCompileError::RoleValidationFailed(report) = error else {
        panic!("expected role validation failure");
    };
    assert!(
        report
            .issues
            .iter()
            .any(|issue| { issue.code == "family_role_cardinality_unsatisfied" })
    );
}

#[test]
fn malformed_fragments_return_validation_reports_instead_of_panicking() {
    let mut style = scifi_industrial_style_implementation();
    style
        .prototypes
        .get_mut("armored_body")
        .expect("body fragment")
        .recipe
        .root_instances
        .push(PartInstanceId(999));

    let error = instantiate_family(
        &crate_family(),
        &scifi_industrial_style_kit(),
        &crate_implementation(),
        &style,
        &request(
            "crate",
            "sci_fi_industrial",
            [("body_width", FamilyValue::Scalar(1.2))],
        ),
    )
    .expect_err("invalid fragment should fail cleanly");

    let FamilyCompileError::FragmentValidationFailed { report, .. } = error else {
        panic!("expected fragment validation failure");
    };
    assert!(!report.issues.is_empty());
}

#[test]
fn family_default_provider_is_not_shadowed_by_same_named_style_prototype() {
    let mut family = crate_family();
    family.part_roles[0].provision = RoleProvision::FamilyDefault;

    let mut kit = scifi_industrial_style_kit();
    kit.part_prototypes.push(PartPrototype {
        id: "family_body".to_owned(),
        display_name: "Family Body Shadow".to_owned(),
        role: "body".to_owned(),
        operation_tags: vec![AllowedOperationKind::Primitive],
        style_tags: vec!["shadow".to_owned()],
    });

    let mut implementation = crate_implementation();
    implementation
        .default_role_providers
        .insert("body".to_owned(), "family_body".to_owned());
    implementation.fragments.insert(
        "family_body".to_owned(),
        rounded_box_fragment("family_body", "body", [0.33, 0.22, 0.11], 0.01),
    );

    let mut style = scifi_industrial_style_implementation();
    style.prototypes.insert(
        "family_body".to_owned(),
        rounded_box_fragment("family_body", "body", [1.8, 0.9, 0.9], 0.08),
    );

    let output = instantiate_family(
        &family,
        &kit,
        &implementation,
        &style,
        &request(
            "crate",
            "sci_fi_industrial",
            [("body_width", FamilyValue::Scalar(0.8))],
        ),
    )
    .expect("family default should be selected");

    assert_eq!(
        output.report.selected_providers.get("body"),
        Some(&"family_body".to_owned())
    );
    let body = output
        .recipe
        .definitions
        .get(&PartDefinitionId(1))
        .expect("body definition");
    let GeometrySource::RoundedBox { half_extents, .. } = body.geometry.source else {
        panic!("expected rounded family body");
    };
    assert_eq!(half_extents[1], 0.22);
}

#[test]
fn rogue_executable_style_prototypes_are_rejected() {
    let mut style = scifi_industrial_style_implementation();
    style.prototypes.insert(
        "rogue_panel".to_owned(),
        plate_fragment("rogue_panel", "panel", [0.5, 0.2], 0.05),
    );

    let error = instantiate_family(
        &crate_family(),
        &scifi_industrial_style_kit(),
        &crate_implementation(),
        &style,
        &request(
            "crate",
            "sci_fi_industrial",
            [("body_width", FamilyValue::Scalar(1.2))],
        ),
    )
    .expect_err("rogue executable style prototype should fail");

    let FamilyCompileError::ImplementationValidationFailed(report) = error else {
        panic!("expected implementation validation failure");
    };
    assert!(
        report
            .issues
            .iter()
            .any(|issue| { issue.code == "undeclared_executable_style_prototype" })
    );
}

#[test]
fn unsupported_fragment_metadata_is_rejected_instead_of_dropped() {
    let mut style = scifi_industrial_style_implementation();
    style
        .prototypes
        .get_mut("armored_body")
        .expect("body fragment")
        .recipe
        .instance_locks
        .insert(LOCAL_INSTANCE);

    let error = instantiate_family(
        &crate_family(),
        &scifi_industrial_style_kit(),
        &crate_implementation(),
        &style,
        &request(
            "crate",
            "sci_fi_industrial",
            [("body_width", FamilyValue::Scalar(1.2))],
        ),
    )
    .expect_err("unsupported fragment metadata should fail");

    assert!(matches!(
        error,
        FamilyCompileError::UnsupportedFragment { .. }
    ));
}

fn assert_concrete_exportable(output: &shape_family_compile::FamilyInstantiation) {
    assert!(validate_asset_recipe(&output.recipe).is_valid());
    assert!(!output.recipe.definitions.is_empty());
    assert!(!output.recipe.instances.is_empty());
    assert!(output.artifact.validation_report.is_valid());
    assert!(output.report.compiled_part_count > 0);
    let export = write_grouped_obj_export(&output.artifact, Some(&output.recipe))
        .expect("instantiated asset should export");
    assert!(export.report.object_count > 0);
    assert!(export.report.face_count > 0);
}

fn bridge_family(style_kits: &[&str]) -> AssetFamilySchema {
    AssetFamilySchema {
        schema_version: ASSET_FAMILY_SCHEMA_VERSION,
        id: "bridge".to_owned(),
        display_name: "Bridge".to_owned(),
        summary: "Theme-neutral crossing structure.".to_owned(),
        part_roles: vec![
            role("support", RoleMultiplicity::Repeated, true),
            role("span", RoleMultiplicity::Single, true),
            role("deck", RoleMultiplicity::Single, true),
            role("brace", RoleMultiplicity::Optional, false),
            role("connector", RoleMultiplicity::Repeated, false),
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
                topology_changing: false,
            },
            FamilyParameterSlot {
                id: "support_shape".to_owned(),
                label: "Support Shape".to_owned(),
                target_role: Some("support".to_owned()),
                kind: FamilyParameterKind::Choice(vec!["box".to_owned(), "round".to_owned()]),
                range: None,
                topology_changing: true,
            },
        ],
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
            profile: "asset-pack".to_owned(),
            required_metadata: vec![RuntimeMetadataRequirement::Pivot],
            triangle_budget_hint: Some(8_000),
        }],
        compatible_style_kits: style_kits.iter().map(|id| (*id).to_owned()).collect(),
        tags: vec!["crossing".to_owned()],
    }
}

fn crate_family() -> AssetFamilySchema {
    AssetFamilySchema {
        schema_version: ASSET_FAMILY_SCHEMA_VERSION,
        id: "crate".to_owned(),
        display_name: "Crate".to_owned(),
        summary: "Theme-neutral hard-surface container.".to_owned(),
        part_roles: vec![
            role("body", RoleMultiplicity::Single, true),
            role("panel", RoleMultiplicity::Repeated, true),
            role("fastener", RoleMultiplicity::Repeated, true),
            role("handle", RoleMultiplicity::Optional, false),
        ],
        attachment_rules: Vec::new(),
        allowed_operations: vec![
            AllowedOperationKind::Primitive,
            AllowedOperationKind::Transform,
            AllowedOperationKind::Bevel,
        ],
        parameter_slots: vec![
            FamilyParameterSlot {
                id: "body_width".to_owned(),
                label: "Body Width".to_owned(),
                target_role: Some("body".to_owned()),
                kind: FamilyParameterKind::Length {
                    unit: LengthUnit::Meters,
                },
                range: Some(ParameterRange {
                    minimum: 0.7,
                    maximum: 2.0,
                    step: 0.05,
                }),
                topology_changing: false,
            },
            FamilyParameterSlot {
                id: "has_handle".to_owned(),
                label: "Has Handle".to_owned(),
                target_role: Some("handle".to_owned()),
                kind: FamilyParameterKind::Toggle,
                range: None,
                topology_changing: true,
            },
            FamilyParameterSlot {
                id: "bolt_segments".to_owned(),
                label: "Bolt Segments".to_owned(),
                target_role: Some("fastener".to_owned()),
                kind: FamilyParameterKind::Count,
                range: Some(ParameterRange {
                    minimum: 8.0,
                    maximum: 32.0,
                    step: 1.0,
                }),
                topology_changing: true,
            },
        ],
        constraints: Vec::new(),
        variant_rules: Vec::new(),
        export_requirements: Vec::new(),
        compatible_style_kits: vec!["sci_fi_industrial".to_owned()],
        tags: vec!["container".to_owned()],
    }
}

fn lamp_family() -> AssetFamilySchema {
    AssetFamilySchema {
        schema_version: ASSET_FAMILY_SCHEMA_VERSION,
        id: "lamp".to_owned(),
        display_name: "Lamp".to_owned(),
        summary: "Theme-neutral desk lamp.".to_owned(),
        part_roles: vec![
            role("base", RoleMultiplicity::Single, true),
            role("stem", RoleMultiplicity::Single, true),
            role("shade", RoleMultiplicity::Single, true),
        ],
        attachment_rules: Vec::new(),
        allowed_operations: vec![AllowedOperationKind::Primitive, AllowedOperationKind::Bevel],
        parameter_slots: vec![
            FamilyParameterSlot {
                id: "shade_scale".to_owned(),
                label: "Shade Scale".to_owned(),
                target_role: Some("shade".to_owned()),
                kind: FamilyParameterKind::Ratio,
                range: Some(ParameterRange {
                    minimum: 0.0,
                    maximum: 1.0,
                    step: 0.05,
                }),
                topology_changing: false,
            },
            FamilyParameterSlot {
                id: "stem_height".to_owned(),
                label: "Stem Height".to_owned(),
                target_role: Some("stem".to_owned()),
                kind: FamilyParameterKind::Length {
                    unit: LengthUnit::Meters,
                },
                range: Some(ParameterRange {
                    minimum: 0.4,
                    maximum: 2.0,
                    step: 0.05,
                }),
                topology_changing: false,
            },
        ],
        constraints: Vec::new(),
        variant_rules: Vec::new(),
        export_requirements: Vec::new(),
        compatible_style_kits: vec!["stylized_furniture".to_owned()],
        tags: vec!["lighting".to_owned()],
    }
}

fn role(id: &str, multiplicity: RoleMultiplicity, required: bool) -> PartRole {
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
        semantic_tags: vec![id.to_owned()],
    }
}

fn industrial_bridge_style_kit() -> StyleKit {
    style_kit(
        "industrial_steel",
        "Industrial Steel",
        &["bridge"],
        &[
            ("box_support", "Box support", "support"),
            ("round_support", "Round support", "support"),
            ("box_span", "Box span", "span"),
            ("deck_plate", "Deck plate", "deck"),
        ],
    )
}

fn scifi_industrial_style_kit() -> StyleKit {
    style_kit(
        "sci_fi_industrial",
        "Sci-Fi Industrial",
        &["crate"],
        &[
            ("armored_body", "Armored body", "body"),
            ("inset_panel", "Inset panel", "panel"),
            ("bolt_head", "Bolt head", "fastener"),
            ("side_handle", "Side handle", "handle"),
        ],
    )
}

fn stylized_furniture_style_kit() -> StyleKit {
    style_kit(
        "stylized_furniture",
        "Stylized Furniture",
        &["lamp"],
        &[
            ("rounded_base", "Rounded base", "base"),
            ("tapered_stem", "Tapered stem", "stem"),
            ("soft_shade", "Soft shade", "shade"),
        ],
    )
}

fn style_kit(
    id: &str,
    display_name: &str,
    compatible_families: &[&str],
    prototypes: &[(&str, &str, &str)],
) -> StyleKit {
    StyleKit {
        schema_version: STYLE_KIT_SCHEMA_VERSION,
        id: id.to_owned(),
        display_name: display_name.to_owned(),
        compatible_families: compatible_families
            .iter()
            .map(|family| (*family).to_owned())
            .collect(),
        proportions: Vec::new(),
        bevel_policy: BevelPolicy {
            width: LengthValue::FamilyUnits(0.03),
            segments: 1,
            profile: NormalizedBevelProfile { normalized: 0.5 },
        },
        profile_language: ProfileLanguage {
            curve_family: "straight".to_owned(),
            allowed_profiles: vec!["box".to_owned(), "round".to_owned()],
            allow_asymmetry: false,
        },
        part_prototypes: prototypes
            .iter()
            .map(|(id, display_name, role)| PartPrototype {
                id: (*id).to_owned(),
                display_name: (*display_name).to_owned(),
                role: (*role).to_owned(),
                operation_tags: vec![AllowedOperationKind::Primitive],
                style_tags: vec![id.replace('_', "-")],
            })
            .collect(),
        detail_modules: Vec::new(),
        repetition: RepetitionPolicy {
            density: 0.5,
            preferred_spacing: LengthValue::FamilyUnits(0.2),
            maximum_default_count: 8,
        },
        symmetry: SymmetryPolicy {
            prefer_mirrors: true,
            allowed_axes: vec!["x".to_owned()],
        },
        exaggeration: ExaggerationPolicy {
            silhouette: 0.25,
            detail: 0.4,
        },
        tags: vec![id.to_owned()],
    }
}

fn bridge_implementation() -> FamilyImplementation {
    FamilyImplementation {
        family_id: "bridge".to_owned(),
        base_recipe: AssetRecipe::new(AssetId(1), "Bridge base"),
        role_bindings: BTreeMap::new(),
        parameter_bindings: vec![ParameterBinding::Scalar {
            slot: "span_length".to_owned(),
            role: "span".to_owned(),
            local_path: definition_scalar_path(
                LOCAL_DEFINITION,
                "geometry.rounded_box.half_extents.x",
            ),
            transform: ScalarTransform::Direct,
        }],
        variant_bindings: BTreeMap::new(),
        default_role_providers: BTreeMap::new(),
        fragments: BTreeMap::new(),
    }
}

fn industrial_bridge_implementation() -> FamilyImplementation {
    let mut implementation = bridge_implementation();
    implementation
        .parameter_bindings
        .push(ParameterBinding::ChoiceToPrototype {
            slot: "support_shape".to_owned(),
            role: "support".to_owned(),
            choices: BTreeMap::from([
                ("box".to_owned(), "box_support".to_owned()),
                ("round".to_owned(), "round_support".to_owned()),
            ]),
        });
    implementation
}

fn crate_implementation() -> FamilyImplementation {
    FamilyImplementation {
        family_id: "crate".to_owned(),
        base_recipe: AssetRecipe::new(AssetId(1), "Crate base"),
        role_bindings: BTreeMap::new(),
        parameter_bindings: vec![
            ParameterBinding::Scalar {
                slot: "body_width".to_owned(),
                role: "body".to_owned(),
                local_path: definition_scalar_path(
                    LOCAL_DEFINITION,
                    "geometry.rounded_box.half_extents.x",
                ),
                transform: ScalarTransform::Direct,
            },
            ParameterBinding::TogglePartPresence {
                slot: "has_handle".to_owned(),
                role: "handle".to_owned(),
            },
            ParameterBinding::Scalar {
                slot: "bolt_segments".to_owned(),
                role: "fastener".to_owned(),
                local_path: definition_scalar_path(
                    LOCAL_DEFINITION,
                    "geometry.cylinder.radial_segments",
                ),
                transform: ScalarTransform::IntegerCount,
            },
        ],
        variant_bindings: BTreeMap::new(),
        default_role_providers: BTreeMap::new(),
        fragments: BTreeMap::new(),
    }
}

fn lamp_implementation() -> FamilyImplementation {
    FamilyImplementation {
        family_id: "lamp".to_owned(),
        base_recipe: AssetRecipe::new(AssetId(1), "Lamp base"),
        role_bindings: BTreeMap::new(),
        parameter_bindings: vec![
            ParameterBinding::Scalar {
                slot: "shade_scale".to_owned(),
                role: "shade".to_owned(),
                local_path: definition_scalar_path(LOCAL_DEFINITION, "geometry.cylinder.radius"),
                transform: ScalarTransform::Ratio {
                    minimum: 0.18,
                    maximum: 0.48,
                },
            },
            ParameterBinding::Scalar {
                slot: "stem_height".to_owned(),
                role: "stem".to_owned(),
                local_path: definition_scalar_path(LOCAL_DEFINITION, "geometry.cylinder.height"),
                transform: ScalarTransform::ScaleOffset {
                    scale: 1.0,
                    offset: 0.1,
                },
            },
        ],
        variant_bindings: BTreeMap::new(),
        default_role_providers: BTreeMap::new(),
        fragments: BTreeMap::new(),
    }
}

fn roman_timber_style_implementation() -> StyleImplementation {
    StyleImplementation {
        style_kit_id: "roman_timber_engineering".to_owned(),
        prototypes: BTreeMap::from([
            (
                "pointed_round_pile".to_owned(),
                cylinder_fragment("pointed_round_pile", "support", 0.16, 1.25, 18),
            ),
            (
                "lashed_deck_plank".to_owned(),
                plate_fragment("lashed_deck_plank", "deck", [3.0, 0.85], 0.09),
            ),
            (
                "hewn_span_beam".to_owned(),
                rounded_box_fragment("hewn_span_beam", "span", [1.5, 0.12, 0.16], 0.02),
            ),
        ]),
        detail_modules: BTreeMap::new(),
    }
}

fn industrial_bridge_style_implementation() -> StyleImplementation {
    StyleImplementation {
        style_kit_id: "industrial_steel".to_owned(),
        prototypes: BTreeMap::from([
            (
                "box_support".to_owned(),
                rounded_box_fragment("box_support", "support", [0.15, 0.15, 0.8], 0.015),
            ),
            (
                "round_support".to_owned(),
                cylinder_fragment("round_support", "support", 0.15, 1.6, 24),
            ),
            (
                "box_span".to_owned(),
                rounded_box_fragment("box_span", "span", [1.4, 0.14, 0.12], 0.03),
            ),
            (
                "deck_plate".to_owned(),
                plate_fragment("deck_plate", "deck", [2.8, 0.8], 0.08),
            ),
        ]),
        detail_modules: BTreeMap::new(),
    }
}

fn scifi_industrial_style_implementation() -> StyleImplementation {
    StyleImplementation {
        style_kit_id: "sci_fi_industrial".to_owned(),
        prototypes: BTreeMap::from([
            (
                "armored_body".to_owned(),
                rounded_box_fragment("armored_body", "body", [1.0, 0.45, 0.65], 0.08),
            ),
            (
                "inset_panel".to_owned(),
                plate_fragment("inset_panel", "panel", [1.35, 0.42], 0.045),
            ),
            (
                "bolt_head".to_owned(),
                cylinder_fragment("bolt_head", "fastener", 0.06, 0.035, 16),
            ),
            (
                "side_handle".to_owned(),
                rounded_box_fragment("side_handle", "handle", [0.12, 0.06, 0.45], 0.03),
            ),
        ]),
        detail_modules: BTreeMap::new(),
    }
}

fn stylized_furniture_style_implementation() -> StyleImplementation {
    StyleImplementation {
        style_kit_id: "stylized_furniture".to_owned(),
        prototypes: BTreeMap::from([
            (
                "rounded_base".to_owned(),
                cylinder_fragment("rounded_base", "base", 0.42, 0.12, 32),
            ),
            (
                "tapered_stem".to_owned(),
                cylinder_fragment("tapered_stem", "stem", 0.06, 1.0, 20),
            ),
            (
                "soft_shade".to_owned(),
                cylinder_fragment("soft_shade", "shade", 0.34, 0.44, 32),
            ),
        ]),
        detail_modules: BTreeMap::new(),
    }
}

fn rounded_box_fragment(
    id: &str,
    role: &str,
    half_extents: [f32; 3],
    radius: f32,
) -> RecipeFragment {
    fragment(
        id,
        role,
        GeometrySource::RoundedBox {
            half_extents,
            radius,
        },
        &[
            ("geometry.rounded_box.half_extents.x", 0.05, 5.0, 0.05),
            ("geometry.rounded_box.half_extents.y", 0.05, 5.0, 0.05),
            ("geometry.rounded_box.half_extents.z", 0.05, 5.0, 0.05),
            ("geometry.rounded_box.radius", 0.0, 0.5, 0.01),
        ],
    )
}

fn plate_fragment(id: &str, role: &str, size: [f32; 2], thickness: f32) -> RecipeFragment {
    fragment(
        id,
        role,
        GeometrySource::Plate { size, thickness },
        &[
            ("geometry.plate.size.x", 0.05, 5.0, 0.05),
            ("geometry.plate.size.y", 0.05, 5.0, 0.05),
            ("geometry.plate.thickness", 0.01, 0.5, 0.01),
        ],
    )
}

fn cylinder_fragment(
    id: &str,
    role: &str,
    radius: f32,
    height: f32,
    radial_segments: u32,
) -> RecipeFragment {
    fragment(
        id,
        role,
        GeometrySource::Cylinder {
            radius,
            height,
            radial_segments,
        },
        &[
            ("geometry.cylinder.radius", 0.01, 2.0, 0.01),
            ("geometry.cylinder.height", 0.01, 5.0, 0.01),
            ("geometry.cylinder.radial_segments", 6.0, 64.0, 1.0),
        ],
    )
}

fn fragment(
    id: &str,
    role: &str,
    source: GeometrySource,
    scalar_paths: &[(&str, f32, f32, f32)],
) -> RecipeFragment {
    let mut recipe = AssetRecipe::new(AssetId(1), format!("{id} fragment"));
    recipe.definitions.insert(
        LOCAL_DEFINITION,
        PartDefinition {
            id: LOCAL_DEFINITION,
            name: format!("{id} definition"),
            tags: BTreeSet::from([role.to_owned()]),
            geometry: GeometryRecipe {
                source,
                operations: Vec::new(),
            },
            regions: BTreeMap::new(),
            sockets: BTreeMap::new(),
            local_pivot: Frame3::default(),
            variant_group: None,
            production_hints: None,
        },
    );
    recipe.instances.insert(
        LOCAL_INSTANCE,
        PartInstance {
            id: LOCAL_INSTANCE,
            definition: LOCAL_DEFINITION,
            name: format!("{id} {role}"),
            parent: None,
            local_transform: Transform3::default(),
            attachment: None,
            enabled: true,
            tags: BTreeSet::from([role.to_owned()]),
            generated_by: None,
        },
    );
    recipe.root_instances.push(LOCAL_INSTANCE);
    for (index, (path, minimum, maximum, step)) in scalar_paths.iter().enumerate() {
        let parameter_id = (index + 1) as u64;
        recipe.parameters.insert(
            shape_asset::ParameterId(parameter_id),
            scalar_parameter(
                parameter_id,
                definition_scalar_path(LOCAL_DEFINITION, path),
                format!("{id} {path}"),
                *minimum,
                *maximum,
                *step,
                path.ends_with("radial_segments"),
            ),
        );
    }
    recipe.next_ids.part_definition = LOCAL_DEFINITION.0 + 1;
    recipe.next_ids.part_instance = LOCAL_INSTANCE.0 + 1;
    recipe.next_ids.parameter = scalar_paths.len() as u64 + 1;
    assert!(validate_asset_recipe(&recipe).is_valid());
    RecipeFragment {
        id: id.to_owned(),
        role: role.to_owned(),
        recipe,
    }
}

fn request<'a>(
    family_id: &str,
    style_kit_id: &str,
    values: impl IntoIterator<Item = (&'a str, FamilyValue)>,
) -> FamilyInstantiationRequest {
    FamilyInstantiationRequest {
        family_id: family_id.to_owned(),
        style_kit_id: style_kit_id.to_owned(),
        parameters: values
            .into_iter()
            .map(|(key, value)| (key.to_owned(), value))
            .collect(),
        seed: 42,
    }
}
