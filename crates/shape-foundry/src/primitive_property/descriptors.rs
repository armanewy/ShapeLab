
fn required_property_ids(primitive_kind: PrimitiveKind) -> &'static [&'static str] {
    match primitive_kind {
        PrimitiveKind::BoxPrimitive => &["width", "depth", "height", "edge_softness"],
        PrimitiveKind::FlatPanelPrimitive => &["width", "height", "thickness", "edge_softness"],
        PrimitiveKind::SpherePrimitive => {
            &["width", "height", "depth", "front_flatten", "back_flatten"]
        }
        PrimitiveKind::CylinderPrimitive => &["radius", "height", "sides", "edge_softness"],
    }
}

fn primitive_property_schema_for_kind(primitive_kind: PrimitiveKind) -> PrimitivePropertySchema {
    match primitive_kind {
        PrimitiveKind::BoxPrimitive => box_primitive_property_schema(),
        PrimitiveKind::FlatPanelPrimitive => flat_panel_primitive_property_schema(),
        PrimitiveKind::SpherePrimitive => sphere_primitive_property_schema(),
        PrimitiveKind::CylinderPrimitive => unreachable!("Cylinder is not product-active"),
    }
}

fn primitive_property_descriptor(
    schema: &PrimitivePropertySchema,
    property: &PrimitiveProperty,
) -> PropertyDescriptor {
    let control_family = control_family_for_property(&property.property_id, property.value_kind);
    let affects = affects_for_control_family(control_family);
    PropertyDescriptor {
        id: format!(
            "{}.{}",
            descriptor_prefix(schema.primitive_kind),
            property.property_id
        ),
        path: format!(
            "primitive.{}.{}",
            descriptor_prefix(schema.primitive_kind),
            property.property_id
        ),
        label: property.display_name.clone(),
        beginner_description: property.user_facing_description.clone(),
        group: group_for_control_family(control_family).to_owned(),
        domain: descriptor_domain(&property.domain),
        default_value: descriptor_value(&property.default_value),
        topology_changing: property.topology_behavior
            == PrimitiveTopologyBehavior::DiscreteTopology,
        affects,
        review_importance: if property.advanced {
            PropertyReviewImportance::Advanced
        } else {
            PropertyReviewImportance::Primary
        },
        control_family,
        authoring_effect: PropertyAuthoringEffect::SetProperty,
    }
}

fn descriptor_prefix(primitive_kind: PrimitiveKind) -> &'static str {
    match primitive_kind {
        PrimitiveKind::BoxPrimitive => "box",
        PrimitiveKind::FlatPanelPrimitive => "flat_panel",
        PrimitiveKind::SpherePrimitive => "sphere",
        PrimitiveKind::CylinderPrimitive => "cylinder",
    }
}

/// Convert a primitive kind into the shared kernel kind when product-active.
#[must_use]
pub fn kernel_kind_for_primitive_kind(primitive_kind: PrimitiveKind) -> Option<KernelKind> {
    match primitive_kind {
        PrimitiveKind::BoxPrimitive => Some(KernelKind::BoxPrimitive),
        PrimitiveKind::FlatPanelPrimitive => Some(KernelKind::FlatPanelPrimitive),
        PrimitiveKind::SpherePrimitive => Some(KernelKind::SpherePrimitive),
        PrimitiveKind::CylinderPrimitive => None,
    }
}

fn control_family_for_property(
    property_id: &str,
    value_kind: PrimitivePropertyValueKind,
) -> OrchardControlFamily {
    match property_id {
        "width" | "depth" | "height" | "thickness" | "radius" => OrchardControlFamily::Stretch,
        "edge_softness" | "front_flatten" | "back_flatten" => OrchardControlFamily::Profile,
        _ => match value_kind {
            PrimitivePropertyValueKind::Choice | PrimitivePropertyValueKind::Boolean => {
                OrchardControlFamily::Option
            }
            PrimitivePropertyValueKind::Angle => OrchardControlFamily::Attachment,
            PrimitivePropertyValueKind::Length | PrimitivePropertyValueKind::Ratio => {
                OrchardControlFamily::Stretch
            }
        },
    }
}

fn affects_for_control_family(control_family: OrchardControlFamily) -> Vec<PropertyAffect> {
    match control_family {
        OrchardControlFamily::Stretch => vec![PropertyAffect::Dimensions],
        OrchardControlFamily::Profile => vec![PropertyAffect::Profile],
        OrchardControlFamily::Attachment => vec![PropertyAffect::AttachmentPlacement],
        OrchardControlFamily::Band
        | OrchardControlFamily::Pattern
        | OrchardControlFamily::Option => {
            vec![PropertyAffect::Composition]
        }
    }
}

fn group_for_control_family(control_family: OrchardControlFamily) -> &'static str {
    match control_family {
        OrchardControlFamily::Stretch => "Dimensions",
        OrchardControlFamily::Profile => "Profile",
        OrchardControlFamily::Attachment => "Placement",
        OrchardControlFamily::Band => "Band",
        OrchardControlFamily::Pattern => "Pattern",
        OrchardControlFamily::Option => "Options",
    }
}

fn descriptor_domain(domain: &PrimitivePropertyDomain) -> PropertyDescriptorDomain {
    match domain {
        PrimitivePropertyDomain::Length {
            minimum,
            maximum,
            step,
        } => PropertyDescriptorDomain::Length {
            minimum: *minimum,
            maximum: *maximum,
            step: *step,
        },
        PrimitivePropertyDomain::Ratio {
            minimum,
            maximum,
            step,
        } => PropertyDescriptorDomain::Ratio {
            minimum: *minimum,
            maximum: *maximum,
            step: *step,
        },
        PrimitivePropertyDomain::Boolean => PropertyDescriptorDomain::Boolean,
        PrimitivePropertyDomain::Choice { options } => PropertyDescriptorDomain::Choice {
            options: options
                .iter()
                .map(|option| option.choice_id.clone())
                .collect(),
        },
        PrimitivePropertyDomain::Angle {
            minimum_degrees,
            maximum_degrees,
            step_degrees,
        } => PropertyDescriptorDomain::Angle {
            minimum_degrees: *minimum_degrees,
            maximum_degrees: *maximum_degrees,
            step_degrees: *step_degrees,
        },
    }
}

fn descriptor_value(value: &PrimitivePropertyValue) -> PropertyDescriptorValue {
    match value {
        PrimitivePropertyValue::Length(value) => PropertyDescriptorValue::Length(*value),
        PrimitivePropertyValue::Ratio(value) => PropertyDescriptorValue::Ratio(*value),
        PrimitivePropertyValue::Boolean(value) => PropertyDescriptorValue::Boolean(*value),
        PrimitivePropertyValue::Choice(value) => PropertyDescriptorValue::Choice(value.clone()),
        PrimitivePropertyValue::Angle(value) => PropertyDescriptorValue::Angle(*value),
    }
}

fn length_property(property_id: &str, display_name: &str, default_value: f32) -> PrimitiveProperty {
    PrimitiveProperty {
        property_id: property_id.to_owned(),
        display_name: display_name.to_owned(),
        value_kind: PrimitivePropertyValueKind::Length,
        domain: PrimitivePropertyDomain::Length {
            minimum: 0.05,
            maximum: 6.0,
            step: 0.01,
        },
        default_value: PrimitivePropertyValue::Length(default_value),
        affects_geometry: true,
        topology_behavior: PrimitiveTopologyBehavior::GeometryPreserving,
        user_facing_description: format!("Adjusts the primitive {display_name}."),
        advanced: false,
    }
}

fn ratio_property(
    property_id: &str,
    display_name: &str,
    default_value: f32,
    minimum: f32,
    maximum: f32,
) -> PrimitiveProperty {
    PrimitiveProperty {
        property_id: property_id.to_owned(),
        display_name: display_name.to_owned(),
        value_kind: PrimitivePropertyValueKind::Ratio,
        domain: PrimitivePropertyDomain::Ratio {
            minimum,
            maximum,
            step: 0.01,
        },
        default_value: PrimitivePropertyValue::Ratio(default_value),
        affects_geometry: true,
        topology_behavior: PrimitiveTopologyBehavior::GeometryPreserving,
        user_facing_description: format!("Adjusts the primitive {display_name}."),
        advanced: false,
    }
}

fn constraint(
    constraint_id: &str,
    display_name: &str,
    user_facing_description: &str,
) -> PrimitiveSchemaConstraint {
    PrimitiveSchemaConstraint {
        constraint_id: constraint_id.to_owned(),
        display_name: display_name.to_owned(),
        user_facing_description: user_facing_description.to_owned(),
    }
}

fn direct_preview_policy() -> PrimitivePreviewPolicy {
    PrimitivePreviewPolicy {
        preserve_previous_valid_preview: true,
        continuous_preview_allowed: true,
        user_facing_description: "Keep the last valid primitive preview while updates compile."
            .to_owned(),
    }
}

fn direct_export_policy(user_facing_description: &str) -> PrimitiveExportPolicy {
    PrimitiveExportPolicy {
        export_current_primitive: true,
        user_facing_description: user_facing_description.to_owned(),
        limitations: vec![
            "This is not a textured, rigged, animated, or game-ready package.".to_owned(),
        ],
    }
}

fn contains_internal_term(value: &str) -> bool {
    let lower = value.to_ascii_lowercase();
    [
        "provider",
        "scalar path",
        "slot",
        "operation id",
        "internal",
        "recipe",
        "raw transform",
    ]
    .iter()
    .any(|term| lower.contains(term))
}

fn contains_blender_like_term(value: &str) -> bool {
    let lower = value.to_ascii_lowercase();
    [
        "vertex",
        "vertices",
        "face",
        "faces",
        "loop",
        "loops",
        "cage",
        "boolean",
        "sculpt",
        "mesh transform",
    ]
    .iter()
    .any(|term| lower.contains(term))
}
