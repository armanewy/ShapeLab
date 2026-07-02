//! Kernel registry and property descriptor bridge.

use orchard_asset::{
    KernelDescriptor, KernelKind, OrchardControlFamily, PropertyAffect, PropertyAuthoringEffect,
    PropertyDescriptor, PropertyDescriptorDomain, PropertyDescriptorValue,
    PropertyReviewImportance,
};

/// Return all current kernel descriptors.
#[must_use]
pub fn kernel_registry() -> Vec<KernelDescriptor> {
    vec![
        KernelDescriptor {
            kind: KernelKind::BoxPrimitive,
            display_name: "Box Primitive".to_owned(),
            beginner_description: "A bounded clay box with editable size and edge softness."
                .to_owned(),
            properties: vec![
                length_descriptor("box.width", "box.width", "Width", "Dimensions", 2.0),
                length_descriptor("box.depth", "box.depth", "Depth", "Dimensions", 1.4),
                length_descriptor("box.height", "box.height", "Height", "Dimensions", 1.0),
                profile_descriptor(
                    "box.edge_softness",
                    "box.edge_softness",
                    "Edge Softness",
                    "Profile",
                    0.08,
                    0.0,
                    0.35,
                ),
            ],
        },
        KernelDescriptor {
            kind: KernelKind::FlatPanelPrimitive,
            display_name: "Flat Panel Primitive".to_owned(),
            beginner_description: "A bounded upright panel with editable size and edge softness."
                .to_owned(),
            properties: vec![
                length_descriptor(
                    "flat_panel.width",
                    "flat_panel.width",
                    "Width",
                    "Dimensions",
                    1.8,
                ),
                length_descriptor(
                    "flat_panel.height",
                    "flat_panel.height",
                    "Height",
                    "Dimensions",
                    2.6,
                ),
                length_descriptor(
                    "flat_panel.thickness",
                    "flat_panel.thickness",
                    "Thickness",
                    "Dimensions",
                    0.18,
                ),
                profile_descriptor(
                    "flat_panel.edge_softness",
                    "flat_panel.edge_softness",
                    "Edge Softness",
                    "Profile",
                    0.05,
                    0.0,
                    0.3,
                ),
            ],
        },
        KernelDescriptor {
            kind: KernelKind::SpherePrimitive,
            display_name: "Sphere Primitive".to_owned(),
            beginner_description: "A bounded round clay form with editable size and flattening."
                .to_owned(),
            properties: vec![
                length_descriptor("sphere.width", "sphere.width", "Width", "Dimensions", 1.0),
                length_descriptor(
                    "sphere.height",
                    "sphere.height",
                    "Height",
                    "Dimensions",
                    1.0,
                ),
                length_descriptor("sphere.depth", "sphere.depth", "Depth", "Dimensions", 1.0),
                profile_descriptor(
                    "sphere.front_flatten",
                    "sphere.front_flatten",
                    "Front Flatten",
                    "Profile",
                    0.0,
                    0.0,
                    0.8,
                ),
                profile_descriptor(
                    "sphere.back_flatten",
                    "sphere.back_flatten",
                    "Back Flatten",
                    "Profile",
                    0.0,
                    0.0,
                    0.8,
                ),
            ],
        },
        KernelDescriptor {
            kind: KernelKind::PanelWithKnobComposition,
            display_name: "Panel with Knob".to_owned(),
            beginner_description:
                "A Flat Panel plus a knob-like Sphere form with bounded placement controls."
                    .to_owned(),
            properties: vec![
                length_descriptor(
                    "panel_knob.panel.width",
                    "panel_knob.panel.width",
                    "Panel Width",
                    "Panel",
                    1.8,
                ),
                length_descriptor(
                    "panel_knob.panel.height",
                    "panel_knob.panel.height",
                    "Panel Height",
                    "Panel",
                    2.6,
                ),
                length_descriptor(
                    "panel_knob.panel.thickness",
                    "panel_knob.panel.thickness",
                    "Panel Thickness",
                    "Panel",
                    0.18,
                ),
                profile_descriptor(
                    "panel_knob.panel.edge_softness",
                    "panel_knob.panel.edge_softness",
                    "Panel Edge Softness",
                    "Panel",
                    0.05,
                    0.0,
                    0.3,
                ),
                length_descriptor(
                    "panel_knob.knob.width",
                    "panel_knob.knob.width",
                    "Knob Width",
                    "Knob",
                    0.32,
                ),
                length_descriptor(
                    "panel_knob.knob.height",
                    "panel_knob.knob.height",
                    "Knob Height",
                    "Knob",
                    0.32,
                ),
                length_descriptor(
                    "panel_knob.knob.depth",
                    "panel_knob.knob.depth",
                    "Knob Depth",
                    "Knob",
                    0.18,
                ),
                profile_descriptor(
                    "panel_knob.knob.front_flatten",
                    "panel_knob.knob.front_flatten",
                    "Knob Front Flatten",
                    "Knob",
                    0.0,
                    0.0,
                    0.8,
                ),
                profile_descriptor(
                    "panel_knob.knob.back_flatten",
                    "panel_knob.knob.back_flatten",
                    "Knob Back Flatten",
                    "Knob",
                    0.45,
                    0.0,
                    0.8,
                ),
                attachment_descriptor(
                    "panel_knob.placement.horizontal",
                    "panel_knob.placement.horizontal",
                    "Knob Horizontal Position",
                    "Placement",
                    0.5,
                ),
                attachment_descriptor(
                    "panel_knob.placement.vertical",
                    "panel_knob.placement.vertical",
                    "Knob Vertical Position",
                    "Placement",
                    0.5,
                ),
            ],
        },
    ]
}

/// Find one kernel descriptor.
#[must_use]
pub fn kernel_descriptor(kind: KernelKind) -> Option<KernelDescriptor> {
    kernel_registry()
        .into_iter()
        .find(|descriptor| descriptor.kind == kind)
}

fn length_descriptor(
    id: &str,
    path: &str,
    label: &str,
    group: &str,
    default_value: f32,
) -> PropertyDescriptor {
    PropertyDescriptor {
        id: id.to_owned(),
        path: semantic_path(path),
        label: label.to_owned(),
        beginner_description: format!("Adjust {label}."),
        group: group.to_owned(),
        domain: PropertyDescriptorDomain::Length {
            minimum: 0.05,
            maximum: 5.0,
            step: 0.01,
        },
        default_value: PropertyDescriptorValue::Length(default_value),
        topology_changing: false,
        affects: vec![PropertyAffect::Dimensions],
        review_importance: PropertyReviewImportance::Primary,
        control_family: OrchardControlFamily::Stretch,
        authoring_effect: PropertyAuthoringEffect::SetProperty,
    }
}

fn profile_descriptor(
    id: &str,
    path: &str,
    label: &str,
    group: &str,
    default_value: f32,
    minimum: f32,
    maximum: f32,
) -> PropertyDescriptor {
    PropertyDescriptor {
        id: id.to_owned(),
        path: semantic_path(path),
        label: label.to_owned(),
        beginner_description: format!("Adjust {label}."),
        group: group.to_owned(),
        domain: PropertyDescriptorDomain::Ratio {
            minimum,
            maximum,
            step: 0.01,
        },
        default_value: PropertyDescriptorValue::Ratio(default_value),
        topology_changing: false,
        affects: vec![PropertyAffect::Profile],
        review_importance: PropertyReviewImportance::Primary,
        control_family: OrchardControlFamily::Profile,
        authoring_effect: PropertyAuthoringEffect::SetProperty,
    }
}

fn attachment_descriptor(
    id: &str,
    path: &str,
    label: &str,
    group: &str,
    default_value: f32,
) -> PropertyDescriptor {
    PropertyDescriptor {
        id: id.to_owned(),
        path: semantic_path(path),
        label: label.to_owned(),
        beginner_description: format!("Adjust {label}."),
        group: group.to_owned(),
        domain: PropertyDescriptorDomain::Ratio {
            minimum: 0.0,
            maximum: 1.0,
            step: 0.01,
        },
        default_value: PropertyDescriptorValue::Ratio(default_value),
        topology_changing: false,
        affects: vec![PropertyAffect::AttachmentPlacement],
        review_importance: PropertyReviewImportance::Primary,
        control_family: OrchardControlFamily::Attachment,
        authoring_effect: PropertyAuthoringEffect::SetProperty,
    }
}

fn semantic_path(path: &str) -> String {
    format!("kernel.{path}")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn property(kind: KernelKind, id: &str) -> PropertyDescriptor {
        kernel_descriptor(kind)
            .expect("kernel descriptor exists")
            .properties
            .into_iter()
            .find(|property| property.id == id)
            .unwrap_or_else(|| panic!("missing property {id}"))
    }

    #[test]
    fn kernel_registry_maps_box_dimensions_to_stretch() {
        for id in ["box.width", "box.depth", "box.height"] {
            assert_eq!(
                property(KernelKind::BoxPrimitive, id).control_family,
                OrchardControlFamily::Stretch
            );
        }
        assert_eq!(
            property(KernelKind::BoxPrimitive, "box.edge_softness").control_family,
            OrchardControlFamily::Profile
        );
    }

    #[test]
    fn kernel_registry_maps_flat_panel_dimensions_to_stretch() {
        for id in [
            "flat_panel.width",
            "flat_panel.height",
            "flat_panel.thickness",
        ] {
            assert_eq!(
                property(KernelKind::FlatPanelPrimitive, id).control_family,
                OrchardControlFamily::Stretch
            );
        }
    }

    #[test]
    fn kernel_registry_maps_sphere_properties_to_control_families() {
        for id in ["sphere.width", "sphere.height", "sphere.depth"] {
            assert_eq!(
                property(KernelKind::SpherePrimitive, id).control_family,
                OrchardControlFamily::Stretch
            );
        }
        for id in ["sphere.front_flatten", "sphere.back_flatten"] {
            assert_eq!(
                property(KernelKind::SpherePrimitive, id).control_family,
                OrchardControlFamily::Profile
            );
        }
    }

    #[test]
    fn kernel_registry_covers_panel_with_knob_composition() {
        let panel_knob = kernel_descriptor(KernelKind::PanelWithKnobComposition)
            .expect("panel with knob kernel exists");
        assert!(panel_knob.properties.iter().any(|p| p.group == "Panel"));
        assert!(panel_knob.properties.iter().any(|p| p.group == "Knob"));
        assert!(panel_knob.properties.iter().any(|p| {
            p.group == "Placement" && p.control_family == OrchardControlFamily::Attachment
        }));
    }

    #[test]
    fn kernel_registry_descriptors_have_domains_and_authoring_effects() {
        for kernel in kernel_registry() {
            assert!(!kernel.properties.is_empty());
            for property in kernel.properties {
                match property.domain {
                    PropertyDescriptorDomain::Length { .. }
                    | PropertyDescriptorDomain::Ratio { .. }
                    | PropertyDescriptorDomain::Boolean
                    | PropertyDescriptorDomain::Choice { .. }
                    | PropertyDescriptorDomain::Angle { .. } => {}
                }
                assert_eq!(
                    property.authoring_effect,
                    PropertyAuthoringEffect::SetProperty
                );
                assert!(!property.affects.is_empty());
            }
        }
    }

    #[test]
    fn kernel_registry_does_not_expose_raw_scalar_paths_to_ui() {
        for kernel in kernel_registry() {
            for property in kernel.properties {
                let path = property.path.to_ascii_lowercase();
                assert!(!path.contains("raw"));
                assert!(!path.contains("scalar"));
                assert!(!path.contains("mesh"));
                assert!(!path.contains("transform"));
            }
        }
    }
}
