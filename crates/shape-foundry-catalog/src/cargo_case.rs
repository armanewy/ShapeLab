//! Cargo Case reusable base-family fixture.

use std::collections::{BTreeMap, BTreeSet};

use shape_asset::{
    AssetId, AssetRecipe, Frame3, GeometryRecipe, GeometrySource, PartDefinition, PartDefinitionId,
    PartInstance, PartInstanceId, Transform3, validate_asset_recipe,
};
use shape_family::{AllowedOperationKind, RoleMultiplicity};
use shape_family_compile::{
    RECIPE_FRAGMENT_SCHEMA_VERSION, RecipeFragment, RecipeFragmentExports, ScalarTransform,
};
use shape_foundry::{CandidateStrategy, ControlValue};

use crate::{
    FamilySchemaSpec, FixtureCatalogSpec, FoundryFixtureCatalog, build_fixture_catalog,
    choice_control, choice_slot, continuous_control, cylinder_fragment, family_implementation,
    family_schema, linear_array, ratio_slot, role, rounded_box_fragment, style_implementation,
    style_kit,
};

/// Cargo Case base-family slug.
pub const CARGO_CASE_BASE_SLUG: &str = "cargo-case-base";
/// Cargo Case family ID.
pub const CARGO_CASE_FAMILY_ID: &str = "cargo_case";
/// Base style ID for neutral Cargo Case proof renders.
pub const CARGO_CASE_BASE_STYLE_ID: &str = "cargo_case_base";

/// Clay display mode used by Cargo Case quality gates.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum CargoCaseClayDisplayMode {
    /// One neutral clay material for all parts.
    PureClay,
    /// Neutral gray display values by semantic part group.
    SemanticClay,
}

/// Preview-only semantic gray assignment.
#[derive(Debug, Clone, PartialEq)]
pub struct CargoCaseSemanticClayAssignment {
    /// Role or semantic part group.
    pub role_or_part_group: &'static str,
    /// Product-safe display label.
    pub display_label: &'static str,
    /// Neutral gray value in 0.0..=1.0.
    pub neutral_gray_value: f32,
    /// Higher priority wins when groups overlap.
    pub priority: u8,
    /// Whether this assignment applies to generated candidates.
    pub applies_to_candidates: bool,
}

/// Pure Clay uses one neutral gray value.
#[must_use]
pub fn pure_clay_gray_value() -> f32 {
    0.68
}

/// Semantic Clay role mapping. Display/preview metadata only.
#[must_use]
pub fn semantic_clay_assignments() -> Vec<CargoCaseSemanticClayAssignment> {
    vec![
        CargoCaseSemanticClayAssignment {
            role_or_part_group: "body",
            display_label: "Primary Mass",
            neutral_gray_value: 0.72,
            priority: 10,
            applies_to_candidates: true,
        },
        CargoCaseSemanticClayAssignment {
            role_or_part_group: "lid,panel_fields,label_plate_geometry",
            display_label: "Secondary Panels",
            neutral_gray_value: 0.64,
            priority: 20,
            applies_to_candidates: true,
        },
        CargoCaseSemanticClayAssignment {
            role_or_part_group: "edge_trim,corner_guards,base_feet_or_skids,handles,utility_rails,reinforcement_bands",
            display_label: "Structural Trim",
            neutral_gray_value: 0.48,
            priority: 30,
            applies_to_candidates: true,
        },
        CargoCaseSemanticClayAssignment {
            role_or_part_group: "vents,side_grilles",
            display_label: "Recesses / Vents",
            neutral_gray_value: 0.32,
            priority: 40,
            applies_to_candidates: true,
        },
        CargoCaseSemanticClayAssignment {
            role_or_part_group: "fasteners,latches,hinge_or_closure_detail",
            display_label: "Fasteners / Detail",
            neutral_gray_value: 0.26,
            priority: 50,
            applies_to_candidates: true,
        },
    ]
}

/// Build the Cargo Case base-family fixture catalog.
#[must_use]
pub fn fixture_catalog() -> FoundryFixtureCatalog {
    let family = family_schema(FamilySchemaSpec {
        id: CARGO_CASE_FAMILY_ID,
        display_name: "Cargo Case",
        summary: "Reusable clay equipment-case family with body, lid, panels, trim, guards, skids, handles, vents, and details.",
        roles: vec![
            role("body", RoleMultiplicity::Single, true),
            role("lid", RoleMultiplicity::Single, true),
            role("panel_fields", RoleMultiplicity::Repeated, true),
            role("edge_trim", RoleMultiplicity::Repeated, true),
            role("corner_guards", RoleMultiplicity::Repeated, true),
            role("base_feet_or_skids", RoleMultiplicity::Repeated, true),
            role("handles", RoleMultiplicity::Optional, false),
            role("latches", RoleMultiplicity::Optional, false),
            role("vents", RoleMultiplicity::Repeated, false),
            role("fasteners", RoleMultiplicity::Repeated, false),
            role("reinforcement_bands", RoleMultiplicity::Optional, false),
            role("utility_rails", RoleMultiplicity::Optional, false),
            role("side_grilles", RoleMultiplicity::Optional, false),
            role("label_plate_geometry", RoleMultiplicity::Optional, false),
            role("hinge_or_closure_detail", RoleMultiplicity::Optional, false),
        ],
        allowed_operations: vec![
            AllowedOperationKind::Primitive,
            AllowedOperationKind::Array,
            AllowedOperationKind::Transform,
            AllowedOperationKind::Bevel,
        ],
        parameter_slots: vec![
            ratio_slot(
                "overall_proportions",
                "Overall Proportions",
                "body",
                0.0,
                1.0,
                0.05,
                0.52,
            ),
            ratio_slot(
                "structural_heft",
                "Structural Heft",
                "body",
                0.0,
                1.0,
                0.05,
                0.46,
            ),
            choice_slot(
                "panel_complexity",
                "Panel Complexity",
                "panel_fields",
                vec![
                    "clean_panel".to_owned(),
                    "shallow_recessed_panel".to_owned(),
                    "deep_framed_panel".to_owned(),
                ],
            ),
            choice_slot(
                "handle_style",
                "Handle Style",
                "handles",
                vec![
                    "flush_grip".to_owned(),
                    "side_rail".to_owned(),
                    "cargo_bar".to_owned(),
                    "inset_latch_handle".to_owned(),
                ],
            ),
            choice_slot(
                "vent_density",
                "Vent Density",
                "vents",
                vec![
                    "none_sparse".to_owned(),
                    "standard_grille".to_owned(),
                    "dense_grille".to_owned(),
                    "side_vent_bay".to_owned(),
                ],
            ),
            choice_slot(
                "trim_style",
                "Trim Style",
                "edge_trim",
                vec![
                    "clean".to_owned(),
                    "utility_rail".to_owned(),
                    "reinforced_edge_trim".to_owned(),
                    "industrial_band".to_owned(),
                ],
            ),
            choice_slot(
                "detail_density",
                "Detail Density",
                "fasteners",
                vec![
                    "low_detail".to_owned(),
                    "medium_detail".to_owned(),
                    "high_detail".to_owned(),
                ],
            ),
        ],
        compatible_style_kits: vec![CARGO_CASE_BASE_STYLE_ID.to_owned()],
        tags: vec!["cargo-case".to_owned(), "equipment-case".to_owned()],
    });

    let style = style_kit(
        CARGO_CASE_BASE_STYLE_ID,
        "Cargo Case Base",
        CARGO_CASE_FAMILY_ID,
        &[
            ("cargo_case_body", "Cargo case body", "body"),
            ("raised_lid", "Raised lid", "lid"),
            ("clean_panel", "Clean panel", "panel_fields"),
            (
                "shallow_recessed_panel",
                "Shallow recessed panel",
                "panel_fields",
            ),
            ("deep_framed_panel", "Deep framed panel", "panel_fields"),
            ("clean_edge_trim", "Clean edge trim", "edge_trim"),
            ("utility_rail_trim", "Utility rail trim", "edge_trim"),
            ("reinforced_edge_trim", "Reinforced edge trim", "edge_trim"),
            ("industrial_band_trim", "Industrial band trim", "edge_trim"),
            ("minimal_corner_cap", "Minimal corner cap", "corner_guards"),
            ("block_corner_guard", "Block corner guard", "corner_guards"),
            (
                "chamfered_armor_block",
                "Chamfered armor block",
                "corner_guards",
            ),
            ("low_case_skids", "Low case skids", "base_feet_or_skids"),
            ("flush_grip_handle", "Flush grip handle", "handles"),
            ("side_rail_handle", "Side rail handle", "handles"),
            ("cargo_bar_handle", "Cargo bar handle", "handles"),
            ("inset_latch_handle", "Inset latch handle", "handles"),
            ("none_sparse_vents", "Sparse vent plates", "vents"),
            ("standard_grille_vents", "Standard grille", "vents"),
            ("dense_grille_vents", "Dense grille", "vents"),
            ("side_vent_bay", "Side vent bay", "vents"),
            ("low_fasteners", "Low fasteners", "fasteners"),
            ("medium_fasteners", "Medium fasteners", "fasteners"),
            ("high_fasteners", "High fasteners", "fasteners"),
            ("latch_pair", "Latch pair", "latches"),
            (
                "center_label_plate",
                "Center label plate",
                "label_plate_geometry",
            ),
            (
                "rear_hinge_detail",
                "Rear hinge detail",
                "hinge_or_closure_detail",
            ),
        ],
        vec!["cargo-case".to_owned(), "clay".to_owned()],
    );

    let family_impl = family_implementation(
        CARGO_CASE_FAMILY_ID,
        "Cargo Case base family",
        vec![
            scalar_binding(
                "overall_proportions",
                "body",
                "geometry.rounded_box.half_extents.x",
                0.88,
                1.78,
            ),
            scalar_binding(
                "overall_proportions",
                "body",
                "geometry.rounded_box.half_extents.z",
                0.58,
                0.92,
            ),
            scalar_binding(
                "structural_heft",
                "body",
                "geometry.rounded_box.half_extents.y",
                0.34,
                0.58,
            ),
            shape_family_compile::ParameterBinding::ChoiceToPrototype {
                slot: "panel_complexity".to_owned(),
                role: "panel_fields".to_owned(),
                choices: BTreeMap::from([
                    ("clean_panel".to_owned(), "clean_panel".to_owned()),
                    (
                        "shallow_recessed_panel".to_owned(),
                        "shallow_recessed_panel".to_owned(),
                    ),
                    (
                        "deep_framed_panel".to_owned(),
                        "deep_framed_panel".to_owned(),
                    ),
                ]),
            },
            shape_family_compile::ParameterBinding::ChoiceToPrototype {
                slot: "handle_style".to_owned(),
                role: "handles".to_owned(),
                choices: BTreeMap::from([
                    ("flush_grip".to_owned(), "flush_grip_handle".to_owned()),
                    ("side_rail".to_owned(), "side_rail_handle".to_owned()),
                    ("cargo_bar".to_owned(), "cargo_bar_handle".to_owned()),
                    (
                        "inset_latch_handle".to_owned(),
                        "inset_latch_handle".to_owned(),
                    ),
                ]),
            },
            shape_family_compile::ParameterBinding::ChoiceToPrototype {
                slot: "vent_density".to_owned(),
                role: "vents".to_owned(),
                choices: BTreeMap::from([
                    ("none_sparse".to_owned(), "none_sparse_vents".to_owned()),
                    (
                        "standard_grille".to_owned(),
                        "standard_grille_vents".to_owned(),
                    ),
                    ("dense_grille".to_owned(), "dense_grille_vents".to_owned()),
                    ("side_vent_bay".to_owned(), "side_vent_bay".to_owned()),
                ]),
            },
            shape_family_compile::ParameterBinding::ChoiceToPrototype {
                slot: "trim_style".to_owned(),
                role: "edge_trim".to_owned(),
                choices: BTreeMap::from([
                    ("clean".to_owned(), "clean_edge_trim".to_owned()),
                    ("utility_rail".to_owned(), "utility_rail_trim".to_owned()),
                    (
                        "reinforced_edge_trim".to_owned(),
                        "reinforced_edge_trim".to_owned(),
                    ),
                    (
                        "industrial_band".to_owned(),
                        "industrial_band_trim".to_owned(),
                    ),
                ]),
            },
            shape_family_compile::ParameterBinding::ChoiceToPrototype {
                slot: "detail_density".to_owned(),
                role: "fasteners".to_owned(),
                choices: BTreeMap::from([
                    ("low_detail".to_owned(), "low_fasteners".to_owned()),
                    ("medium_detail".to_owned(), "medium_fasteners".to_owned()),
                    ("high_detail".to_owned(), "high_fasteners".to_owned()),
                ]),
            },
        ],
    );

    let style_impl = style_implementation(
        CARGO_CASE_BASE_STYLE_ID,
        CARGO_CASE_FAMILY_ID,
        BTreeMap::from([
            ("body".to_owned(), "cargo_case_body".to_owned()),
            ("lid".to_owned(), "raised_lid".to_owned()),
            (
                "panel_fields".to_owned(),
                "shallow_recessed_panel".to_owned(),
            ),
            ("edge_trim".to_owned(), "clean_edge_trim".to_owned()),
            ("corner_guards".to_owned(), "block_corner_guard".to_owned()),
            ("base_feet_or_skids".to_owned(), "low_case_skids".to_owned()),
            ("handles".to_owned(), "flush_grip_handle".to_owned()),
            ("vents".to_owned(), "standard_grille_vents".to_owned()),
            ("fasteners".to_owned(), "medium_fasteners".to_owned()),
        ]),
        vec![
            body_fragment(),
            rounded_box_fragment(
                "raised_lid",
                "lid",
                [1.12, 0.055, 0.075],
                0.025,
                [0.0, 0.52, 0.80],
                Vec::new(),
            ),
            rounded_box_fragment(
                "clean_panel",
                "panel_fields",
                [0.84, 0.055, 0.22],
                0.018,
                [0.0, 0.53, 0.16],
                Vec::new(),
            ),
            panel_assembly("shallow_recessed_panel", 0.055, 0.36),
            panel_assembly("deep_framed_panel", 0.08, 0.52),
            trim_assembly("clean_edge_trim", 0.055, 0.055),
            trim_assembly("utility_rail_trim", 0.075, 0.095),
            trim_assembly("reinforced_edge_trim", 0.095, 0.085),
            trim_assembly("industrial_band_trim", 0.12, 0.13),
            corner_guard_assembly("minimal_corner_cap", [0.11, 0.07, 0.11]),
            corner_guard_assembly("block_corner_guard", [0.16, 0.08, 0.16]),
            corner_guard_assembly("chamfered_armor_block", [0.22, 0.095, 0.18]),
            skid_assembly(),
            handle_assembly(
                "flush_grip_handle",
                &[
                    box_part(
                        90,
                        91,
                        None,
                        "flush grip back",
                        [0.54, 0.055, 0.13],
                        [0.0, 0.68, -0.46],
                    ),
                    box_part(
                        92,
                        93,
                        Some(91),
                        "left flush grip cheek",
                        [0.055, 0.05, 0.18],
                        [-0.42, 0.105, 0.0],
                    ),
                    box_part(
                        94,
                        95,
                        Some(91),
                        "right flush grip cheek",
                        [0.055, 0.05, 0.18],
                        [0.42, 0.105, 0.0],
                    ),
                ],
            ),
            handle_assembly(
                "side_rail_handle",
                &[
                    box_part(
                        90,
                        91,
                        None,
                        "left rail grip",
                        [0.065, 0.06, 0.36],
                        [-0.68, 0.68, -0.20],
                    ),
                    box_part(
                        92,
                        93,
                        Some(91),
                        "right rail grip",
                        [0.065, 0.06, 0.36],
                        [1.36, 0.0, 0.0],
                    ),
                    box_part(
                        94,
                        95,
                        Some(91),
                        "upper rail bridge",
                        [0.72, 0.045, 0.05],
                        [0.68, 0.105, 0.31],
                    ),
                    box_part(
                        96,
                        97,
                        Some(91),
                        "lower rail bridge",
                        [0.72, 0.045, 0.05],
                        [0.68, 0.105, -0.31],
                    ),
                ],
            ),
            handle_assembly(
                "cargo_bar_handle",
                &[
                    box_part(
                        90,
                        91,
                        None,
                        "cargo bar grip",
                        [0.78, 0.055, 0.06],
                        [0.0, 0.68, -0.50],
                    ),
                    box_part(
                        92,
                        93,
                        Some(91),
                        "left cargo mount",
                        [0.11, 0.055, 0.14],
                        [-0.62, 0.11, 0.0],
                    ),
                    box_part(
                        94,
                        95,
                        Some(91),
                        "right cargo mount",
                        [0.11, 0.055, 0.14],
                        [0.62, 0.11, 0.0],
                    ),
                ],
            ),
            handle_assembly(
                "inset_latch_handle",
                &[
                    box_part(
                        90,
                        91,
                        None,
                        "inset handle well",
                        [0.5, 0.055, 0.11],
                        [0.0, 0.68, -0.46],
                    ),
                    box_part(
                        92,
                        93,
                        Some(91),
                        "latch pull",
                        [0.22, 0.055, 0.035],
                        [0.0, 0.18, 0.0],
                    ),
                    box_part(
                        94,
                        95,
                        Some(91),
                        "left latch block",
                        [0.06, 0.055, 0.1],
                        [-0.36, 0.18, 0.0],
                    ),
                    box_part(
                        96,
                        97,
                        Some(91),
                        "right latch block",
                        [0.06, 0.055, 0.1],
                        [0.36, 0.18, 0.0],
                    ),
                ],
            ),
            vent_assembly("none_sparse_vents", 2, 0.2, 0.06),
            vent_assembly("standard_grille_vents", 4, 0.18, 0.055),
            vent_assembly("dense_grille_vents", 8, 0.11, 0.04),
            side_vent_bay(),
            fastener_fragment("low_fasteners", 4),
            fastener_fragment("medium_fasteners", 10),
            fastener_fragment("high_fasteners", 18),
            rounded_box_fragment(
                "latch_pair",
                "latches",
                [0.13, 0.055, 0.085],
                0.02,
                [-0.86, 0.74, 0.43],
                vec![linear_array(1, 2, [1.72, 0.0, 0.0])],
            ),
            rounded_box_fragment(
                "center_label_plate",
                "label_plate_geometry",
                [0.28, 0.055, 0.08],
                0.014,
                [0.0, 0.74, 0.45],
                Vec::new(),
            ),
            rounded_box_fragment(
                "rear_hinge_detail",
                "hinge_or_closure_detail",
                [0.82, 0.055, 0.055],
                0.014,
                [0.0, -0.62, 0.66],
                Vec::new(),
            ),
        ],
    );

    let mut profile = crate::customizer_profile(
        CARGO_CASE_FAMILY_ID,
        CARGO_CASE_BASE_STYLE_ID,
        vec![
            continuous_control(
                "overall_proportions",
                "Overall Proportions",
                "overall_proportions",
                0.52,
                0.0,
                1.0,
            ),
            continuous_control(
                "structural_heft",
                "Structural Heft",
                "structural_heft",
                0.46,
                0.0,
                1.0,
            ),
            choice_control(
                "panel_complexity",
                "Panel Complexity",
                "panel_complexity",
                &["clean_panel", "shallow_recessed_panel", "deep_framed_panel"],
            ),
            choice_control(
                "handle_style",
                "Handle Style",
                "handle_style",
                &["flush_grip", "side_rail", "cargo_bar", "inset_latch_handle"],
            ),
            choice_control(
                "vent_density",
                "Vent Density",
                "vent_density",
                &[
                    "none_sparse",
                    "standard_grille",
                    "dense_grille",
                    "side_vent_bay",
                ],
            ),
            choice_control(
                "trim_style",
                "Trim Style",
                "trim_style",
                &[
                    "clean",
                    "utility_rail",
                    "reinforced_edge_trim",
                    "industrial_band",
                ],
            ),
            choice_control(
                "detail_density",
                "Detail Density",
                "detail_density",
                &["low_detail", "medium_detail", "high_detail"],
            ),
        ],
    );
    profile.candidate_strategies = vec![
        strategy("light-utility", "Light Utility"),
        strategy("reinforced", "Reinforced"),
        strategy("compact", "Compact"),
        strategy("wide", "Wide"),
        strategy("minimal", "Minimal"),
        strategy("detailed", "Detailed"),
    ];

    build_fixture_catalog(FixtureCatalogSpec {
        slug: CARGO_CASE_BASE_SLUG,
        document_id: "cargo-case-base-doc",
        family,
        style,
        family_implementation: family_impl,
        style_implementation: style_impl,
        customizer_profile: profile,
        control_state: BTreeMap::from([
            ("overall_proportions".to_owned(), ControlValue::Scalar(0.52)),
            ("structural_heft".to_owned(), ControlValue::Scalar(0.46)),
            (
                "panel_complexity".to_owned(),
                ControlValue::Choice("shallow_recessed_panel".to_owned()),
            ),
            (
                "handle_style".to_owned(),
                ControlValue::Choice("flush_grip".to_owned()),
            ),
            (
                "vent_density".to_owned(),
                ControlValue::Choice("standard_grille".to_owned()),
            ),
            (
                "trim_style".to_owned(),
                ControlValue::Choice("clean".to_owned()),
            ),
            (
                "detail_density".to_owned(),
                ControlValue::Choice("medium_detail".to_owned()),
            ),
        ]),
    })
}

fn scalar_binding(
    slot: &str,
    role: &str,
    local_path: &str,
    minimum: f32,
    maximum: f32,
) -> shape_family_compile::ParameterBinding {
    shape_family_compile::ParameterBinding::Scalar {
        slot: slot.to_owned(),
        role: role.to_owned(),
        local_path: shape_asset::definition_scalar_path(crate::LOCAL_DEFINITION, local_path),
        transform: ScalarTransform::Ratio { minimum, maximum },
    }
}

fn strategy(id: &str, label: &str) -> CandidateStrategy {
    CandidateStrategy {
        id: id.to_owned(),
        label: label.to_owned(),
        control_ids: vec![
            "overall_proportions".to_owned(),
            "structural_heft".to_owned(),
            "panel_complexity".to_owned(),
            "handle_style".to_owned(),
            "vent_density".to_owned(),
            "trim_style".to_owned(),
            "detail_density".to_owned(),
        ],
    }
}

fn body_fragment() -> RecipeFragment {
    rounded_box_fragment(
        "cargo_case_body",
        "body",
        [1.25, 0.46, 0.72],
        0.055,
        [0.0, 0.0, 0.0],
        Vec::new(),
    )
}

fn panel_assembly(id: &str, thickness: f32, height: f32) -> RecipeFragment {
    rounded_box_assembly_fragment(
        id,
        "panel_fields",
        &[
            box_part(
                90,
                91,
                None,
                "left panel field",
                [0.38, thickness, height],
                [-0.48, 0.53, 0.16],
            ),
            box_part(
                92,
                93,
                None,
                "right panel field",
                [0.38, thickness, height],
                [0.48, 0.53, 0.16],
            ),
            box_part(
                94,
                95,
                None,
                "center rail between panel fields",
                [0.045, thickness + 0.02, height + 0.08],
                [0.0, 0.53, 0.16],
            ),
        ],
    )
}

fn trim_assembly(id: &str, rail: f32, depth: f32) -> RecipeFragment {
    rounded_box_assembly_fragment(
        id,
        "edge_trim",
        &[
            box_part(
                90,
                91,
                None,
                "top trim rail",
                [0.74, depth, rail],
                [0.0, 0.66, 0.60],
            ),
            box_part(
                92,
                93,
                None,
                "bottom trim rail",
                [0.74, depth, rail],
                [0.0, 0.66, -0.66],
            ),
            box_part(
                94,
                95,
                None,
                "left trim rail",
                [rail, depth, 0.30],
                [-1.03, 0.66, 0.0],
            ),
            box_part(
                96,
                97,
                None,
                "right trim rail",
                [rail, depth, 0.30],
                [1.03, 0.66, 0.0],
            ),
        ],
    )
}

fn corner_guard_assembly(id: &str, half_extents: [f32; 3]) -> RecipeFragment {
    rounded_box_assembly_fragment(
        id,
        "corner_guards",
        &[
            box_part(
                90,
                91,
                None,
                "upper left guard",
                half_extents,
                [-1.12, 0.78, 0.58],
            ),
            box_part(
                92,
                93,
                None,
                "upper right guard",
                half_extents,
                [1.12, 0.78, 0.58],
            ),
            box_part(
                94,
                95,
                None,
                "lower left guard",
                half_extents,
                [-1.12, 0.78, -0.58],
            ),
            box_part(
                96,
                97,
                None,
                "lower right guard",
                half_extents,
                [1.12, 0.78, -0.58],
            ),
        ],
    )
}

fn skid_assembly() -> RecipeFragment {
    rounded_box_assembly_fragment(
        "low_case_skids",
        "base_feet_or_skids",
        &[
            box_part(
                90,
                91,
                None,
                "left low skid",
                [0.42, 0.09, 0.055],
                [-0.55, 0.0, -0.82],
            ),
            box_part(
                92,
                93,
                None,
                "right low skid",
                [0.42, 0.09, 0.055],
                [0.55, 0.0, -0.82],
            ),
        ],
    )
}

fn vent_assembly(id: &str, count: usize, width: f32, height: f32) -> RecipeFragment {
    let spacing = if count <= 2 { 0.38 } else { 0.2 };
    let start = -((count as f32 - 1.0) * spacing) * 0.5;
    let parts = (0..count)
        .map(|index| {
            box_part(
                90 + index as u64 * 2,
                91 + index as u64 * 2,
                None,
                "front vent blade",
                [width * 0.5, 0.035, height],
                [start + index as f32 * spacing, 0.92, -0.18],
            )
        })
        .collect::<Vec<_>>();
    rounded_box_assembly_fragment(id, "vents", &parts)
}

fn side_vent_bay() -> RecipeFragment {
    rounded_box_assembly_fragment(
        "side_vent_bay",
        "vents",
        &[
            box_part(
                90,
                91,
                None,
                "left side vent bay",
                [0.11, 0.04, 0.28],
                [-1.18, 0.92, -0.12],
            ),
            box_part(
                92,
                93,
                None,
                "right side vent bay",
                [0.11, 0.04, 0.28],
                [1.18, 0.92, -0.12],
            ),
            box_part(
                94,
                95,
                None,
                "left bay center slit",
                [0.045, 0.045, 0.35],
                [-1.18, 1.02, -0.12],
            ),
            box_part(
                96,
                97,
                None,
                "right bay center slit",
                [0.045, 0.045, 0.35],
                [1.18, 1.02, -0.12],
            ),
        ],
    )
}

fn fastener_fragment(id: &str, count: u32) -> RecipeFragment {
    cylinder_fragment(
        id,
        "fasteners",
        0.038,
        0.09,
        18,
        [-0.78, 0.98, 0.48],
        vec![linear_array(1, count, [0.17, 0.0, 0.0])],
    )
}

#[derive(Debug, Clone, Copy)]
struct BoxAssemblyPart {
    definition: PartDefinitionId,
    instance: PartInstanceId,
    parent: Option<PartInstanceId>,
    name: &'static str,
    half_extents: [f32; 3],
    translation: [f32; 3],
}

fn box_part(
    definition: u64,
    instance: u64,
    parent: Option<u64>,
    name: &'static str,
    half_extents: [f32; 3],
    translation: [f32; 3],
) -> BoxAssemblyPart {
    BoxAssemblyPart {
        definition: PartDefinitionId(definition),
        instance: PartInstanceId(instance),
        parent: parent.map(PartInstanceId),
        name,
        half_extents,
        translation,
    }
}

fn handle_assembly(id: &str, parts: &[BoxAssemblyPart]) -> RecipeFragment {
    rounded_box_assembly_fragment(id, "handles", parts)
}

fn rounded_box_assembly_fragment(
    id: &str,
    role_name: &str,
    parts: &[BoxAssemblyPart],
) -> RecipeFragment {
    let mut recipe = AssetRecipe::new(AssetId(1), format!("{id} fragment"));
    for part in parts {
        recipe.definitions.insert(
            part.definition,
            PartDefinition {
                id: part.definition,
                name: part.name.to_owned(),
                tags: BTreeSet::from([role_name.to_owned(), format!("role:{role_name}")]),
                geometry: GeometryRecipe {
                    source: GeometrySource::RoundedBox {
                        half_extents: part.half_extents,
                        radius: 0.018,
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
        recipe.instances.insert(
            part.instance,
            PartInstance {
                id: part.instance,
                definition: part.definition,
                name: part.name.to_owned(),
                parent: part.parent,
                local_transform: Transform3 {
                    translation: part.translation,
                    ..Transform3::default()
                },
                attachment: None,
                enabled: true,
                tags: BTreeSet::from([role_name.to_owned(), format!("role:{role_name}")]),
                generated_by: None,
            },
        );
    }
    recipe.root_instances.extend(
        parts
            .iter()
            .filter(|part| part.parent.is_none())
            .map(|part| part.instance),
    );
    recipe.next_ids.part_definition = parts
        .iter()
        .map(|part| part.definition.0)
        .max()
        .unwrap_or(crate::LOCAL_DEFINITION.0)
        + 1;
    recipe.next_ids.part_instance = parts
        .iter()
        .map(|part| part.instance.0)
        .max()
        .unwrap_or(crate::LOCAL_INSTANCE.0)
        + 1;
    recipe.next_ids.parameter = 1;
    recipe.next_ids.operation = 1;
    recipe.next_ids.socket = 1;
    assert!(
        validate_asset_recipe(&recipe).is_valid(),
        "{id} assembly should validate"
    );
    RecipeFragment {
        schema_version: RECIPE_FRAGMENT_SCHEMA_VERSION,
        id: id.to_owned(),
        provided_role: role_name.to_owned(),
        exports: RecipeFragmentExports {
            role_occurrence_roots: recipe.root_instances.clone(),
            internal_roots: Vec::new(),
            socket_ports: Vec::new(),
            surface_ports: Vec::new(),
        },
        recipe,
    }
}
