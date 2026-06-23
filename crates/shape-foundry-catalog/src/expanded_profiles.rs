//! Wave 26 expanded Foundry fixture profiles.

use std::collections::BTreeMap;

use shape_asset::GeometrySource;
use shape_family::{AllowedOperationKind, RoleMultiplicity};
use shape_family_compile::{ParameterBinding, RecipeFragment, ScalarTransform};
use shape_foundry::{ControlValue, CustomizerControl};

use crate::{
    FamilySchemaSpec, FixtureCatalogSpec, FoundryFixtureCatalog, build_fixture_catalog,
    choice_control, choice_slot, continuous_control, count_slot, family_implementation,
    family_schema, fragment, integer_control, linear_array, ratio_slot, role, style_implementation,
    style_kit, toggle_control, toggle_slot,
};

#[derive(Clone)]
struct ExpansionProfileSpec {
    slug: &'static str,
    document_id: &'static str,
    family_id: &'static str,
    family_name: &'static str,
    family_summary: &'static str,
    style_id: &'static str,
    style_name: &'static str,
    core_role: &'static str,
    accent_role: &'static str,
    detail_role: &'static str,
    accessory_role: &'static str,
    tags: &'static [&'static str],
    core_variants: Vec<FragmentSpec>,
    accent_variants: Vec<FragmentSpec>,
    detail_fragment: FragmentSpec,
    accessory_fragment: FragmentSpec,
}

#[derive(Clone, Copy)]
struct FragmentSpec {
    id: &'static str,
    label: &'static str,
    role: &'static str,
    shape: FragmentShape,
    translation: [f32; 3],
    array: Option<ArraySpec>,
}

#[derive(Clone, Copy)]
struct ArraySpec {
    count: u32,
    offset: [f32; 3],
}

#[derive(Clone, Copy)]
struct CylinderSpec {
    radius: f32,
    height: f32,
    radial_segments: u32,
}

#[derive(Clone, Copy)]
enum FragmentShape {
    RoundedBox {
        half_extents: [f32; 3],
        radius: f32,
    },
    Plate {
        size: [f32; 2],
        thickness: f32,
    },
    Cylinder {
        radius: f32,
        height: f32,
        radial_segments: u32,
    },
}

/// Build the Market Stall fixture catalog.
#[must_use]
pub fn market_stall_fixture_catalog() -> FoundryFixtureCatalog {
    expansion_fixture(ExpansionProfileSpec {
        slug: "market-stall",
        document_id: "market-stall-doc",
        family_id: "market_stall",
        family_name: "Market Stall",
        family_summary: "Small architectural kit with frame, awning, goods, and sign accessory.",
        style_id: "timber_canvas_market",
        style_name: "Timber Canvas Market",
        core_role: "frame",
        accent_role: "awning",
        detail_role: "goods",
        accessory_role: "sign",
        tags: &["market", "stall", "architecture"],
        core_variants: vec![
            rounded(
                "open_timber_frame",
                "Open timber frame",
                "frame",
                [0.95, 0.72, 0.42],
                0.035,
                [0.0, 0.72, 0.0],
            ),
            rounded(
                "boxy_counter_frame",
                "Boxy counter frame",
                "frame",
                [0.88, 0.62, 0.48],
                0.045,
                [0.0, 0.62, 0.0],
            ),
            rounded(
                "tall_booth_frame",
                "Tall booth frame",
                "frame",
                [0.82, 0.92, 0.44],
                0.035,
                [0.0, 0.92, 0.0],
            ),
        ],
        accent_variants: vec![
            plate(
                "flat_canvas_awning",
                "Flat canvas awning",
                "awning",
                [1.25, 0.62],
                0.045,
                [0.0, 1.58, 0.08],
            ),
            plate(
                "deep_canvas_awning",
                "Deep canvas awning",
                "awning",
                [1.38, 0.74],
                0.055,
                [0.0, 1.6, 0.12],
            ),
            plate(
                "narrow_canvas_awning",
                "Narrow canvas awning",
                "awning",
                [1.08, 0.52],
                0.04,
                [0.0, 1.56, 0.04],
            ),
        ],
        detail_fragment: rounded_array(
            "produce_bins",
            "Produce bins",
            "goods",
            [0.1, 0.08, 0.16],
            0.015,
            [-0.56, 0.78, 0.7],
            array(5, [0.28, 0.0, 0.0]),
        ),
        accessory_fragment: plate(
            "hanging_price_sign",
            "Hanging price sign",
            "sign",
            [0.34, 0.22],
            0.025,
            [0.0, 1.24, 0.75],
        ),
    })
}

/// Build the sci-fi door-panel fixture catalog.
#[must_use]
pub fn scifi_door_fixture_catalog() -> FoundryFixtureCatalog {
    expansion_fixture(ExpansionProfileSpec {
        slug: "sci-fi-door",
        document_id: "sci-fi-door-doc",
        family_id: "door_panel",
        family_name: "Door Panel",
        family_summary: "Hard-surface doorway panel with frame, insert, bolts, and keypad accessory.",
        style_id: "sci_fi_panel_system",
        style_name: "Sci-Fi Panel System",
        core_role: "frame",
        accent_role: "insert",
        detail_role: "fastener",
        accessory_role: "keypad",
        tags: &["sci-fi", "door", "panel"],
        core_variants: vec![
            rounded(
                "single_slab_frame",
                "Single slab frame",
                "frame",
                [0.68, 1.05, 0.08],
                0.035,
                [0.0, 1.05, 0.0],
            ),
            rounded(
                "wide_blast_frame",
                "Wide blast frame",
                "frame",
                [0.86, 1.0, 0.09],
                0.045,
                [0.0, 1.0, 0.0],
            ),
            rounded(
                "tall_bulkhead_frame",
                "Tall bulkhead frame",
                "frame",
                [0.62, 1.22, 0.1],
                0.04,
                [0.0, 1.22, 0.0],
            ),
        ],
        accent_variants: vec![
            rounded(
                "center_recess_insert",
                "Center recess insert",
                "insert",
                [0.42, 0.78, 0.045],
                0.025,
                [0.0, 1.05, 0.16],
            ),
            rounded(
                "split_armor_insert",
                "Split armor insert",
                "insert",
                [0.52, 0.68, 0.05],
                0.02,
                [0.0, 1.02, 0.17],
            ),
            rounded(
                "warning_stripe_insert",
                "Warning stripe insert",
                "insert",
                [0.48, 0.84, 0.035],
                0.015,
                [0.0, 1.08, 0.16],
            ),
        ],
        detail_fragment: cylinder_array(
            "frame_bolts",
            "Frame bolts",
            "fastener",
            cylinder_spec(0.045, 0.035, 12),
            [-0.56, 0.38, 0.28],
            array(6, [0.225, 0.0, 0.0]),
        ),
        accessory_fragment: rounded(
            "side_keypad",
            "Side keypad",
            "keypad",
            [0.11, 0.18, 0.035],
            0.012,
            [0.72, 1.02, 0.25],
        ),
    })
}

/// Build the coopered storage barrel fixture catalog.
#[must_use]
pub fn barrel_fixture_catalog() -> FoundryFixtureCatalog {
    expansion_fixture(ExpansionProfileSpec {
        slug: "storage-barrel",
        document_id: "storage-barrel-doc",
        family_id: "storage_barrel",
        family_name: "Storage Barrel",
        family_summary: "Storage prop with rounded body, hoops, stave detail, and side label.",
        style_id: "coopered_storage",
        style_name: "Coopered Storage",
        core_role: "body",
        accent_role: "hoop",
        detail_role: "stave",
        accessory_role: "label",
        tags: &["barrel", "storage", "prop"],
        core_variants: vec![
            rounded(
                "squat_barrel_body",
                "Squat barrel body",
                "body",
                [0.42, 0.58, 0.42],
                0.12,
                [0.0, 0.58, 0.0],
            ),
            rounded(
                "tall_barrel_body",
                "Tall barrel body",
                "body",
                [0.36, 0.76, 0.36],
                0.11,
                [0.0, 0.76, 0.0],
            ),
            rounded(
                "wide_cask_body",
                "Wide cask body",
                "body",
                [0.52, 0.55, 0.42],
                0.13,
                [0.0, 0.55, 0.0],
            ),
        ],
        accent_variants: vec![
            rounded(
                "thin_iron_hoop",
                "Thin iron hoop",
                "hoop",
                [0.5, 0.035, 0.48],
                0.018,
                [0.0, 1.22, 0.0],
            ),
            rounded(
                "double_iron_hoop",
                "Double iron hoop",
                "hoop",
                [0.53, 0.045, 0.5],
                0.02,
                [0.0, 1.24, 0.0],
            ),
            rounded(
                "rope_hoop",
                "Rope hoop",
                "hoop",
                [0.49, 0.055, 0.47],
                0.025,
                [0.0, 1.23, 0.0],
            ),
        ],
        detail_fragment: rounded_array(
            "vertical_staves",
            "Vertical staves",
            "stave",
            [0.035, 0.45, 0.028],
            0.006,
            [-0.33, 0.58, 0.5],
            array(7, [0.11, 0.0, 0.0]),
        ),
        accessory_fragment: plate(
            "painted_storage_label",
            "Painted storage label",
            "label",
            [0.28, 0.2],
            0.018,
            [0.0, 0.62, 0.7],
        ),
    })
}

/// Build the wayfinding signpost fixture catalog.
#[must_use]
pub fn signpost_fixture_catalog() -> FoundryFixtureCatalog {
    expansion_fixture(ExpansionProfileSpec {
        slug: "signpost",
        document_id: "signpost-doc",
        family_id: "signpost",
        family_name: "Signpost",
        family_summary: "Environment prop with post, directional boards, pegs, and hanging marker.",
        style_id: "wayfinding_timber",
        style_name: "Wayfinding Timber",
        core_role: "post",
        accent_role: "board",
        detail_role: "peg",
        accessory_role: "marker",
        tags: &["signpost", "environment", "prop"],
        core_variants: vec![
            rounded(
                "straight_post",
                "Straight post",
                "post",
                [0.08, 0.9, 0.08],
                0.025,
                [0.0, 0.9, 0.0],
            ),
            rounded(
                "chunky_post",
                "Chunky post",
                "post",
                [0.11, 0.82, 0.1],
                0.03,
                [0.0, 0.82, 0.0],
            ),
            rounded(
                "tall_post",
                "Tall post",
                "post",
                [0.075, 1.1, 0.075],
                0.022,
                [0.0, 1.1, 0.0],
            ),
        ],
        accent_variants: vec![
            rounded(
                "left_arrow_board",
                "Left arrow board",
                "board",
                [0.45, 0.09, 0.045],
                0.012,
                [-0.28, 1.35, 0.18],
            ),
            rounded(
                "right_arrow_board",
                "Right arrow board",
                "board",
                [0.45, 0.09, 0.045],
                0.012,
                [0.28, 1.2, 0.18],
            ),
            rounded(
                "stacked_board",
                "Stacked board",
                "board",
                [0.38, 0.12, 0.05],
                0.014,
                [0.0, 1.3, 0.18],
            ),
        ],
        detail_fragment: cylinder_array(
            "board_pegs",
            "Board pegs",
            "peg",
            cylinder_spec(0.025, 0.035, 10),
            [-0.34, 1.2, 0.28],
            array(4, [0.23, 0.0, 0.0]),
        ),
        accessory_fragment: plate(
            "hanging_marker",
            "Hanging marker",
            "marker",
            [0.18, 0.24],
            0.018,
            [0.0, 0.82, 0.35],
        ),
    })
}

/// Build the workshop chair fixture catalog.
#[must_use]
pub fn chair_fixture_catalog() -> FoundryFixtureCatalog {
    expansion_fixture(ExpansionProfileSpec {
        slug: "workshop-chair",
        document_id: "workshop-chair-doc",
        family_id: "chair",
        family_name: "Chair",
        family_summary: "Furniture prop with seat, back, leg detail, and optional cushion.",
        style_id: "workshop_furniture",
        style_name: "Workshop Furniture",
        core_role: "seat",
        accent_role: "back",
        detail_role: "leg",
        accessory_role: "cushion",
        tags: &["chair", "furniture", "prop"],
        core_variants: vec![
            rounded(
                "square_seat",
                "Square seat",
                "seat",
                [0.42, 0.08, 0.42],
                0.035,
                [0.0, 0.52, 0.0],
            ),
            rounded(
                "wide_seat",
                "Wide seat",
                "seat",
                [0.52, 0.075, 0.4],
                0.035,
                [0.0, 0.52, 0.0],
            ),
            rounded(
                "compact_stool_seat",
                "Compact stool seat",
                "seat",
                [0.34, 0.07, 0.34],
                0.04,
                [0.0, 0.5, 0.0],
            ),
        ],
        accent_variants: vec![
            rounded(
                "plain_back",
                "Plain back",
                "back",
                [0.42, 0.38, 0.055],
                0.025,
                [0.0, 0.92, -0.55],
            ),
            rounded(
                "tall_back",
                "Tall back",
                "back",
                [0.38, 0.55, 0.05],
                0.025,
                [0.0, 1.02, -0.55],
            ),
            rounded(
                "low_back",
                "Low back",
                "back",
                [0.46, 0.28, 0.05],
                0.025,
                [0.0, 0.82, -0.55],
            ),
        ],
        detail_fragment: rounded_array(
            "chair_legs",
            "Chair legs",
            "leg",
            [0.045, 0.32, 0.045],
            0.012,
            [-0.31, 0.1, -0.31],
            array(4, [0.205, 0.0, 0.205]),
        ),
        accessory_fragment: rounded(
            "soft_cushion",
            "Soft cushion",
            "cushion",
            [0.38, 0.045, 0.38],
            0.045,
            [0.0, 0.67, 0.0],
        ),
    })
}

/// Build the market handcart fixture catalog.
#[must_use]
pub fn handcart_fixture_catalog() -> FoundryFixtureCatalog {
    expansion_fixture(ExpansionProfileSpec {
        slug: "handcart",
        document_id: "handcart-doc",
        family_id: "handcart",
        family_name: "Handcart",
        family_summary: "Vehicle-like prop with tray, handles, wheel detail, and cargo accessory.",
        style_id: "market_handcart",
        style_name: "Market Handcart",
        core_role: "tray",
        accent_role: "handle",
        detail_role: "wheel",
        accessory_role: "cargo",
        tags: &["cart", "market", "prop"],
        core_variants: vec![
            rounded(
                "flat_tray",
                "Flat tray",
                "tray",
                [0.72, 0.12, 0.42],
                0.035,
                [0.0, 0.42, 0.0],
            ),
            rounded(
                "deep_tray",
                "Deep tray",
                "tray",
                [0.66, 0.2, 0.46],
                0.04,
                [0.0, 0.48, 0.0],
            ),
            rounded(
                "narrow_tray",
                "Narrow tray",
                "tray",
                [0.58, 0.13, 0.34],
                0.035,
                [0.0, 0.42, 0.0],
            ),
        ],
        accent_variants: vec![
            rounded(
                "straight_handles",
                "Straight handles",
                "handle",
                [0.08, 0.06, 0.56],
                0.018,
                [0.0, 0.58, -1.05],
            ),
            rounded(
                "long_handles",
                "Long handles",
                "handle",
                [0.075, 0.055, 0.72],
                0.018,
                [0.0, 0.58, -1.2],
            ),
            rounded(
                "raised_handles",
                "Raised handles",
                "handle",
                [0.08, 0.08, 0.58],
                0.02,
                [0.0, 0.68, -1.05],
            ),
        ],
        detail_fragment: cylinder_array(
            "side_wheels",
            "Side wheels",
            "wheel",
            cylinder_spec(0.16, 0.08, 16),
            [-0.62, 0.24, 0.24],
            array(2, [1.24, 0.0, 0.0]),
        ),
        accessory_fragment: rounded(
            "stacked_cargo",
            "Stacked cargo",
            "cargo",
            [0.34, 0.18, 0.28],
            0.03,
            [0.0, 0.78, 0.02],
        ),
    })
}

/// Build the storybook tree fixture catalog.
#[must_use]
pub fn stylized_tree_fixture_catalog() -> FoundryFixtureCatalog {
    expansion_fixture(ExpansionProfileSpec {
        slug: "stylized-tree",
        document_id: "stylized-tree-doc",
        family_id: "stylized_tree",
        family_name: "Stylized Tree",
        family_summary: "Organic-but-controlled prop with trunk, canopy, fruit detail, and stump accessory.",
        style_id: "storybook_tree",
        style_name: "Storybook Tree",
        core_role: "trunk",
        accent_role: "canopy",
        detail_role: "fruit",
        accessory_role: "stump",
        tags: &["tree", "organic", "prop"],
        core_variants: vec![
            rounded(
                "straight_trunk",
                "Straight trunk",
                "trunk",
                [0.13, 0.72, 0.13],
                0.055,
                [0.0, 0.72, 0.0],
            ),
            rounded(
                "chunky_trunk",
                "Chunky trunk",
                "trunk",
                [0.18, 0.62, 0.16],
                0.065,
                [0.0, 0.62, 0.0],
            ),
            rounded(
                "tall_trunk",
                "Tall trunk",
                "trunk",
                [0.12, 0.9, 0.12],
                0.05,
                [0.0, 0.9, 0.0],
            ),
        ],
        accent_variants: vec![
            rounded(
                "round_canopy",
                "Round canopy",
                "canopy",
                [0.62, 0.42, 0.58],
                0.18,
                [0.0, 1.95, 0.0],
            ),
            rounded(
                "tiered_canopy",
                "Tiered canopy",
                "canopy",
                [0.7, 0.32, 0.62],
                0.16,
                [0.0, 1.86, 0.0],
            ),
            rounded(
                "narrow_canopy",
                "Narrow canopy",
                "canopy",
                [0.48, 0.48, 0.44],
                0.16,
                [0.0, 2.02, 0.0],
            ),
        ],
        detail_fragment: rounded_array(
            "canopy_fruit",
            "Canopy fruit",
            "fruit",
            [0.045, 0.045, 0.045],
            0.035,
            [-0.34, 2.0, 0.72],
            array(5, [0.17, -0.035, 0.0]),
        ),
        accessory_fragment: rounded(
            "cut_stump",
            "Cut stump",
            "stump",
            [0.22, 0.12, 0.2],
            0.05,
            [0.46, 0.12, -0.28],
        ),
    })
}

fn expansion_fixture(spec: ExpansionProfileSpec) -> FoundryFixtureCatalog {
    let family = family_schema(FamilySchemaSpec {
        id: spec.family_id,
        display_name: spec.family_name,
        summary: spec.family_summary,
        roles: vec![
            role(spec.core_role, RoleMultiplicity::Single, true),
            role(spec.accent_role, RoleMultiplicity::Single, true),
            role(spec.detail_role, RoleMultiplicity::Repeated, true),
            role(spec.accessory_role, RoleMultiplicity::Optional, false),
        ],
        allowed_operations: vec![
            AllowedOperationKind::Primitive,
            AllowedOperationKind::Array,
            AllowedOperationKind::Transform,
            AllowedOperationKind::Bevel,
        ],
        parameter_slots: vec![
            ratio_slot("width", "Width", spec.core_role, 0.0, 1.0, 0.05, 0.5),
            ratio_slot("height", "Height", spec.core_role, 0.0, 1.0, 0.05, 0.5),
            ratio_slot("depth", "Depth", spec.core_role, 0.0, 1.0, 0.05, 0.5),
            choice_slot(
                "body_variant",
                "Body Variant",
                spec.core_role,
                fragment_ids(&spec.core_variants),
            ),
            choice_slot(
                "accent_style",
                "Accent Style",
                spec.accent_role,
                fragment_ids(&spec.accent_variants),
            ),
            count_slot(
                "detail_density",
                "Detail Density",
                spec.detail_role,
                2.0,
                8.0,
                1.0,
                spec.detail_fragment.array.map_or(4, |array| array.count),
            ),
            toggle_slot("has_accessory", "Accessory", spec.accessory_role, true),
        ],
        compatible_style_kits: vec![spec.style_id.to_owned()],
        tags: spec.tags.iter().map(|tag| (*tag).to_owned()).collect(),
    });
    let prototypes = prototypes(&spec);
    let style = style_kit(
        spec.style_id,
        spec.style_name,
        spec.family_id,
        &prototypes,
        spec.tags.iter().map(|tag| (*tag).to_owned()).collect(),
    );
    let core_dimensions = rounded_half_extents(spec.core_variants[0]);
    let family_impl = family_implementation(
        spec.family_id,
        spec.family_name,
        vec![
            ParameterBinding::Scalar {
                slot: "width".to_owned(),
                role: spec.core_role.to_owned(),
                local_path: shape_asset::definition_scalar_path(
                    crate::LOCAL_DEFINITION,
                    "geometry.rounded_box.half_extents.x",
                ),
                transform: ScalarTransform::Ratio {
                    minimum: ratio_minimum(core_dimensions[0]),
                    maximum: ratio_maximum(core_dimensions[0]),
                },
            },
            ParameterBinding::Scalar {
                slot: "height".to_owned(),
                role: spec.core_role.to_owned(),
                local_path: shape_asset::definition_scalar_path(
                    crate::LOCAL_DEFINITION,
                    "geometry.rounded_box.half_extents.y",
                ),
                transform: ScalarTransform::Ratio {
                    minimum: ratio_minimum(core_dimensions[1]),
                    maximum: ratio_maximum(core_dimensions[1]),
                },
            },
            ParameterBinding::Scalar {
                slot: "depth".to_owned(),
                role: spec.core_role.to_owned(),
                local_path: shape_asset::definition_scalar_path(
                    crate::LOCAL_DEFINITION,
                    "geometry.rounded_box.half_extents.z",
                ),
                transform: ScalarTransform::Ratio {
                    minimum: ratio_minimum(core_dimensions[2]),
                    maximum: ratio_maximum(core_dimensions[2]),
                },
            },
            ParameterBinding::ChoiceToPrototype {
                slot: "body_variant".to_owned(),
                role: spec.core_role.to_owned(),
                choices: choice_map(&spec.core_variants),
            },
            ParameterBinding::ChoiceToPrototype {
                slot: "accent_style".to_owned(),
                role: spec.accent_role.to_owned(),
                choices: choice_map(&spec.accent_variants),
            },
            ParameterBinding::Scalar {
                slot: "detail_density".to_owned(),
                role: spec.detail_role.to_owned(),
                local_path: shape_asset::definition_scalar_path(
                    crate::LOCAL_DEFINITION,
                    "operation.1.linear_array.count",
                ),
                transform: ScalarTransform::IntegerCount,
            },
            ParameterBinding::TogglePartPresence {
                slot: "has_accessory".to_owned(),
                role: spec.accessory_role.to_owned(),
            },
        ],
    );
    let style_impl = style_implementation(
        spec.style_id,
        spec.family_id,
        BTreeMap::from([
            (
                spec.core_role.to_owned(),
                spec.core_variants[0].id.to_owned(),
            ),
            (
                spec.accent_role.to_owned(),
                spec.accent_variants[0].id.to_owned(),
            ),
            (
                spec.detail_role.to_owned(),
                spec.detail_fragment.id.to_owned(),
            ),
            (
                spec.accessory_role.to_owned(),
                spec.accessory_fragment.id.to_owned(),
            ),
        ]),
        fragments(&spec),
    );
    build_fixture_catalog(FixtureCatalogSpec {
        slug: spec.slug,
        document_id: spec.document_id,
        family,
        style,
        family_implementation: family_impl,
        style_implementation: style_impl,
        customizer_profile: expansion_customizer_profile(&spec),
        control_state: BTreeMap::from([
            ("width".to_owned(), ControlValue::Scalar(0.5)),
            ("height".to_owned(), ControlValue::Scalar(0.5)),
            ("depth".to_owned(), ControlValue::Scalar(0.5)),
            (
                "body_variant".to_owned(),
                ControlValue::Choice(spec.core_variants[0].id.to_owned()),
            ),
            (
                "accent_style".to_owned(),
                ControlValue::Choice(spec.accent_variants[0].id.to_owned()),
            ),
            (
                "detail_density".to_owned(),
                ControlValue::Integer(
                    spec.detail_fragment.array.map_or(4, |array| array.count) as i64
                ),
            ),
            ("has_accessory".to_owned(), ControlValue::Toggle(true)),
        ]),
    })
}

fn rounded_half_extents(fragment: FragmentSpec) -> [f32; 3] {
    match fragment.shape {
        FragmentShape::RoundedBox { half_extents, .. } => half_extents,
        _ => panic!("core expansion fragments must be rounded boxes"),
    }
}

fn ratio_minimum(default: f32) -> f32 {
    (default * 0.65).max(0.01)
}

fn ratio_maximum(default: f32) -> f32 {
    (default * 1.35).max(ratio_minimum(default) + 0.01)
}

fn expansion_customizer_profile(spec: &ExpansionProfileSpec) -> shape_foundry::CustomizerProfile {
    let body_values = fragment_id_refs(&spec.core_variants);
    let accent_values = fragment_id_refs(&spec.accent_variants);
    let detail_default = spec.detail_fragment.array.map_or(4, |array| array.count) as i64;
    let controls: Vec<CustomizerControl> = vec![
        continuous_control("width", "Width", "width", 0.5, 0.0, 1.0),
        continuous_control("height", "Height", "height", 0.5, 0.0, 1.0),
        continuous_control("depth", "Depth", "depth", 0.5, 0.0, 1.0),
        choice_control("body_variant", "Body Variant", "body_variant", &body_values),
        choice_control(
            "accent_style",
            "Accent Style",
            "accent_style",
            &accent_values,
        ),
        integer_control(
            "detail_density",
            "Detail Density",
            "detail_density",
            detail_default,
            2,
            8,
        ),
        toggle_control("has_accessory", "Accessory", "has_accessory", true),
    ];
    crate::customizer_profile(spec.family_id, spec.style_id, controls)
}

fn prototypes(spec: &ExpansionProfileSpec) -> Vec<(&str, &str, &str)> {
    all_fragment_specs(spec)
        .into_iter()
        .map(|fragment| (fragment.id, fragment.label, fragment.role))
        .collect()
}

fn fragments(spec: &ExpansionProfileSpec) -> Vec<RecipeFragment> {
    all_fragment_specs(spec)
        .into_iter()
        .map(recipe_fragment)
        .collect()
}

fn all_fragment_specs(spec: &ExpansionProfileSpec) -> Vec<FragmentSpec> {
    spec.core_variants
        .iter()
        .chain(spec.accent_variants.iter())
        .copied()
        .chain([spec.detail_fragment, spec.accessory_fragment])
        .collect()
}

fn fragment_ids(fragments: &[FragmentSpec]) -> Vec<String> {
    fragments
        .iter()
        .map(|fragment| fragment.id.to_owned())
        .collect()
}

fn fragment_id_refs(fragments: &[FragmentSpec]) -> Vec<&str> {
    fragments.iter().map(|fragment| fragment.id).collect()
}

fn choice_map(fragments: &[FragmentSpec]) -> BTreeMap<String, String> {
    fragments
        .iter()
        .map(|fragment| (fragment.id.to_owned(), fragment.id.to_owned()))
        .collect()
}

fn recipe_fragment(spec: FragmentSpec) -> RecipeFragment {
    let operations = spec
        .array
        .map(|array| vec![linear_array(1, array.count, array.offset)])
        .unwrap_or_default();
    match spec.shape {
        FragmentShape::RoundedBox {
            half_extents,
            radius,
        } => fragment(
            spec.id,
            spec.role,
            GeometrySource::RoundedBox {
                half_extents,
                radius,
            },
            spec.translation,
            operations,
            &scalar_paths(
                &[
                    ("geometry.rounded_box.half_extents.x", 0.01, 5.0, 0.01),
                    ("geometry.rounded_box.half_extents.y", 0.01, 5.0, 0.01),
                    ("geometry.rounded_box.half_extents.z", 0.01, 5.0, 0.01),
                    ("geometry.rounded_box.radius", 0.0, 0.5, 0.01),
                ],
                spec.array,
            ),
        ),
        FragmentShape::Plate { size, thickness } => fragment(
            spec.id,
            spec.role,
            GeometrySource::Plate { size, thickness },
            spec.translation,
            operations,
            &scalar_paths(
                &[
                    ("geometry.plate.size.x", 0.05, 5.0, 0.05),
                    ("geometry.plate.size.y", 0.05, 5.0, 0.05),
                    ("geometry.plate.thickness", 0.01, 0.5, 0.01),
                ],
                spec.array,
            ),
        ),
        FragmentShape::Cylinder {
            radius,
            height,
            radial_segments,
        } => fragment(
            spec.id,
            spec.role,
            GeometrySource::Cylinder {
                radius,
                height,
                radial_segments,
            },
            spec.translation,
            operations,
            &scalar_paths(
                &[
                    ("geometry.cylinder.radius", 0.01, 2.0, 0.01),
                    ("geometry.cylinder.height", 0.01, 5.0, 0.01),
                    ("geometry.cylinder.radial_segments", 6.0, 64.0, 1.0),
                ],
                spec.array,
            ),
        ),
    }
}

fn scalar_paths(
    base_paths: &[(&'static str, f32, f32, f32)],
    array: Option<ArraySpec>,
) -> Vec<(&'static str, f32, f32, f32)> {
    let mut paths = base_paths.to_vec();
    if array.is_some() {
        paths.push(("operation.1.linear_array.count", 1.0, 12.0, 1.0));
    }
    paths
}

const fn rounded(
    id: &'static str,
    label: &'static str,
    role: &'static str,
    half_extents: [f32; 3],
    radius: f32,
    translation: [f32; 3],
) -> FragmentSpec {
    FragmentSpec {
        id,
        label,
        role,
        shape: FragmentShape::RoundedBox {
            half_extents,
            radius,
        },
        translation,
        array: None,
    }
}

const fn rounded_array(
    id: &'static str,
    label: &'static str,
    role: &'static str,
    half_extents: [f32; 3],
    radius: f32,
    translation: [f32; 3],
    array: ArraySpec,
) -> FragmentSpec {
    FragmentSpec {
        id,
        label,
        role,
        shape: FragmentShape::RoundedBox {
            half_extents,
            radius,
        },
        translation,
        array: Some(array),
    }
}

const fn plate(
    id: &'static str,
    label: &'static str,
    role: &'static str,
    size: [f32; 2],
    thickness: f32,
    translation: [f32; 3],
) -> FragmentSpec {
    FragmentSpec {
        id,
        label,
        role,
        shape: FragmentShape::Plate { size, thickness },
        translation,
        array: None,
    }
}

const fn cylinder_array(
    id: &'static str,
    label: &'static str,
    role: &'static str,
    cylinder: CylinderSpec,
    translation: [f32; 3],
    array: ArraySpec,
) -> FragmentSpec {
    FragmentSpec {
        id,
        label,
        role,
        shape: FragmentShape::Cylinder {
            radius: cylinder.radius,
            height: cylinder.height,
            radial_segments: cylinder.radial_segments,
        },
        translation,
        array: Some(array),
    }
}

const fn array(count: u32, offset: [f32; 3]) -> ArraySpec {
    ArraySpec { count, offset }
}

const fn cylinder_spec(radius: f32, height: f32, radial_segments: u32) -> CylinderSpec {
    CylinderSpec {
        radius,
        height,
        radial_segments,
    }
}
