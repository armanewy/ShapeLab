//! Roman bridge headless foundry fixture.

use std::collections::BTreeMap;

use shape_caesar_assets::style_kits::roman_timber_engineering_style_kit;
use shape_family::{AllowedOperationKind, RoleMultiplicity};
use shape_family_compile::{ParameterBinding, ScalarTransform};
use shape_foundry::ControlValue;

use crate::{
    FamilySchemaSpec, FixtureCatalogSpec, FoundryFixtureCatalog, build_fixture_catalog,
    choice_control, choice_slot, continuous_control, cylinder_fragment, family_implementation,
    family_schema, length_slot, linear_array, plate_fragment, role, rounded_box_fragment,
    style_implementation,
};

/// Build the Roman bridge fixture catalog.
#[must_use]
pub fn fixture_catalog() -> FoundryFixtureCatalog {
    let family = family_schema(FamilySchemaSpec {
        id: "bridge",
        display_name: "Bridge",
        summary: "Theme-neutral crossing structure for headless foundry builds.",
        roles: vec![
            role("support", RoleMultiplicity::Repeated, true),
            role("span", RoleMultiplicity::Single, true),
            role("deck", RoleMultiplicity::Single, true),
            role("brace", RoleMultiplicity::Optional, false),
            role("connector", RoleMultiplicity::Repeated, false),
        ],
        allowed_operations: vec![
            AllowedOperationKind::Primitive,
            AllowedOperationKind::Array,
            AllowedOperationKind::Transform,
            AllowedOperationKind::Lathe,
            AllowedOperationKind::Bevel,
        ],
        parameter_slots: vec![
            length_slot("span_length", "Span Length", "span", 1.8, 4.8, 0.1, 3.2),
            choice_slot(
                "support_shape",
                "Support Shape",
                "support",
                vec!["box".to_owned(), "round".to_owned()],
            ),
        ],
        compatible_style_kits: vec!["roman_timber_engineering".to_owned()],
        tags: vec!["roman".to_owned(), "bridge".to_owned()],
    });
    let style = roman_timber_engineering_style_kit();
    let family_impl = family_implementation(
        "bridge",
        "Roman bridge base",
        vec![
            ParameterBinding::Scalar {
                slot: "span_length".to_owned(),
                role: "span".to_owned(),
                local_path: shape_asset::definition_scalar_path(
                    crate::LOCAL_DEFINITION,
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
    );
    let style_impl = style_implementation(
        "roman_timber_engineering",
        "bridge",
        BTreeMap::from([
            ("support".to_owned(), "pointed_round_pile".to_owned()),
            ("deck".to_owned(), "lashed_deck_plank".to_owned()),
            ("span".to_owned(), "hewn_span_beam".to_owned()),
        ]),
        vec![
            cylinder_fragment(
                "pointed_round_pile",
                "support",
                0.14,
                1.0,
                18,
                [-1.25, -0.78, 0.0],
                vec![linear_array(1, 3, [1.25, 0.0, 0.0])],
            ),
            plate_fragment(
                "lashed_deck_plank",
                "deck",
                [3.0, 0.85],
                0.09,
                [0.0, 0.12, 0.0],
                Vec::new(),
            ),
            rounded_box_fragment(
                "hewn_span_beam",
                "span",
                [1.55, 0.11, 0.16],
                0.025,
                [0.0, -0.08, -0.18],
                vec![linear_array(1, 2, [0.0, 0.0, 0.36])],
            ),
        ],
    );
    let profile = crate::customizer_profile(
        "bridge",
        "roman_timber_engineering",
        vec![
            continuous_control("span_length", "Span Length", "span_length", 3.2, 1.8, 4.8),
            choice_control(
                "support_shape",
                "Support Shape",
                "support_shape",
                &["box", "round"],
            ),
        ],
    );

    build_fixture_catalog(FixtureCatalogSpec {
        slug: "roman-bridge",
        document_id: "roman-bridge-doc",
        family,
        style,
        family_implementation: family_impl,
        style_implementation: style_impl,
        customizer_profile: profile,
        control_state: BTreeMap::from([
            ("span_length".to_owned(), ControlValue::Scalar(3.4)),
            (
                "support_shape".to_owned(),
                ControlValue::Choice("round".to_owned()),
            ),
        ]),
    })
}
