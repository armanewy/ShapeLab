use orchard_foundry::{
    PrimitiveProperty, PrimitivePropertyDomain, PrimitivePropertyValue, PrimitivePropertyValueKind,
    PrimitiveTopologyBehavior, box_primitive_property_schema, flat_panel_primitive_property_schema,
    kernel_kind_for_primitive_kind, primitive_default_property_values,
    primitive_property_descriptors_for_kind, sphere_primitive_property_schema,
    validate_primitive_property_schema, validate_primitive_property_values,
};

#[test]
fn primitive_property_box_schema_validates() {
    let schema = box_primitive_property_schema();

    assert!(validate_primitive_property_schema(&schema).is_valid());
    assert_eq!(
        property_labels(&schema),
        ["Width", "Depth", "Height", "Edge Softness"]
    );
    assert!(
        validate_primitive_property_values(&schema, &primitive_default_property_values(&schema))
            .is_valid()
    );
}

#[test]
fn primitive_property_flat_panel_schema_validates() {
    let schema = flat_panel_primitive_property_schema();

    assert!(validate_primitive_property_schema(&schema).is_valid());
    assert_eq!(
        property_labels(&schema),
        ["Width", "Height", "Thickness", "Edge Softness"]
    );
}

#[test]
fn primitive_property_sphere_schema_validates() {
    let schema = sphere_primitive_property_schema();

    assert!(validate_primitive_property_schema(&schema).is_valid());
    assert_eq!(
        property_labels(&schema),
        ["Width", "Height", "Depth", "Front Flatten", "Back Flatten"]
    );
}

#[test]
fn primitive_property_invalid_default_rejected() {
    let mut schema = box_primitive_property_schema();
    schema.properties[0].default_value = PrimitivePropertyValue::Length(99.0);

    let report = validate_primitive_property_schema(&schema);

    assert_issue(&report, "property_default_outside_domain");
}

#[test]
fn primitive_property_continuous_topology_change_rejected() {
    let mut schema = box_primitive_property_schema();
    schema.properties[0].topology_behavior = PrimitiveTopologyBehavior::DiscreteTopology;

    let report = validate_primitive_property_schema(&schema);

    assert_issue(&report, "topology_change_must_be_choice");
}

#[test]
fn primitive_property_raw_internal_term_in_display_label_rejected() {
    let mut schema = box_primitive_property_schema();
    schema.properties[0].display_name = "Provider Slot".to_owned();

    let report = validate_primitive_property_schema(&schema);

    assert_issue(&report, "invalid_primitive_property_label");
}

#[test]
fn primitive_property_invalid_current_state_rejected() {
    let schema = sphere_primitive_property_schema();
    let mut values = primitive_default_property_values(&schema);
    values.insert(
        "front_flatten".to_owned(),
        PrimitivePropertyValue::Ratio(1.25),
    );

    let report = validate_primitive_property_values(&schema, &values);

    assert_issue(&report, "invalid_current_property_value");
}

#[test]
fn primitive_property_serde_roundtrip_is_deterministic() {
    let schema = sphere_primitive_property_schema();
    let first = serde_json::to_string(&schema).expect("schema serializes");
    let decoded = serde_json::from_str(&first).expect("schema deserializes");
    let second =
        serde_json::to_string::<orchard_foundry::PrimitivePropertySchema>(&decoded).unwrap();

    assert_eq!(first, second);
    assert_eq!(decoded, schema);
}

#[test]
fn primitive_property_choice_topology_contract_is_valid() {
    let mut schema = box_primitive_property_schema();
    schema.properties.push(PrimitiveProperty {
        property_id: "corner_style".to_owned(),
        display_name: "Corner Style".to_owned(),
        value_kind: PrimitivePropertyValueKind::Choice,
        domain: PrimitivePropertyDomain::Choice {
            options: vec![
                orchard_foundry::PrimitiveChoiceOption {
                    choice_id: "soft".to_owned(),
                    display_name: "Soft".to_owned(),
                },
                orchard_foundry::PrimitiveChoiceOption {
                    choice_id: "crisp".to_owned(),
                    display_name: "Crisp".to_owned(),
                },
            ],
        },
        default_value: PrimitivePropertyValue::Choice("soft".to_owned()),
        affects_geometry: true,
        topology_behavior: PrimitiveTopologyBehavior::DiscreteTopology,
        user_facing_description: "Selects the primitive corner style.".to_owned(),
        advanced: true,
    });

    assert!(validate_primitive_property_schema(&schema).is_valid());
}

#[test]
fn primitive_property_descriptor_bridge_covers_box_schema() {
    let descriptors =
        primitive_property_descriptors_for_kind(orchard_foundry::PrimitiveKind::BoxPrimitive);

    assert_eq!(descriptors.len(), 4);
    assert_eq!(
        descriptors
            .iter()
            .find(|descriptor| descriptor.id == "box.width")
            .expect("box width descriptor")
            .control_family,
        orchard_asset::OrchardControlFamily::Stretch
    );
    assert_eq!(
        descriptors
            .iter()
            .find(|descriptor| descriptor.id == "box.edge_softness")
            .expect("box edge softness descriptor")
            .control_family,
        orchard_asset::OrchardControlFamily::Profile
    );
}

#[test]
fn primitive_property_descriptor_bridge_maps_every_current_schema_property() {
    for primitive_kind in [
        orchard_foundry::PrimitiveKind::BoxPrimitive,
        orchard_foundry::PrimitiveKind::FlatPanelPrimitive,
        orchard_foundry::PrimitiveKind::SpherePrimitive,
    ] {
        let schema = match primitive_kind {
            orchard_foundry::PrimitiveKind::BoxPrimitive => box_primitive_property_schema(),
            orchard_foundry::PrimitiveKind::FlatPanelPrimitive => {
                flat_panel_primitive_property_schema()
            }
            orchard_foundry::PrimitiveKind::SpherePrimitive => sphere_primitive_property_schema(),
            orchard_foundry::PrimitiveKind::CylinderPrimitive => unreachable!(),
        };
        let descriptors = primitive_property_descriptors_for_kind(primitive_kind);

        assert_eq!(descriptors.len(), schema.properties.len());
        assert!(kernel_kind_for_primitive_kind(primitive_kind).is_some());
        for property in &schema.properties {
            let descriptor_id = format!(
                "{}.{}",
                match primitive_kind {
                    orchard_foundry::PrimitiveKind::BoxPrimitive => "box",
                    orchard_foundry::PrimitiveKind::FlatPanelPrimitive => "flat_panel",
                    orchard_foundry::PrimitiveKind::SpherePrimitive => "sphere",
                    orchard_foundry::PrimitiveKind::CylinderPrimitive => unreachable!(),
                },
                property.property_id
            );
            let descriptor = descriptors
                .iter()
                .find(|descriptor| descriptor.id == descriptor_id)
                .unwrap_or_else(|| panic!("missing descriptor {descriptor_id}"));
            assert_eq!(
                descriptor.authoring_effect,
                orchard_asset::PropertyAuthoringEffect::SetProperty
            );
            assert!(!descriptor.affects.is_empty());
        }
    }
}

#[test]
fn primitive_property_descriptor_paths_are_semantic_not_raw() {
    let blocked = ["raw", "scalar", "mesh", "transform"];

    for primitive_kind in [
        orchard_foundry::PrimitiveKind::BoxPrimitive,
        orchard_foundry::PrimitiveKind::FlatPanelPrimitive,
        orchard_foundry::PrimitiveKind::SpherePrimitive,
    ] {
        for descriptor in primitive_property_descriptors_for_kind(primitive_kind) {
            let path = descriptor.path.to_ascii_lowercase();
            for term in blocked {
                assert!(
                    !path.contains(term),
                    "descriptor path {} exposes {term}",
                    descriptor.path
                );
            }
        }
    }
}

fn property_labels(schema: &orchard_foundry::PrimitivePropertySchema) -> Vec<&str> {
    schema
        .properties
        .iter()
        .map(|property| property.display_name.as_str())
        .collect()
}

fn assert_issue(report: &orchard_foundry::PrimitivePropertyValidationReport, expected_code: &str) {
    assert!(
        report
            .issues
            .iter()
            .any(|issue| issue.code == expected_code),
        "expected issue {expected_code}, got {:?}",
        report.issues
    );
}
