use std::collections::{BTreeMap, BTreeSet};

use shape_asset::{
    AssetId, AssetRecipe, AttachmentMode, BoundaryLoopId, Frame3, GeometryRecipe, GeometrySource,
    OperationId, ParameterId, PartDefinition, PartDefinitionId, PartInstance, PartInstanceId,
    RegionId, SocketId, SocketSpec, Transform3, definition_scalar_path, get_scalar,
    validate_asset_recipe,
};
use shape_caesar_assets::style_kits::roman_timber_engineering_style_kit;
use shape_compile::export::write_grouped_obj_export;
use shape_family::{
    ASSET_FAMILY_SCHEMA_VERSION, AllowedOperationKind, AssetFamilySchema, AttachmentRule,
    BevelPolicy, ConstraintKind, ExaggerationPolicy, ExportRequirement, FamilyDefaultValue,
    FamilyParameterKind, FamilyParameterSlot, FamilyStyleFacet, FamilyStylePolicyOverrides,
    GeometricConstraint, LengthUnit, LengthValue, NormalizedBevelProfile, ParameterExecutionPolicy,
    ParameterRange, PartPrototype, PartRole, ProfileLanguage, RepetitionPolicy, RoleMultiplicity,
    RoleProvision, RuntimeMetadataRequirement, STYLE_KIT_SCHEMA_VERSION, StyleKit, SymmetryPolicy,
    VariantMode, VariantRule,
};
use shape_family_compile::{
    FAMILY_IMPLEMENTATION_SCHEMA_VERSION, FamilyCompileError, FamilyImplementation,
    FamilyInstantiationRequest, FamilyValue, FragmentAttachmentBinding, FragmentAttachmentPairing,
    FragmentSocketPort, FragmentSurfacePort, FragmentSurfaceTarget, ParameterBinding,
    RECIPE_FRAGMENT_SCHEMA_VERSION, RecipeFragment, RecipeFragmentExports, RigidOffset,
    STYLE_IMPLEMENTATION_SCHEMA_VERSION, ScalarTransform, StyleImplementation, instantiate_family,
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
    kit.family_facets
        .get_mut("crate")
        .expect("crate facet")
        .part_prototypes
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
fn fragment_remap_reports_are_complete_unique_and_fresh() {
    let output = instantiate_family(
        &crate_family(),
        &scifi_industrial_style_kit(),
        &crate_implementation(),
        &scifi_industrial_style_implementation(),
        &request("crate", "sci_fi_industrial", []),
    )
    .expect("crate instantiates");

    assert!(!output.report.fragment_remaps.is_empty());
    for report in &output.report.fragment_remaps {
        assert_numeric_remap(
            &report.remap.definitions,
            &report.allocated.definitions,
            output.recipe.next_ids.part_definition,
        );
        assert_numeric_remap(
            &report.remap.instances,
            &report.allocated.instances,
            output.recipe.next_ids.part_instance,
        );
        assert_numeric_remap(
            &report.remap.parameters,
            &report.allocated.parameters,
            output.recipe.next_ids.parameter,
        );
        assert_numeric_remap(
            &report.remap.operations,
            &report.allocated.operations,
            output.recipe.next_ids.operation,
        );
        assert_numeric_remap(
            &report.remap.regions,
            &report.allocated.regions,
            output.recipe.next_ids.region,
        );
        assert_numeric_remap(
            &report.remap.boundary_loops,
            &report.allocated.boundary_loops,
            output.recipe.next_ids.boundary_loop,
        );
        assert_numeric_remap(
            &report.remap.sockets,
            &report.allocated.sockets,
            output.recipe.next_ids.socket,
        );
    }
    assert!(!output.recipe.definitions.contains_key(&LOCAL_DEFINITION));
    assert!(!output.recipe.instances.contains_key(&LOCAL_INSTANCE));
}

#[test]
fn omitted_request_values_use_family_parameter_defaults() {
    let output = instantiate_family(
        &crate_family(),
        &scifi_industrial_style_kit(),
        &crate_implementation(),
        &scifi_industrial_style_implementation(),
        &request("crate", "sci_fi_industrial", []),
    )
    .expect("defaulted request should instantiate");

    assert_eq!(
        output.report.selected_providers.get("body"),
        Some(&"armored_body".to_owned())
    );
    assert!(
        !output.report.selected_providers.contains_key("handle"),
        "default false presence toggle should not merge the optional handle"
    );
    assert!(
        output
            .report
            .parameter_applications
            .iter()
            .any(|application| {
                application.slot == "body_width" && application.value == "1.200000"
            })
    );
    assert!(
        output
            .report
            .parameter_applications
            .iter()
            .any(|application| {
                application.slot == "bolt_segments" && application.value == "16.000000"
            })
    );
}

#[test]
fn style_default_provider_is_explicit_not_map_ordered() {
    let family = bridge_family(&["industrial_steel"]);
    let mut kit = industrial_bridge_style_kit();
    kit.family_facets
        .get_mut("bridge")
        .expect("bridge facet")
        .part_prototypes
        .push(PartPrototype {
            id: "aaa_round_support".to_owned(),
            display_name: "Alphabetically First Support".to_owned(),
            role: "support".to_owned(),
            operation_tags: vec![AllowedOperationKind::Primitive],
            style_tags: vec!["test".to_owned()],
        });
    let mut style = industrial_bridge_style_implementation();
    style.prototypes.insert(
        "aaa_round_support".to_owned(),
        cylinder_fragment("aaa_round_support", "support", 0.2, 2.0, 18),
    );

    let output = instantiate_family(
        &family,
        &kit,
        &industrial_bridge_implementation(),
        &style,
        &request(
            "bridge",
            "industrial_steel",
            [("span_length", FamilyValue::Scalar(3.0))],
        ),
    )
    .expect("explicit default should instantiate");

    assert_eq!(
        output.report.selected_providers.get("support"),
        Some(&"box_support".to_owned())
    );
}

#[test]
fn missing_required_style_default_provider_is_rejected() {
    let mut style = scifi_industrial_style_implementation();
    style.default_role_providers.remove("body");

    let error = instantiate_family(
        &crate_family(),
        &scifi_industrial_style_kit(),
        &crate_implementation(),
        &style,
        &request("crate", "sci_fi_industrial", []),
    )
    .expect_err("required style role without explicit default should fail");

    let FamilyCompileError::ImplementationValidationFailed(report) = error else {
        panic!("expected implementation validation failure");
    };
    assert!(
        report
            .issues
            .iter()
            .any(|issue| { issue.code == "missing_required_style_default_provider" })
    );
}

#[test]
fn role_cardinality_counts_exported_occurrence_roots_not_internal_fragments() {
    let mut style = scifi_industrial_style_implementation();
    let body = style
        .prototypes
        .get_mut("armored_body")
        .expect("body fragment");
    body.recipe.definitions.insert(
        PartDefinitionId(91),
        PartDefinition {
            id: PartDefinitionId(91),
            name: "internal rib definition".to_owned(),
            tags: BTreeSet::from(["body".to_owned(), "internal".to_owned()]),
            geometry: GeometryRecipe {
                source: GeometrySource::RoundedBox {
                    half_extents: [0.08, 0.02, 0.3],
                    radius: 0.01,
                },
                operations: Vec::new(),
            },
            regions: BTreeMap::new(),
            sockets: BTreeMap::new(),
            local_pivot: Frame3::default(),
            variant_group: None,
            production_hints: None,
        },
    );
    body.recipe.instances.insert(
        PartInstanceId(92),
        PartInstance {
            id: PartInstanceId(92),
            definition: PartDefinitionId(91),
            name: "internal body rib".to_owned(),
            parent: Some(LOCAL_INSTANCE),
            local_transform: Transform3::default(),
            attachment: None,
            enabled: true,
            tags: BTreeSet::from(["body".to_owned(), "internal".to_owned()]),
            generated_by: None,
        },
    );
    body.recipe.instances.insert(
        PartInstanceId(93),
        PartInstance {
            id: PartInstanceId(93),
            definition: PartDefinitionId(91),
            name: "internal helper root".to_owned(),
            parent: None,
            local_transform: Transform3::default(),
            attachment: None,
            enabled: true,
            tags: BTreeSet::from(["body".to_owned(), "internal".to_owned()]),
            generated_by: None,
        },
    );
    body.recipe.root_instances.push(PartInstanceId(93));
    body.exports.internal_roots.push(PartInstanceId(93));
    body.recipe.next_ids.part_definition = 92;
    body.recipe.next_ids.part_instance = 94;

    let output = instantiate_family(
        &crate_family(),
        &scifi_industrial_style_kit(),
        &crate_implementation(),
        &style,
        &request("crate", "sci_fi_industrial", []),
    )
    .expect("single exported body occurrence should satisfy cardinality");

    assert!(
        output
            .recipe
            .instances
            .values()
            .any(|instance| { instance.name == "internal body rib" && instance.enabled })
    );
    assert!(
        output
            .recipe
            .instances
            .values()
            .any(|instance| { instance.name == "internal helper root" && instance.enabled })
    );
}

#[test]
fn executable_binding_schema_versions_are_enforced() {
    let mut implementation = crate_implementation();
    implementation.schema_version = FAMILY_IMPLEMENTATION_SCHEMA_VERSION + 1;

    let error = instantiate_family(
        &crate_family(),
        &scifi_industrial_style_kit(),
        &implementation,
        &scifi_industrial_style_implementation(),
        &request("crate", "sci_fi_industrial", []),
    )
    .expect_err("unsupported family implementation version should fail");

    let FamilyCompileError::ImplementationValidationFailed(report) = error else {
        panic!("expected implementation validation failure");
    };
    assert!(
        report
            .issues
            .iter()
            .any(|issue| { issue.code == "unsupported_family_implementation_schema" })
    );

    let mut style = scifi_industrial_style_implementation();
    style.schema_version = STYLE_IMPLEMENTATION_SCHEMA_VERSION + 1;

    let error = instantiate_family(
        &crate_family(),
        &scifi_industrial_style_kit(),
        &crate_implementation(),
        &style,
        &request("crate", "sci_fi_industrial", []),
    )
    .expect_err("unsupported style implementation version should fail");

    let FamilyCompileError::ImplementationValidationFailed(report) = error else {
        panic!("expected implementation validation failure");
    };
    assert!(
        report
            .issues
            .iter()
            .any(|issue| { issue.code == "unsupported_style_implementation_schema" })
    );

    let mut value =
        serde_json::to_value(scifi_industrial_style_implementation()).expect("style json");
    value["schema_version"] = serde_json::json!(STYLE_IMPLEMENTATION_SCHEMA_VERSION - 1);
    value
        .as_object_mut()
        .expect("style implementation object")
        .remove("family_id");
    serde_json::from_value::<StyleImplementation>(value)
        .expect_err("v3 style implementation requires an explicit family_id");

    let mut style = scifi_industrial_style_implementation();
    style
        .prototypes
        .get_mut("armored_body")
        .expect("body fragment")
        .schema_version = RECIPE_FRAGMENT_SCHEMA_VERSION + 1;

    let error = instantiate_family(
        &crate_family(),
        &scifi_industrial_style_kit(),
        &crate_implementation(),
        &style,
        &request("crate", "sci_fi_industrial", []),
    )
    .expect_err("unsupported fragment version should fail");

    let FamilyCompileError::ImplementationValidationFailed(report) = error else {
        panic!("expected implementation validation failure");
    };
    assert!(
        report
            .issues
            .iter()
            .any(|issue| { issue.code == "unsupported_recipe_fragment_schema" })
    );
}

#[test]
fn removed_executable_binding_placeholders_are_rejected_by_strict_schemas() {
    let mut family_impl_json =
        serde_json::to_value(crate_implementation()).expect("family implementation json");
    family_impl_json
        .as_object_mut()
        .expect("family implementation object")
        .insert("role_bindings".to_owned(), serde_json::json!({}));
    serde_json::from_value::<FamilyImplementation>(family_impl_json)
        .expect_err("family implementation v3 must reject removed role_bindings");

    let mut fragment_json =
        serde_json::to_value(rounded_box_fragment("body", "body", [1.0, 0.5, 0.5], 0.05))
            .expect("fragment json");
    fragment_json
        .as_object_mut()
        .expect("fragment object")
        .insert(
            "role_occurrence_roots".to_owned(),
            serde_json::json!([LOCAL_INSTANCE]),
        );
    serde_json::from_value::<RecipeFragment>(fragment_json)
        .expect_err("fragment v2 must reject removed direct root exports");
}

#[test]
fn rich_metadata_in_unselected_fragments_does_not_block_selected_providers() {
    let mut style = industrial_bridge_style_implementation();
    style
        .prototypes
        .get_mut("round_support")
        .expect("unselected support fragment")
        .recipe
        .instance_locks
        .insert(LOCAL_INSTANCE);

    let output = instantiate_family(
        &bridge_family(&["industrial_steel"]),
        &industrial_bridge_style_kit(),
        &industrial_bridge_implementation(),
        &style,
        &request("bridge", "industrial_steel", []),
    )
    .expect("unselected rich metadata should not block selected providers");

    assert!(output.recipe.instance_locks.is_empty());
}

#[test]
fn fragment_ports_validate_stable_ids_and_local_references() {
    let mut style = scifi_industrial_style_implementation();
    style
        .prototypes
        .get_mut("armored_body")
        .expect("body fragment")
        .exports
        .socket_ports
        .push(FragmentSocketPort {
            id: "Bad Port".to_owned(),
            local_occurrence_root: PartInstanceId(999),
            local_socket: SocketId(1),
            compatibility_tags: vec!["mount".to_owned()],
        });

    let error = instantiate_family(
        &crate_family(),
        &scifi_industrial_style_kit(),
        &crate_implementation(),
        &style,
        &request("crate", "sci_fi_industrial", []),
    )
    .expect_err("invalid fragment port should fail implementation validation");

    let FamilyCompileError::ImplementationValidationFailed(report) = error else {
        panic!("expected implementation validation failure");
    };
    let codes = issue_codes(&report);
    assert!(codes.contains(&"invalid_fragment_port_id"));
    assert!(codes.contains(&"unknown_fragment_socket_port_occurrence"));
}

#[test]
fn selected_fragment_socket_ports_are_remapped_into_the_instantiated_recipe() {
    let mut style = scifi_industrial_style_implementation();
    let body = style
        .prototypes
        .get_mut("armored_body")
        .expect("body fragment");
    body.recipe
        .definitions
        .get_mut(&LOCAL_DEFINITION)
        .expect("body definition")
        .sockets
        .insert(
            SocketId(7),
            SocketSpec {
                id: SocketId(7),
                name: "Mount".to_owned(),
                local_frame: Frame3::default(),
                role: "mount".to_owned(),
                tags: BTreeSet::from(["mount".to_owned()]),
            },
        );
    body.exports.socket_ports.push(FragmentSocketPort {
        id: "mount".to_owned(),
        local_occurrence_root: LOCAL_INSTANCE,
        local_socket: SocketId(7),
        compatibility_tags: vec!["mount".to_owned()],
    });
    body.recipe.next_ids.socket = 8;

    let output = instantiate_family(
        &crate_family(),
        &scifi_industrial_style_kit(),
        &crate_implementation(),
        &style,
        &request("crate", "sci_fi_industrial", []),
    )
    .expect("socket ports should be remapped");

    assert!(
        output
            .recipe
            .definitions
            .values()
            .any(|definition| !definition.sockets.is_empty())
    );
    assert!(
        output
            .report
            .fragment_remaps
            .iter()
            .any(|report| !report.remap.sockets.is_empty())
    );
}

#[test]
fn socket_and_surface_port_ids_share_one_fragment_namespace() {
    let mut style = scifi_industrial_style_implementation();
    let body = style
        .prototypes
        .get_mut("armored_body")
        .expect("body fragment");
    body.exports.socket_ports.push(FragmentSocketPort {
        id: "mount".to_owned(),
        local_occurrence_root: LOCAL_INSTANCE,
        local_socket: SocketId(7),
        compatibility_tags: Vec::new(),
    });
    body.exports.surface_ports.push(FragmentSurfacePort {
        id: "mount".to_owned(),
        target: FragmentSurfaceTarget::Definition(LOCAL_DEFINITION),
        local_region: RegionId(1),
        semantic_tags: Vec::new(),
    });

    let error = instantiate_family(
        &crate_family(),
        &scifi_industrial_style_kit(),
        &crate_implementation(),
        &style,
        &request("crate", "sci_fi_industrial", []),
    )
    .expect_err("duplicate port namespace should fail implementation validation");

    let FamilyCompileError::ImplementationValidationFailed(report) = error else {
        panic!("expected implementation validation failure");
    };
    assert!(issue_codes(&report).contains(&"duplicate_fragment_port_id"));
}

#[test]
fn fragment_attachment_bindings_attach_child_occurrences_to_parent_ports() {
    let mut implementation = bridge_implementation();
    implementation
        .attachment_bindings
        .push(FragmentAttachmentBinding {
            family_attachment_rule: "support_span".to_owned(),
            parent_role: "span".to_owned(),
            parent_port: "span_socket".to_owned(),
            child_role: "support".to_owned(),
            child_port: "support_socket".to_owned(),
            pairing: FragmentAttachmentPairing::ByOccurrenceIndex,
            rigid_offset: RigidOffset::default(),
            attachment_mode: AttachmentMode::RigidSeparate,
        });
    let mut style = roman_timber_style_implementation();
    add_socket_port(
        style
            .prototypes
            .get_mut("pointed_round_pile")
            .expect("support fragment"),
        "support_socket",
        "support",
        "load_path",
    );
    add_socket_port(
        style
            .prototypes
            .get_mut("hewn_span_beam")
            .expect("span fragment"),
        "span_socket",
        "span",
        "load_path",
    );

    let output = instantiate_family(
        &bridge_family(&["roman_timber_engineering"]),
        &roman_timber_engineering_style_kit(),
        &implementation,
        &style,
        &request("bridge", "roman_timber_engineering", []),
    )
    .expect("attachment binding should apply through remapped ports");

    assert_eq!(output.report.fragment_attachment_applications.len(), 1);
    let attachment = &output.report.fragment_attachment_applications[0];
    assert_eq!(attachment.child_role, "support");
    assert_eq!(attachment.parent_role, "span");
    let child = output
        .recipe
        .instances
        .get(&attachment.child_instance)
        .expect("attached child instance");
    assert_eq!(child.parent, Some(attachment.parent_instance));
    assert!(child.attachment.is_some());
}

#[test]
fn asset_id_is_hash_derived_from_seed_and_semantic_parameters() {
    let family = crate_family();
    let kit = scifi_industrial_style_kit();
    let implementation = crate_implementation();
    let style = scifi_industrial_style_implementation();
    let mut seed_zero = request("crate", "sci_fi_industrial", []);
    seed_zero.seed = 0;
    let mut seed_one = seed_zero.clone();
    seed_one.seed = 1;
    let mut wider = seed_zero.clone();
    wider
        .parameters
        .insert("body_width".to_owned(), FamilyValue::Scalar(1.6));

    let zero =
        instantiate_family(&family, &kit, &implementation, &style, &seed_zero).expect("seed zero");
    let one =
        instantiate_family(&family, &kit, &implementation, &style, &seed_one).expect("seed one");
    let wider =
        instantiate_family(&family, &kit, &implementation, &style, &wider).expect("wider body");

    assert_ne!(zero.recipe.id, AssetId(0));
    assert_ne!(zero.recipe.id, AssetId(1));
    assert_ne!(zero.recipe.id, one.recipe.id);
    assert_ne!(zero.recipe.id, wider.recipe.id);
    assert_eq!(zero.report.instantiation_fingerprint.len(), 64);
    assert_eq!(zero.report.geometry_input_fingerprint.len(), 64);
    assert_eq!(zero.report.foundry_intent_fingerprint.len(), 64);
    assert_eq!(
        zero.report.instantiation_fingerprint,
        zero.report.geometry_input_fingerprint
    );
}

#[test]
fn export_requirements_do_not_change_geometry_identity() {
    let family = crate_family();
    let mut export_changed = family.clone();
    export_changed.export_requirements.push(ExportRequirement {
        profile: "runtime-packaging".to_owned(),
        required_metadata: vec![RuntimeMetadataRequirement::Pivot],
        triangle_budget_hint: Some(1_024),
    });
    let kit = scifi_industrial_style_kit();
    let implementation = crate_implementation();
    let style = scifi_industrial_style_implementation();
    let request = request("crate", "sci_fi_industrial", []);

    let baseline = instantiate_family(&family, &kit, &implementation, &style, &request)
        .expect("baseline should instantiate");
    let changed = instantiate_family(&export_changed, &kit, &implementation, &style, &request)
        .expect("export requirement changed should instantiate");

    assert_eq!(baseline.recipe.id, changed.recipe.id);
    assert_eq!(
        baseline.report.geometry_input_fingerprint,
        changed.report.geometry_input_fingerprint
    );
    assert_ne!(
        baseline.report.foundry_intent_fingerprint,
        changed.report.foundry_intent_fingerprint
    );
}

#[test]
fn base_recipe_placeholder_id_and_title_do_not_change_geometry_identity() {
    let family = crate_family();
    let kit = scifi_industrial_style_kit();
    let implementation = crate_implementation();
    let mut placeholder_changed = implementation.clone();
    placeholder_changed.base_recipe.id = AssetId(999);
    placeholder_changed.base_recipe.title = "Different placeholder".to_owned();
    let style = scifi_industrial_style_implementation();
    let request = request("crate", "sci_fi_industrial", []);

    let baseline = instantiate_family(&family, &kit, &implementation, &style, &request)
        .expect("baseline should instantiate");
    let changed = instantiate_family(&family, &kit, &placeholder_changed, &style, &request)
        .expect("placeholder changed should instantiate");

    assert_eq!(baseline.recipe.id, changed.recipe.id);
    assert_eq!(
        baseline.report.geometry_input_fingerprint,
        changed.report.geometry_input_fingerprint
    );
    assert_ne!(
        baseline.report.foundry_intent_fingerprint,
        changed.report.foundry_intent_fingerprint
    );
}

#[test]
fn asset_id_changes_when_selected_fragment_content_changes_without_version_bump() {
    let family = crate_family();
    let kit = scifi_industrial_style_kit();
    let implementation = crate_implementation();
    let request = request("crate", "sci_fi_industrial", []);
    let baseline_style = scifi_industrial_style_implementation();
    let mut changed_style = baseline_style.clone();
    let body = changed_style
        .prototypes
        .get_mut("armored_body")
        .expect("body fragment");
    let definition = body
        .recipe
        .definitions
        .get_mut(&LOCAL_DEFINITION)
        .expect("body definition");
    let GeometrySource::RoundedBox { half_extents, .. } = &mut definition.geometry.source else {
        panic!("expected rounded box body fragment");
    };
    half_extents[0] += 0.125;

    let baseline = instantiate_family(&family, &kit, &implementation, &baseline_style, &request)
        .expect("baseline crate");
    let changed = instantiate_family(&family, &kit, &implementation, &changed_style, &request)
        .expect("crate with content-only fragment change");

    assert_eq!(baseline_style.schema_version, changed_style.schema_version);
    assert_eq!(
        baseline_style.prototypes["armored_body"].schema_version,
        changed_style.prototypes["armored_body"].schema_version
    );
    assert_ne!(baseline.recipe.id, changed.recipe.id);
    assert_ne!(
        baseline.report.instantiation_fingerprint,
        changed.report.instantiation_fingerprint
    );
}

#[test]
fn incompatible_family_style_pair_is_rejected_before_geometry() {
    let family = crate_family();
    let error = instantiate_family(
        &family,
        &roman_timber_engineering_style_kit(),
        &crate_implementation(),
        &{
            let mut style = scifi_industrial_style_implementation();
            style.style_kit_id = "roman_timber_engineering".to_owned();
            style
        },
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
        default_value: Some(FamilyDefaultValue::Toggle(true)),
        execution_policy: ParameterExecutionPolicy::RequiredBinding,
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
fn presence_toggle_disables_selected_occurrence_subtree() {
    let mut family = crate_family();
    family.parameter_slots.push(FamilyParameterSlot {
        id: "handle_choice".to_owned(),
        label: "Handle Choice".to_owned(),
        target_role: Some("handle".to_owned()),
        kind: FamilyParameterKind::Choice(vec!["side".to_owned()]),
        range: None,
        default_value: Some(FamilyDefaultValue::Choice("side".to_owned())),
        execution_policy: ParameterExecutionPolicy::RequiredBinding,
        topology_changing: true,
    });
    let mut implementation = crate_implementation();
    implementation
        .parameter_bindings
        .push(ParameterBinding::ChoiceToPrototype {
            slot: "handle_choice".to_owned(),
            role: "handle".to_owned(),
            choices: BTreeMap::from([("side".to_owned(), "side_handle".to_owned())]),
        });
    let mut style = scifi_industrial_style_implementation();
    let handle = style
        .prototypes
        .get_mut("side_handle")
        .expect("handle fragment");
    handle.recipe.instances.insert(
        PartInstanceId(92),
        PartInstance {
            id: PartInstanceId(92),
            definition: LOCAL_DEFINITION,
            name: "handle child detail".to_owned(),
            parent: Some(LOCAL_INSTANCE),
            local_transform: Transform3::default(),
            attachment: None,
            enabled: true,
            tags: BTreeSet::from(["handle".to_owned(), "detail".to_owned()]),
            generated_by: None,
        },
    );
    handle.recipe.instances.insert(
        PartInstanceId(93),
        PartInstance {
            id: PartInstanceId(93),
            definition: LOCAL_DEFINITION,
            name: "handle internal helper root".to_owned(),
            parent: None,
            local_transform: Transform3::default(),
            attachment: None,
            enabled: true,
            tags: BTreeSet::from(["handle".to_owned(), "internal".to_owned()]),
            generated_by: None,
        },
    );
    handle.recipe.root_instances.push(PartInstanceId(93));
    handle.exports.internal_roots.push(PartInstanceId(93));
    handle.recipe.next_ids.part_instance = 94;

    let output = instantiate_family(
        &family,
        &scifi_industrial_style_kit(),
        &implementation,
        &style,
        &request(
            "crate",
            "sci_fi_industrial",
            [("has_handle", FamilyValue::Toggle(false))],
        ),
    )
    .expect("choice-selected optional handle can be disabled by presence toggle");

    assert_eq!(
        output.report.selected_providers.get("handle"),
        Some(&"side_handle".to_owned())
    );
    let handle_instances = output
        .recipe
        .instances
        .values()
        .filter(|instance| {
            instance.name.contains("side_handle")
                || instance.name == "handle child detail"
                || instance.name == "handle internal helper root"
        })
        .collect::<Vec<_>>();
    assert_eq!(handle_instances.len(), 3);
    assert!(handle_instances.iter().all(|instance| !instance.enabled));
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
fn invalid_fragment_export_lists_are_rejected_before_merge() {
    let mut style = scifi_industrial_style_implementation();
    style
        .prototypes
        .get_mut("armored_body")
        .expect("body fragment")
        .exports
        .role_occurrence_roots = vec![PartInstanceId(999)];

    let error = instantiate_family(
        &crate_family(),
        &scifi_industrial_style_kit(),
        &crate_implementation(),
        &style,
        &request("crate", "sci_fi_industrial", []),
    )
    .expect_err("bad fragment exports should fail implementation validation");

    let FamilyCompileError::ImplementationValidationFailed(report) = error else {
        panic!("expected implementation validation failure");
    };
    assert!(
        report
            .issues
            .iter()
            .any(|issue| { issue.code == "unknown_role_occurrence_root" })
    );
}

#[test]
fn nested_role_occurrence_roots_are_rejected_before_merge() {
    let mut style = scifi_industrial_style_implementation();
    let body = style
        .prototypes
        .get_mut("armored_body")
        .expect("body fragment");
    body.recipe.instances.insert(
        PartInstanceId(92),
        PartInstance {
            id: PartInstanceId(92),
            definition: LOCAL_DEFINITION,
            name: "nested body occurrence".to_owned(),
            parent: Some(LOCAL_INSTANCE),
            local_transform: Transform3::default(),
            attachment: None,
            enabled: true,
            tags: BTreeSet::from(["body".to_owned()]),
            generated_by: None,
        },
    );
    body.recipe.next_ids.part_instance = 93;
    body.exports.role_occurrence_roots = vec![LOCAL_INSTANCE, PartInstanceId(92)];

    let error = instantiate_family(
        &crate_family(),
        &scifi_industrial_style_kit(),
        &crate_implementation(),
        &style,
        &request("crate", "sci_fi_industrial", []),
    )
    .expect_err("nested exported roots should fail implementation validation");

    let FamilyCompileError::ImplementationValidationFailed(report) = error else {
        panic!("expected implementation validation failure");
    };
    assert!(issue_codes(&report).contains(&"nested_fragment_export_root"));
}

#[test]
fn internal_fragment_roots_cannot_overlap_exported_occurrences() {
    let mut style = scifi_industrial_style_implementation();
    let body = style
        .prototypes
        .get_mut("armored_body")
        .expect("body fragment");
    body.recipe.instances.insert(
        PartInstanceId(92),
        PartInstance {
            id: PartInstanceId(92),
            definition: LOCAL_DEFINITION,
            name: "body helper detail".to_owned(),
            parent: Some(LOCAL_INSTANCE),
            local_transform: Transform3::default(),
            attachment: None,
            enabled: true,
            tags: BTreeSet::from(["body".to_owned(), "helper".to_owned()]),
            generated_by: None,
        },
    );
    body.recipe.next_ids.part_instance = 93;
    body.exports.internal_roots = vec![PartInstanceId(92)];

    let error = instantiate_family(
        &crate_family(),
        &scifi_industrial_style_kit(),
        &crate_implementation(),
        &style,
        &request("crate", "sci_fi_industrial", []),
    )
    .expect_err("internal roots inside exported occurrence subtrees should fail");

    let FamilyCompileError::ImplementationValidationFailed(report) = error else {
        panic!("expected implementation validation failure");
    };
    assert!(issue_codes(&report).contains(&"internal_instance_overlaps_occurrence_root"));
}

#[test]
fn generated_provenance_must_reference_a_fragment_local_operation() {
    let mut style = scifi_industrial_style_implementation();
    style
        .prototypes
        .get_mut("armored_body")
        .expect("body fragment")
        .recipe
        .instances
        .get_mut(&LOCAL_INSTANCE)
        .expect("body root")
        .generated_by = Some(OperationId(777));

    let error = instantiate_family(
        &crate_family(),
        &scifi_industrial_style_kit(),
        &crate_implementation(),
        &style,
        &request("crate", "sci_fi_industrial", []),
    )
    .expect_err("generated provenance must target a local operation");

    let FamilyCompileError::FragmentRemap(
        shape_family_compile::remap::FragmentRemapError::ExternalReference { id_kind, id, .. },
    ) = error
    else {
        panic!("expected typed external-reference remap failure");
    };
    assert_eq!(id_kind, "operation");
    assert_eq!(id, "777");
}

#[test]
fn failed_family_fragment_remap_does_not_mutate_base_recipe_or_counters() {
    let implementation = crate_implementation();
    let before_base = implementation.base_recipe.clone();
    let mut invalid_style = scifi_industrial_style_implementation();
    invalid_style
        .prototypes
        .get_mut("armored_body")
        .expect("body fragment")
        .recipe
        .instances
        .get_mut(&LOCAL_INSTANCE)
        .expect("body root")
        .generated_by = Some(OperationId(777));

    instantiate_family(
        &crate_family(),
        &scifi_industrial_style_kit(),
        &implementation,
        &invalid_style,
        &request("crate", "sci_fi_industrial", []),
    )
    .expect_err("invalid remap should fail");

    assert_eq!(implementation.base_recipe, before_base);

    let valid = instantiate_family(
        &crate_family(),
        &scifi_industrial_style_kit(),
        &implementation,
        &scifi_industrial_style_implementation(),
        &request("crate", "sci_fi_industrial", []),
    )
    .expect("valid remap should still allocate from original base counters");
    assert_eq!(
        valid.recipe.definitions.keys().copied().collect::<Vec<_>>(),
        vec![
            PartDefinitionId(1),
            PartDefinitionId(2),
            PartDefinitionId(3)
        ]
    );
}

#[test]
fn required_parameter_slots_need_executable_bindings() {
    let mut family = crate_family();
    family.parameter_slots.push(FamilyParameterSlot {
        id: "ornament_density".to_owned(),
        label: "Ornament Density".to_owned(),
        target_role: Some("panel".to_owned()),
        kind: FamilyParameterKind::Ratio,
        range: Some(ParameterRange {
            minimum: 0.0,
            maximum: 1.0,
            step: 0.05,
        }),
        default_value: Some(FamilyDefaultValue::Scalar(0.5)),
        execution_policy: ParameterExecutionPolicy::RequiredBinding,
        topology_changing: false,
    });

    let error = instantiate_family(
        &family,
        &scifi_industrial_style_kit(),
        &crate_implementation(),
        &scifi_industrial_style_implementation(),
        &request("crate", "sci_fi_industrial", []),
    )
    .expect_err("required unbound parameter slot should fail implementation validation");

    let FamilyCompileError::ImplementationValidationFailed(report) = error else {
        panic!("expected implementation validation failure");
    };
    assert!(issue_codes(&report).contains(&"missing_required_parameter_binding"));
}

#[test]
fn advisory_and_runtime_parameter_slots_may_be_unbound() {
    let mut family = crate_family();
    family.parameter_slots.push(FamilyParameterSlot {
        id: "style_hint".to_owned(),
        label: "Style Hint".to_owned(),
        target_role: Some("panel".to_owned()),
        kind: FamilyParameterKind::Ratio,
        range: Some(ParameterRange {
            minimum: 0.0,
            maximum: 1.0,
            step: 0.05,
        }),
        default_value: Some(FamilyDefaultValue::Scalar(0.5)),
        execution_policy: ParameterExecutionPolicy::AdvisoryOnly,
        topology_changing: false,
    });
    family.parameter_slots.push(FamilyParameterSlot {
        id: "runtime_lod".to_owned(),
        label: "Runtime LOD".to_owned(),
        target_role: None,
        kind: FamilyParameterKind::Count,
        range: Some(ParameterRange {
            minimum: 1.0,
            maximum: 4.0,
            step: 1.0,
        }),
        default_value: Some(FamilyDefaultValue::Integer(2)),
        execution_policy: ParameterExecutionPolicy::RuntimeOnly,
        topology_changing: false,
    });

    instantiate_family(
        &family,
        &scifi_industrial_style_kit(),
        &crate_implementation(),
        &scifi_industrial_style_implementation(),
        &request("crate", "sci_fi_industrial", []),
    )
    .expect("advisory and runtime-only parameters should not require executable bindings");
}

#[test]
fn advisory_and_runtime_values_do_not_change_geometry_identity() {
    let mut family = crate_family();
    family.parameter_slots.push(FamilyParameterSlot {
        id: "style_hint".to_owned(),
        label: "Style Hint".to_owned(),
        target_role: Some("panel".to_owned()),
        kind: FamilyParameterKind::Ratio,
        range: Some(ParameterRange {
            minimum: 0.0,
            maximum: 1.0,
            step: 0.05,
        }),
        default_value: Some(FamilyDefaultValue::Scalar(0.5)),
        execution_policy: ParameterExecutionPolicy::AdvisoryOnly,
        topology_changing: false,
    });
    family.parameter_slots.push(FamilyParameterSlot {
        id: "runtime_lod".to_owned(),
        label: "Runtime LOD".to_owned(),
        target_role: None,
        kind: FamilyParameterKind::Count,
        range: Some(ParameterRange {
            minimum: 1.0,
            maximum: 4.0,
            step: 1.0,
        }),
        default_value: Some(FamilyDefaultValue::Integer(2)),
        execution_policy: ParameterExecutionPolicy::RuntimeOnly,
        topology_changing: false,
    });
    let mut baseline_request = request("crate", "sci_fi_industrial", []);
    baseline_request
        .parameters
        .insert("style_hint".to_owned(), FamilyValue::Scalar(0.2));
    baseline_request
        .parameters
        .insert("runtime_lod".to_owned(), FamilyValue::Integer(1));
    let mut advisory_changed = baseline_request.clone();
    advisory_changed
        .parameters
        .insert("style_hint".to_owned(), FamilyValue::Scalar(0.8));
    advisory_changed
        .parameters
        .insert("runtime_lod".to_owned(), FamilyValue::Integer(4));

    let baseline = instantiate_family(
        &family,
        &scifi_industrial_style_kit(),
        &crate_implementation(),
        &scifi_industrial_style_implementation(),
        &baseline_request,
    )
    .expect("baseline should instantiate");
    let changed = instantiate_family(
        &family,
        &scifi_industrial_style_kit(),
        &crate_implementation(),
        &scifi_industrial_style_implementation(),
        &advisory_changed,
    )
    .expect("advisory/runtime change should instantiate");

    assert_eq!(baseline.recipe.id, changed.recipe.id);
    assert_eq!(
        baseline.report.geometry_input_fingerprint,
        changed.report.geometry_input_fingerprint
    );
    assert_ne!(
        baseline.report.foundry_intent_fingerprint,
        changed.report.foundry_intent_fingerprint
    );
}

#[test]
fn non_required_parameter_bindings_are_rejected_as_non_executable() {
    let mut family = crate_family();
    family.parameter_slots.push(FamilyParameterSlot {
        id: "style_hint".to_owned(),
        label: "Style Hint".to_owned(),
        target_role: Some("body".to_owned()),
        kind: FamilyParameterKind::Ratio,
        range: Some(ParameterRange {
            minimum: 0.0,
            maximum: 1.0,
            step: 0.05,
        }),
        default_value: Some(FamilyDefaultValue::Scalar(0.5)),
        execution_policy: ParameterExecutionPolicy::AdvisoryOnly,
        topology_changing: false,
    });
    let mut implementation = crate_implementation();
    implementation
        .parameter_bindings
        .push(ParameterBinding::Scalar {
            slot: "style_hint".to_owned(),
            role: "body".to_owned(),
            local_path: definition_scalar_path(LOCAL_DEFINITION, "geometry.rounded_box.radius"),
            transform: ScalarTransform::Direct,
        });

    let error = instantiate_family(
        &family,
        &scifi_industrial_style_kit(),
        &implementation,
        &scifi_industrial_style_implementation(),
        &request("crate", "sci_fi_industrial", []),
    )
    .expect_err("advisory binding should fail implementation validation");

    let FamilyCompileError::ImplementationValidationFailed(report) = error else {
        panic!("expected implementation validation failure");
    };
    assert!(issue_codes(&report).contains(&"non_executable_parameter_binding"));
}

#[test]
fn conflicting_parameter_bindings_are_rejected() {
    let mut implementation = crate_implementation();
    implementation
        .parameter_bindings
        .push(ParameterBinding::TogglePartPresence {
            slot: "has_handle".to_owned(),
            role: "handle".to_owned(),
        });

    let error = instantiate_family(
        &crate_family(),
        &scifi_industrial_style_kit(),
        &implementation,
        &scifi_industrial_style_implementation(),
        &request("crate", "sci_fi_industrial", []),
    )
    .expect_err("duplicate presence binding should fail implementation validation");

    let FamilyCompileError::ImplementationValidationFailed(report) = error else {
        panic!("expected implementation validation failure");
    };
    assert!(issue_codes(&report).contains(&"conflicting_presence_binding"));
}

#[test]
fn conflicting_provider_selection_bindings_are_rejected() {
    let mut implementation = industrial_bridge_implementation();
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

    let error = instantiate_family(
        &bridge_family(&["industrial_steel"]),
        &industrial_bridge_style_kit(),
        &implementation,
        &industrial_bridge_style_implementation(),
        &request("bridge", "industrial_steel", []),
    )
    .expect_err("duplicate provider-selection binding should fail implementation validation");

    let FamilyCompileError::ImplementationValidationFailed(report) = error else {
        panic!("expected implementation validation failure");
    };
    assert!(issue_codes(&report).contains(&"conflicting_provider_selection_binding"));
}

#[test]
fn degenerate_scalar_transforms_are_rejected() {
    let mut implementation = crate_implementation();
    let ParameterBinding::Scalar { transform, .. } = &mut implementation.parameter_bindings[0]
    else {
        panic!("expected first crate binding to be scalar");
    };
    *transform = ScalarTransform::ScaleOffset {
        scale: 0.0,
        offset: 1.0,
    };

    let error = instantiate_family(
        &crate_family(),
        &scifi_industrial_style_kit(),
        &implementation,
        &scifi_industrial_style_implementation(),
        &request("crate", "sci_fi_industrial", []),
    )
    .expect_err("constant scalar transform should fail implementation validation");

    let FamilyCompileError::ImplementationValidationFailed(report) = error else {
        panic!("expected implementation validation failure");
    };
    assert!(issue_codes(&report).contains(&"degenerate_scalar_transform"));
}

#[test]
fn family_default_provider_is_not_shadowed_by_same_named_style_prototype() {
    let mut family = crate_family();
    family.part_roles[0].provision = RoleProvision::FamilyDefault;

    let mut kit = scifi_industrial_style_kit();
    kit.family_facets
        .get_mut("crate")
        .expect("crate facet")
        .part_prototypes
        .push(PartPrototype {
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
    style.default_role_providers.remove("body");
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
fn invalid_family_default_provider_role_provision_is_rejected() {
    let mut implementation = crate_implementation();
    implementation
        .default_role_providers
        .insert("body".to_owned(), "family_body".to_owned());
    implementation.fragments.insert(
        "family_body".to_owned(),
        rounded_box_fragment("family_body", "body", [0.33, 0.22, 0.11], 0.01),
    );

    let error = instantiate_family(
        &crate_family(),
        &scifi_industrial_style_kit(),
        &implementation,
        &scifi_industrial_style_implementation(),
        &request("crate", "sci_fi_industrial", []),
    )
    .expect_err("style-required body should not accept a family default entry");

    let FamilyCompileError::ImplementationValidationFailed(report) = error else {
        panic!("expected implementation validation failure");
    };
    assert!(
        report
            .issues
            .iter()
            .any(|issue| { issue.code == "family_default_provider_invalid_role_provision" })
    );
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
fn selected_fragment_metadata_is_remapped_instead_of_dropped() {
    let mut style = scifi_industrial_style_implementation();
    style
        .prototypes
        .get_mut("armored_body")
        .expect("body fragment")
        .recipe
        .instance_locks
        .insert(LOCAL_INSTANCE);

    let output = instantiate_family(
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
    .expect("selected fragment metadata should be remapped");

    let remapped_body = output
        .report
        .fragment_remaps
        .iter()
        .find(|report| report.fragment_id == "armored_body")
        .and_then(|report| report.remap.instances.get(&LOCAL_INSTANCE).copied())
        .expect("body root should be remapped");
    assert!(output.recipe.instance_locks.contains(&remapped_body));
    assert!(!output.recipe.instance_locks.contains(&LOCAL_INSTANCE));
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
                default_value: Some(FamilyDefaultValue::Scalar(3.0)),
                execution_policy: ParameterExecutionPolicy::RequiredBinding,
                topology_changing: false,
            },
            FamilyParameterSlot {
                id: "support_shape".to_owned(),
                label: "Support Shape".to_owned(),
                target_role: Some("support".to_owned()),
                kind: FamilyParameterKind::Choice(vec!["box".to_owned(), "round".to_owned()]),
                range: None,
                default_value: Some(FamilyDefaultValue::Choice("box".to_owned())),
                execution_policy: ParameterExecutionPolicy::RequiredBinding,
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
                default_value: Some(FamilyDefaultValue::Scalar(1.2)),
                execution_policy: ParameterExecutionPolicy::RequiredBinding,
                topology_changing: false,
            },
            FamilyParameterSlot {
                id: "has_handle".to_owned(),
                label: "Has Handle".to_owned(),
                target_role: Some("handle".to_owned()),
                kind: FamilyParameterKind::Toggle,
                range: None,
                default_value: Some(FamilyDefaultValue::Toggle(false)),
                execution_policy: ParameterExecutionPolicy::RequiredBinding,
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
                default_value: Some(FamilyDefaultValue::Integer(16)),
                execution_policy: ParameterExecutionPolicy::RequiredBinding,
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
                default_value: Some(FamilyDefaultValue::Scalar(0.5)),
                execution_policy: ParameterExecutionPolicy::RequiredBinding,
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
                default_value: Some(FamilyDefaultValue::Scalar(1.0)),
                execution_policy: ParameterExecutionPolicy::RequiredBinding,
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
    let part_prototypes = prototypes
        .iter()
        .map(|(id, display_name, role)| PartPrototype {
            id: (*id).to_owned(),
            display_name: (*display_name).to_owned(),
            role: (*role).to_owned(),
            operation_tags: vec![AllowedOperationKind::Primitive],
            style_tags: vec![id.replace('_', "-")],
        })
        .collect::<Vec<_>>();
    let family_id = compatible_families
        .first()
        .expect("test style kit must target one family")
        .to_string();
    StyleKit {
        schema_version: STYLE_KIT_SCHEMA_VERSION,
        id: id.to_owned(),
        display_name: display_name.to_owned(),
        compatible_families: compatible_families
            .iter()
            .map(|family| (*family).to_owned())
            .collect(),
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
        family_facets: BTreeMap::from([(
            family_id.clone(),
            FamilyStyleFacet {
                family_id,
                proportions: Vec::new(),
                part_prototypes,
                detail_modules: Vec::new(),
                policy_overrides: FamilyStylePolicyOverrides::default(),
            },
        )]),
        tags: vec![id.to_owned()],
    }
}

fn bridge_implementation() -> FamilyImplementation {
    FamilyImplementation {
        schema_version: FAMILY_IMPLEMENTATION_SCHEMA_VERSION,
        family_id: "bridge".to_owned(),
        base_recipe: AssetRecipe::new(AssetId(1), "Bridge base"),
        parameter_bindings: vec![
            ParameterBinding::Scalar {
                slot: "span_length".to_owned(),
                role: "span".to_owned(),
                local_path: definition_scalar_path(
                    LOCAL_DEFINITION,
                    "geometry.rounded_box.half_extents.x",
                ),
                transform: ScalarTransform::Direct,
            },
            ParameterBinding::ChoiceToPrototype {
                slot: "support_shape".to_owned(),
                role: "support".to_owned(),
                choices: BTreeMap::from([
                    ("box".to_owned(), "pointed_round_pile".to_owned()),
                    ("round".to_owned(), "pointed_round_pile".to_owned()),
                ]),
            },
        ],
        default_role_providers: BTreeMap::new(),
        fragments: BTreeMap::new(),
        attachment_bindings: Vec::new(),
    }
}

fn industrial_bridge_implementation() -> FamilyImplementation {
    let mut implementation = bridge_implementation();
    let ParameterBinding::ChoiceToPrototype { choices, .. } = implementation
        .parameter_bindings
        .iter_mut()
        .find(|binding| matches!(binding, ParameterBinding::ChoiceToPrototype { .. }))
        .expect("support shape binding")
    else {
        panic!("expected support shape binding");
    };
    *choices = BTreeMap::from([
        ("box".to_owned(), "box_support".to_owned()),
        ("round".to_owned(), "round_support".to_owned()),
    ]);
    implementation
}

fn crate_implementation() -> FamilyImplementation {
    FamilyImplementation {
        schema_version: FAMILY_IMPLEMENTATION_SCHEMA_VERSION,
        family_id: "crate".to_owned(),
        base_recipe: AssetRecipe::new(AssetId(1), "Crate base"),
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
        default_role_providers: BTreeMap::new(),
        fragments: BTreeMap::new(),
        attachment_bindings: Vec::new(),
    }
}

fn lamp_implementation() -> FamilyImplementation {
    FamilyImplementation {
        schema_version: FAMILY_IMPLEMENTATION_SCHEMA_VERSION,
        family_id: "lamp".to_owned(),
        base_recipe: AssetRecipe::new(AssetId(1), "Lamp base"),
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
        default_role_providers: BTreeMap::new(),
        fragments: BTreeMap::new(),
        attachment_bindings: Vec::new(),
    }
}

fn roman_timber_style_implementation() -> StyleImplementation {
    StyleImplementation {
        schema_version: STYLE_IMPLEMENTATION_SCHEMA_VERSION,
        style_kit_id: "roman_timber_engineering".to_owned(),
        family_id: "bridge".to_owned(),
        default_role_providers: BTreeMap::from([
            ("deck".to_owned(), "lashed_deck_plank".to_owned()),
            ("span".to_owned(), "hewn_span_beam".to_owned()),
            ("support".to_owned(), "pointed_round_pile".to_owned()),
        ]),
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
        schema_version: STYLE_IMPLEMENTATION_SCHEMA_VERSION,
        style_kit_id: "industrial_steel".to_owned(),
        family_id: "bridge".to_owned(),
        default_role_providers: BTreeMap::from([
            ("deck".to_owned(), "deck_plate".to_owned()),
            ("span".to_owned(), "box_span".to_owned()),
            ("support".to_owned(), "box_support".to_owned()),
        ]),
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
        schema_version: STYLE_IMPLEMENTATION_SCHEMA_VERSION,
        style_kit_id: "sci_fi_industrial".to_owned(),
        family_id: "crate".to_owned(),
        default_role_providers: BTreeMap::from([
            ("body".to_owned(), "armored_body".to_owned()),
            ("fastener".to_owned(), "bolt_head".to_owned()),
            ("handle".to_owned(), "side_handle".to_owned()),
            ("panel".to_owned(), "inset_panel".to_owned()),
        ]),
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
        schema_version: STYLE_IMPLEMENTATION_SCHEMA_VERSION,
        style_kit_id: "stylized_furniture".to_owned(),
        family_id: "lamp".to_owned(),
        default_role_providers: BTreeMap::from([
            ("base".to_owned(), "rounded_base".to_owned()),
            ("shade".to_owned(), "soft_shade".to_owned()),
            ("stem".to_owned(), "tapered_stem".to_owned()),
        ]),
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

fn add_socket_port(
    fragment: &mut RecipeFragment,
    port_id: &str,
    socket_role: &str,
    compatibility_tag: &str,
) {
    fragment
        .recipe
        .definitions
        .get_mut(&LOCAL_DEFINITION)
        .expect("fragment definition")
        .sockets
        .insert(
            SocketId(7),
            SocketSpec {
                id: SocketId(7),
                name: port_id.to_owned(),
                local_frame: Frame3::default(),
                role: socket_role.to_owned(),
                tags: BTreeSet::from([compatibility_tag.to_owned()]),
            },
        );
    fragment.exports.socket_ports.push(FragmentSocketPort {
        id: port_id.to_owned(),
        local_occurrence_root: LOCAL_INSTANCE,
        local_socket: SocketId(7),
        compatibility_tags: vec![compatibility_tag.to_owned()],
    });
    fragment.recipe.next_ids.socket = 8;
    assert!(validate_asset_recipe(&fragment.recipe).is_valid());
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
        schema_version: RECIPE_FRAGMENT_SCHEMA_VERSION,
        id: id.to_owned(),
        provided_role: role.to_owned(),
        exports: RecipeFragmentExports {
            role_occurrence_roots: vec![LOCAL_INSTANCE],
            internal_roots: Vec::new(),
            socket_ports: Vec::new(),
            surface_ports: Vec::new(),
        },
        recipe,
    }
}

fn issue_codes(report: &shape_family::FamilyValidationReport) -> Vec<&str> {
    report
        .issues
        .iter()
        .map(|issue| issue.code.as_str())
        .collect()
}

fn assert_numeric_remap<K>(map: &BTreeMap<K, K>, allocated: &[K], next_id: u64)
where
    K: Copy + Ord + std::fmt::Debug + TestIdNumber,
{
    assert_eq!(map.len(), allocated.len());
    let targets = map.values().copied().collect::<BTreeSet<_>>();
    let allocated = allocated.iter().copied().collect::<BTreeSet<_>>();
    assert_eq!(targets, allocated);
    let sources = map.keys().copied().collect::<BTreeSet<_>>();
    for (source, target) in map {
        assert_ne!(source.id_number(), target.id_number());
        assert!(!sources.contains(target));
        assert!(target.id_number() < next_id);
    }
}

trait TestIdNumber {
    fn id_number(self) -> u64;
}

impl TestIdNumber for PartDefinitionId {
    fn id_number(self) -> u64 {
        self.0
    }
}

impl TestIdNumber for PartInstanceId {
    fn id_number(self) -> u64 {
        self.0
    }
}

impl TestIdNumber for ParameterId {
    fn id_number(self) -> u64 {
        self.0
    }
}

impl TestIdNumber for OperationId {
    fn id_number(self) -> u64 {
        self.0
    }
}

impl TestIdNumber for RegionId {
    fn id_number(self) -> u64 {
        self.0
    }
}

impl TestIdNumber for BoundaryLoopId {
    fn id_number(self) -> u64 {
        self.0
    }
}

impl TestIdNumber for SocketId {
    fn id_number(self) -> u64 {
        self.0
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
