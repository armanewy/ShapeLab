//! Stylized lamp headless foundry fixture.

use std::collections::BTreeMap;

use shape_family::{AllowedOperationKind, RoleMultiplicity};
use shape_family_compile::{ParameterBinding, ScalarTransform};
use shape_foundry::ControlValue;

use crate::{
    FamilySchemaSpec, FixtureCatalogSpec, FoundryFixtureCatalog, build_fixture_catalog,
    continuous_control, cylinder_fragment, family_implementation, family_schema, length_slot,
    ratio_slot, role, style_implementation, style_kit,
};

/// Build the stylized lamp fixture catalog.
#[must_use]
pub fn fixture_catalog() -> FoundryFixtureCatalog {
    let family = family_schema(FamilySchemaSpec {
        id: "lamp",
        display_name: "Lamp",
        summary: "Theme-neutral desk lamp.",
        roles: vec![
            role("base", RoleMultiplicity::Single, true),
            role("stem", RoleMultiplicity::Single, true),
            role("shade", RoleMultiplicity::Single, true),
        ],
        allowed_operations: vec![
            AllowedOperationKind::Primitive,
            AllowedOperationKind::Array,
            AllowedOperationKind::Transform,
            AllowedOperationKind::Bevel,
        ],
        parameter_slots: vec![
            ratio_slot("shade_scale", "Shade Scale", "shade", 0.0, 1.0, 0.05, 0.5),
            length_slot("stem_height", "Stem Height", "stem", 0.4, 2.0, 0.05, 1.0),
        ],
        compatible_style_kits: vec!["stylized_furniture".to_owned()],
        tags: vec!["lighting".to_owned(), "stylized".to_owned()],
    });
    let style = style_kit(
        "stylized_furniture",
        "Stylized Furniture",
        "lamp",
        &[
            ("rounded_base", "Rounded base", "base"),
            ("tapered_stem", "Tapered stem", "stem"),
            ("soft_shade", "Soft shade", "shade"),
        ],
        vec!["furniture".to_owned(), "soft".to_owned()],
    );
    let family_impl = family_implementation(
        "lamp",
        "Stylized lamp base",
        vec![
            ParameterBinding::Scalar {
                slot: "shade_scale".to_owned(),
                role: "shade".to_owned(),
                local_path: shape_asset::definition_scalar_path(
                    crate::LOCAL_DEFINITION,
                    "geometry.cylinder.radius",
                ),
                transform: ScalarTransform::Ratio {
                    minimum: 0.22,
                    maximum: 0.52,
                },
            },
            ParameterBinding::Scalar {
                slot: "stem_height".to_owned(),
                role: "stem".to_owned(),
                local_path: shape_asset::definition_scalar_path(
                    crate::LOCAL_DEFINITION,
                    "geometry.cylinder.height",
                ),
                transform: ScalarTransform::ScaleOffset {
                    scale: 1.0,
                    offset: 0.1,
                },
            },
        ],
    );
    let style_impl = style_implementation(
        "stylized_furniture",
        "lamp",
        BTreeMap::from([
            ("base".to_owned(), "rounded_base".to_owned()),
            ("stem".to_owned(), "tapered_stem".to_owned()),
            ("shade".to_owned(), "soft_shade".to_owned()),
        ]),
        vec![
            cylinder_fragment(
                "rounded_base",
                "base",
                0.42,
                0.12,
                32,
                [0.0, -0.76, 0.0],
                Vec::new(),
            ),
            cylinder_fragment(
                "tapered_stem",
                "stem",
                0.06,
                1.0,
                20,
                [0.0, 0.0, 0.0],
                Vec::new(),
            ),
            cylinder_fragment(
                "soft_shade",
                "shade",
                0.34,
                0.44,
                32,
                [0.0, 0.92, 0.0],
                Vec::new(),
            ),
        ],
    );
    let profile = crate::customizer_profile(
        "lamp",
        "stylized_furniture",
        vec![
            continuous_control("shade_scale", "Shade Scale", "shade_scale", 0.5, 0.0, 1.0),
            continuous_control("stem_height", "Stem Height", "stem_height", 1.0, 0.4, 2.0),
        ],
    );

    build_fixture_catalog(FixtureCatalogSpec {
        slug: "stylized-lamp",
        document_id: "stylized-lamp-doc",
        family,
        style,
        family_implementation: family_impl,
        style_implementation: style_impl,
        customizer_profile: profile,
        control_state: BTreeMap::from([
            ("shade_scale".to_owned(), ControlValue::Scalar(0.75)),
            ("stem_height".to_owned(), ControlValue::Scalar(1.2)),
        ]),
    })
}
