//! Sci-fi crate headless foundry fixture.

use std::collections::BTreeMap;

use shape_family::{AllowedOperationKind, RoleMultiplicity};
use shape_family_compile::{ParameterBinding, ScalarTransform};
use shape_foundry::ControlValue;

use crate::{
    FamilySchemaSpec, FixtureCatalogSpec, FoundryFixtureCatalog, advisory_control,
    advisory_ratio_slot, build_fixture_catalog, continuous_control, count_slot, cylinder_fragment,
    family_implementation, family_schema, integer_control, length_slot, linear_array,
    plate_fragment, role, rounded_box_fragment, runtime_control, runtime_ratio_slot,
    style_implementation, style_kit, toggle_control, toggle_slot,
};

/// Build the sci-fi crate fixture catalog.
#[must_use]
pub fn fixture_catalog() -> FoundryFixtureCatalog {
    let family = family_schema(FamilySchemaSpec {
        id: "crate",
        display_name: "Crate",
        summary: "Theme-neutral hard-surface container.",
        roles: vec![
            role("body", RoleMultiplicity::Single, true),
            role("panel", RoleMultiplicity::Repeated, true),
            role("fastener", RoleMultiplicity::Repeated, true),
            role("handle", RoleMultiplicity::Optional, false),
        ],
        allowed_operations: vec![
            AllowedOperationKind::Primitive,
            AllowedOperationKind::Array,
            AllowedOperationKind::Transform,
            AllowedOperationKind::Bevel,
        ],
        parameter_slots: vec![
            length_slot("body_width", "Body Width", "body", 0.7, 2.0, 0.05, 1.2),
            toggle_slot("has_handle", "Has Handle", "handle", false),
            count_slot(
                "bolt_segments",
                "Bolt Segments",
                "fastener",
                8.0,
                32.0,
                1.0,
                16,
            ),
            runtime_ratio_slot("runtime_wear", "Runtime Wear", "body", 0.25),
            advisory_ratio_slot("advisory_weathering", "Advisory Weathering", "body", 0.2),
        ],
        compatible_style_kits: vec!["sci_fi_industrial".to_owned()],
        tags: vec!["crate".to_owned(), "hard_surface".to_owned()],
    });
    let style = style_kit(
        "sci_fi_industrial",
        "Sci-Fi Industrial",
        "crate",
        &[
            ("armored_body", "Armored body", "body"),
            ("inset_panel", "Inset panel", "panel"),
            ("bolt_head", "Bolt head", "fastener"),
            ("side_handle", "Side handle", "handle"),
        ],
        vec!["sci-fi".to_owned(), "industrial".to_owned()],
    );
    let family_impl = family_implementation(
        "crate",
        "Sci-fi crate base",
        vec![
            ParameterBinding::Scalar {
                slot: "body_width".to_owned(),
                role: "body".to_owned(),
                local_path: shape_asset::definition_scalar_path(
                    crate::LOCAL_DEFINITION,
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
                local_path: shape_asset::definition_scalar_path(
                    crate::LOCAL_DEFINITION,
                    "geometry.cylinder.radial_segments",
                ),
                transform: ScalarTransform::IntegerCount,
            },
        ],
    );
    let style_impl = style_implementation(
        "sci_fi_industrial",
        "crate",
        BTreeMap::from([
            ("body".to_owned(), "armored_body".to_owned()),
            ("panel".to_owned(), "inset_panel".to_owned()),
            ("fastener".to_owned(), "bolt_head".to_owned()),
            ("handle".to_owned(), "side_handle".to_owned()),
        ]),
        vec![
            rounded_box_fragment(
                "armored_body",
                "body",
                [1.0, 0.45, 0.65],
                0.08,
                [0.0, 0.0, 0.0],
                Vec::new(),
            ),
            plate_fragment(
                "inset_panel",
                "panel",
                [1.35, 0.42],
                0.045,
                [0.0, 0.0, 0.9],
                Vec::new(),
            ),
            cylinder_fragment(
                "bolt_head",
                "fastener",
                0.055,
                0.035,
                16,
                [-0.55, -0.36, 0.72],
                vec![linear_array(1, 6, [0.22, 0.0, 0.0])],
            ),
            rounded_box_fragment(
                "side_handle",
                "handle",
                [0.1, 0.08, 0.46],
                0.03,
                [1.55, 0.0, 0.0],
                Vec::new(),
            ),
        ],
    );
    let profile = crate::customizer_profile(
        "crate",
        "sci_fi_industrial",
        vec![
            continuous_control("body_width", "Body Width", "body_width", 1.2, 0.7, 2.0),
            toggle_control("has_handle", "Has Handle", "has_handle", true),
            integer_control("bolt_segments", "Bolt Segments", "bolt_segments", 20, 8, 32),
            runtime_control("runtime_wear", "Runtime Wear", "runtime_wear", 0.25),
            advisory_control(
                "advisory_weathering",
                "Advisory Weathering",
                "advisory_weathering",
                0.2,
            ),
        ],
    );

    build_fixture_catalog(FixtureCatalogSpec {
        slug: "sci-fi-crate",
        document_id: "sci-fi-crate-doc",
        family,
        style,
        family_implementation: family_impl,
        style_implementation: style_impl,
        customizer_profile: profile,
        control_state: BTreeMap::from([
            ("body_width".to_owned(), ControlValue::Scalar(1.35)),
            ("has_handle".to_owned(), ControlValue::Toggle(true)),
            ("bolt_segments".to_owned(), ControlValue::Integer(20)),
            ("runtime_wear".to_owned(), ControlValue::Scalar(0.25)),
            ("advisory_weathering".to_owned(), ControlValue::Scalar(0.2)),
        ]),
    })
}
