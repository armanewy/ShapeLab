//! Utility Crate reusable family fixture.

use std::collections::{BTreeMap, BTreeSet};

use shape_asset::{
    AssetId, AssetRecipe, Frame3, GeometryRecipe, GeometrySource, PartDefinition, PartDefinitionId,
    PartInstance, PartInstanceId, Transform3, definition_scalar_path, validate_asset_recipe,
};
use shape_family::{AllowedOperationKind, RoleMultiplicity};
use shape_family_compile::{
    ParameterBinding, RECIPE_FRAGMENT_SCHEMA_VERSION, RecipeFragment, RecipeFragmentExports,
    ScalarTransform,
};
use shape_foundry::{CandidateStrategy, ControlValue};

use crate::{
    CatalogCurationMetadata, FamilySchemaSpec, FixtureCatalogSpec, FoundryFixtureCatalog,
    StarterTemplateQualityEvidence, build_fixture_catalog, choice_control, choice_slot,
    continuous_control, family_implementation, family_schema, role, rounded_box_fragment,
    starter_template_curation_state_from_quality, style_implementation, style_kit,
};

/// Utility Crate profile slug.
pub const UTILITY_CRATE_SLUG: &str = "utility-crate";
/// Utility Crate family ID.
pub const UTILITY_CRATE_FAMILY_ID: &str = "utility_crate";
/// Practical neutral style ID for Utility Crate.
pub const UTILITY_CRATE_STYLE_ID: &str = "utility_crate_practical";

const BODY_HALF_HEIGHT: f32 = 0.42;

/// Quality evidence used to gate novice catalog exposure for Utility Crate.
#[must_use]
pub const fn quality_evidence() -> StarterTemplateQualityEvidence {
    StarterTemplateQualityEvidence {
        profile_slug: UTILITY_CRATE_SLUG,
        visible_idea_count: 6,
        distinct_visible_idea_count: 5,
        primary_control_count: 7,
        endpoint_reported_primary_control_count: 7,
        endpoint_readable_primary_control_count: 7,
        returned_too_subtle_candidate_count: 0,
        broken_or_floating_part_count: 0,
        export_conformance_clean: true,
        advanced_recipe_required: false,
        raw_technical_summary_count: 0,
    }
}

/// Curation metadata for Utility Crate.
#[must_use]
pub fn curation_metadata() -> CatalogCurationMetadata {
    CatalogCurationMetadata {
        profile_slug: UTILITY_CRATE_SLUG,
        state: starter_template_curation_state_from_quality(quality_evidence()),
        has_visual_direction_evidence: true,
        has_readable_control_evidence: true,
        has_human_showcase_review: false,
        note: "Utility Crate v1 has clay evidence for seven novice-safe controls and six reusable crate directions.",
    }
}

/// Build the Utility Crate fixture catalog.
#[must_use]
pub fn fixture_catalog() -> FoundryFixtureCatalog {
    let family = family_schema(FamilySchemaSpec {
        id: UTILITY_CRATE_FAMILY_ID,
        display_name: "Utility Crate",
        summary: "Reusable practical clay crate family with body, lid, panel fields, trim bands, optional handles, latches, and feet or skids.",
        roles: vec![
            role("body", RoleMultiplicity::Single, true),
            role("lid", RoleMultiplicity::Single, true),
            role("panel_fields", RoleMultiplicity::Repeated, true),
            role("trim_bands", RoleMultiplicity::Repeated, false),
            role("handles", RoleMultiplicity::Repeated, false),
            role("latches", RoleMultiplicity::Repeated, false),
            role("feet_or_skids", RoleMultiplicity::Repeated, false),
            role("detail_marks", RoleMultiplicity::Repeated, false),
        ],
        allowed_operations: vec![
            AllowedOperationKind::Primitive,
            AllowedOperationKind::Array,
            AllowedOperationKind::Transform,
            AllowedOperationKind::Bevel,
        ],
        parameter_slots: vec![
            crate::ratio_slot("proportions", "Proportions", "body", 0.0, 1.0, 0.05, 0.48),
            choice_slot(
                "lid_style",
                "Lid Style",
                "lid",
                vec![
                    "flat_lid".to_owned(),
                    "raised_lid".to_owned(),
                    "rimmed_lid".to_owned(),
                ],
            ),
            choice_slot(
                "panel_style",
                "Panel Style",
                "panel_fields",
                vec![
                    "clean".to_owned(),
                    "shallow_panels".to_owned(),
                    "framed_panels".to_owned(),
                ],
            ),
            choice_slot(
                "trim_style",
                "Trim Style",
                "trim_bands",
                vec![
                    "none".to_owned(),
                    "simple_band".to_owned(),
                    "reinforced_band".to_owned(),
                ],
            ),
            choice_slot(
                "handle_style",
                "Handle Style",
                "handles",
                vec![
                    "none".to_owned(),
                    "cutout_grip".to_owned(),
                    "simple_side_handle".to_owned(),
                ],
            ),
            choice_slot(
                "latch_detail",
                "Latch Detail",
                "latches",
                vec![
                    "none".to_owned(),
                    "simple_latch".to_owned(),
                    "double_latch".to_owned(),
                ],
            ),
            choice_slot(
                "detail_density",
                "Detail Density",
                "detail_marks",
                vec![
                    "low_detail".to_owned(),
                    "medium_detail".to_owned(),
                    "high_detail".to_owned(),
                ],
            ),
        ],
        compatible_style_kits: vec![UTILITY_CRATE_STYLE_ID.to_owned()],
        tags: vec![
            "utility-crate".to_owned(),
            "reusable-family".to_owned(),
            "clay".to_owned(),
        ],
    });

    let style = style_kit(
        UTILITY_CRATE_STYLE_ID,
        "Utility Crate Practical",
        UTILITY_CRATE_FAMILY_ID,
        &style_prototypes(),
        vec![
            "utility-crate".to_owned(),
            "practical".to_owned(),
            "clay".to_owned(),
        ],
    );

    let family_impl = family_implementation(
        UTILITY_CRATE_FAMILY_ID,
        "Utility Crate reusable family",
        parameter_bindings(),
    );

    let style_impl = style_implementation(
        UTILITY_CRATE_STYLE_ID,
        UTILITY_CRATE_FAMILY_ID,
        default_provider_map(),
        recipe_fragments(),
    );

    let mut profile = crate::customizer_profile(
        UTILITY_CRATE_FAMILY_ID,
        UTILITY_CRATE_STYLE_ID,
        vec![
            continuous_control("proportions", "Proportions", "proportions", 0.48, 0.0, 1.0),
            choice_control(
                "lid_style",
                "Lid Style",
                "lid_style",
                &["flat_lid", "raised_lid", "rimmed_lid"],
            ),
            choice_control(
                "panel_style",
                "Panel Style",
                "panel_style",
                &["clean", "shallow_panels", "framed_panels"],
            ),
            choice_control(
                "trim_style",
                "Trim Style",
                "trim_style",
                &["none", "simple_band", "reinforced_band"],
            ),
            choice_control(
                "handle_style",
                "Handle Style",
                "handle_style",
                &["none", "cutout_grip", "simple_side_handle"],
            ),
            choice_control(
                "latch_detail",
                "Latch Detail",
                "latch_detail",
                &["none", "simple_latch", "double_latch"],
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
        strategy(
            "clean-storage-crate",
            "Clean Storage Crate",
            &["panel_style", "trim_style", "lid_style"],
        ),
        strategy(
            "reinforced-utility-crate",
            "Reinforced Utility Crate",
            &["trim_style", "latch_detail", "detail_density"],
        ),
        strategy(
            "compact-carry-crate",
            "Compact Carry Crate",
            &["proportions", "handle_style", "detail_density"],
        ),
        strategy(
            "wide-supply-crate",
            "Wide Supply Crate",
            &["proportions", "panel_style", "latch_detail"],
        ),
        strategy(
            "lidded-field-crate",
            "Lidded Field Crate",
            &["lid_style", "latch_detail", "trim_style"],
        ),
        strategy(
            "minimal-workshop-crate",
            "Minimal Workshop Crate",
            &[
                "panel_style",
                "trim_style",
                "handle_style",
                "latch_detail",
                "detail_density",
            ],
        ),
    ];

    build_fixture_catalog(FixtureCatalogSpec {
        slug: UTILITY_CRATE_SLUG,
        document_id: "utility-crate-doc",
        family,
        style,
        family_implementation: family_impl,
        style_implementation: style_impl,
        customizer_profile: profile,
        control_state: BTreeMap::from([
            ("proportions".to_owned(), ControlValue::Scalar(0.48)),
            (
                "lid_style".to_owned(),
                ControlValue::Choice("raised_lid".to_owned()),
            ),
            (
                "panel_style".to_owned(),
                ControlValue::Choice("shallow_panels".to_owned()),
            ),
            (
                "trim_style".to_owned(),
                ControlValue::Choice("simple_band".to_owned()),
            ),
            (
                "handle_style".to_owned(),
                ControlValue::Choice("cutout_grip".to_owned()),
            ),
            (
                "latch_detail".to_owned(),
                ControlValue::Choice("simple_latch".to_owned()),
            ),
            (
                "detail_density".to_owned(),
                ControlValue::Choice("medium_detail".to_owned()),
            ),
        ]),
    })
}

fn style_prototypes() -> Vec<(&'static str, &'static str, &'static str)> {
    vec![
        ("utility_crate_body", "Utility crate body", "body"),
        ("flat_lid", "Flat lid", "lid"),
        ("raised_lid", "Raised lid", "lid"),
        ("rimmed_lid", "Rimmed lid", "lid"),
        ("clean_panel_field", "Clean panel field", "panel_fields"),
        (
            "shallow_panel_fields",
            "Shallow panel fields",
            "panel_fields",
        ),
        ("framed_panel_fields", "Framed panel fields", "panel_fields"),
        ("no_trim", "No trim", "trim_bands"),
        ("simple_trim_band", "Simple trim band", "trim_bands"),
        ("reinforced_trim_band", "Reinforced trim band", "trim_bands"),
        ("no_handles", "No handles", "handles"),
        ("cutout_grip_handle", "Cutout grip", "handles"),
        ("simple_side_handle", "Simple side handle", "handles"),
        ("no_latches", "No latches", "latches"),
        ("simple_latch", "Simple latch", "latches"),
        ("double_latch", "Double latch", "latches"),
        ("no_feet", "No feet", "feet_or_skids"),
        ("small_feet", "Small feet", "feet_or_skids"),
        ("utility_skids", "Skids", "feet_or_skids"),
        ("low_detail_marks", "Low detail marks", "detail_marks"),
        ("medium_detail_marks", "Medium detail marks", "detail_marks"),
        ("high_detail_marks", "High detail marks", "detail_marks"),
    ]
}

fn default_provider_map() -> BTreeMap<String, String> {
    BTreeMap::from([
        ("body".to_owned(), "utility_crate_body".to_owned()),
        ("lid".to_owned(), "raised_lid".to_owned()),
        ("panel_fields".to_owned(), "shallow_panel_fields".to_owned()),
        ("trim_bands".to_owned(), "simple_trim_band".to_owned()),
        ("handles".to_owned(), "cutout_grip_handle".to_owned()),
        ("latches".to_owned(), "simple_latch".to_owned()),
        ("feet_or_skids".to_owned(), "small_feet".to_owned()),
        ("detail_marks".to_owned(), "medium_detail_marks".to_owned()),
    ])
}

fn parameter_bindings() -> Vec<ParameterBinding> {
    let mut bindings = vec![
        choice_binding(
            "lid_style",
            "lid",
            [
                ("flat_lid", "flat_lid"),
                ("raised_lid", "raised_lid"),
                ("rimmed_lid", "rimmed_lid"),
            ],
        ),
        choice_binding(
            "panel_style",
            "panel_fields",
            [
                ("clean", "clean_panel_field"),
                ("shallow_panels", "shallow_panel_fields"),
                ("framed_panels", "framed_panel_fields"),
            ],
        ),
        choice_binding(
            "trim_style",
            "trim_bands",
            [
                ("none", "no_trim"),
                ("simple_band", "simple_trim_band"),
                ("reinforced_band", "reinforced_trim_band"),
            ],
        ),
        choice_binding(
            "handle_style",
            "handles",
            [
                ("none", "no_handles"),
                ("cutout_grip", "cutout_grip_handle"),
                ("simple_side_handle", "simple_side_handle"),
            ],
        ),
        choice_binding(
            "latch_detail",
            "latches",
            [
                ("none", "no_latches"),
                ("simple_latch", "simple_latch"),
                ("double_latch", "double_latch"),
            ],
        ),
        choice_binding(
            "detail_density",
            "detail_marks",
            [
                ("low_detail", "low_detail_marks"),
                ("medium_detail", "medium_detail_marks"),
                ("high_detail", "high_detail_marks"),
            ],
        ),
        choice_binding(
            "detail_density",
            "feet_or_skids",
            [
                ("low_detail", "no_feet"),
                ("medium_detail", "small_feet"),
                ("high_detail", "utility_skids"),
            ],
        ),
    ];

    bindings.push(definition_binding(
        "proportions",
        "body",
        "geometry.rounded_box.half_extents.x",
        0.78,
        1.45,
    ));
    bindings.push(definition_binding(
        "proportions",
        "body",
        "geometry.rounded_box.half_extents.z",
        0.50,
        0.58,
    ));

    bindings
}

fn choice_binding<const N: usize>(
    slot: &str,
    role_name: &str,
    choices: [(&str, &str); N],
) -> ParameterBinding {
    ParameterBinding::ChoiceToPrototype {
        slot: slot.to_owned(),
        role: role_name.to_owned(),
        choices: choices
            .into_iter()
            .map(|(choice, provider)| (choice.to_owned(), provider.to_owned()))
            .collect(),
    }
}

fn definition_binding(
    slot: &str,
    role_name: &str,
    local_key: &str,
    minimum: f32,
    maximum: f32,
) -> ParameterBinding {
    ParameterBinding::Scalar {
        slot: slot.to_owned(),
        role: role_name.to_owned(),
        local_path: definition_scalar_path(crate::LOCAL_DEFINITION, local_key),
        transform: ScalarTransform::Ratio { minimum, maximum },
    }
}

fn strategy(id: &str, label: &str, control_ids: &[&str]) -> CandidateStrategy {
    CandidateStrategy {
        id: id.to_owned(),
        label: label.to_owned(),
        control_ids: control_ids
            .iter()
            .map(|control_id| (*control_id).to_owned())
            .collect(),
    }
}

fn recipe_fragments() -> Vec<RecipeFragment> {
    vec![
        rounded_box_fragment(
            "utility_crate_body",
            "body",
            [1.05, BODY_HALF_HEIGHT, 0.62],
            0.055,
            [0.0, 0.0, 0.0],
            Vec::new(),
        ),
        rounded_box_fragment(
            "flat_lid",
            "lid",
            [1.02, 0.055, 0.60],
            0.026,
            [0.0, BODY_HALF_HEIGHT + 0.055, 0.0],
            Vec::new(),
        ),
        rounded_box_fragment(
            "raised_lid",
            "lid",
            [1.18, 0.13, 0.68],
            0.032,
            [0.0, BODY_HALF_HEIGHT + 0.13, 0.0],
            Vec::new(),
        ),
        rounded_box_fragment(
            "rimmed_lid",
            "lid",
            [1.42, 0.08, 0.82],
            0.024,
            [0.0, BODY_HALF_HEIGHT + 0.08, 0.0],
            Vec::new(),
        ),
        clean_panel_fragment(),
        shallow_panel_fragment(),
        framed_panel_fragment(),
        disabled_fragment("no_trim", "trim_bands", "trim disabled marker"),
        simple_trim_fragment(),
        reinforced_trim_fragment(),
        disabled_fragment("no_handles", "handles", "handle disabled marker"),
        cutout_grip_fragment(),
        side_handle_fragment(),
        disabled_fragment("no_latches", "latches", "latch disabled marker"),
        simple_latch_fragment(),
        double_latch_fragment(),
        disabled_fragment("no_feet", "feet_or_skids", "feet disabled marker"),
        small_feet_fragment(),
        skid_fragment(),
        detail_fragment("low_detail_marks", 2),
        detail_fragment("medium_detail_marks", 6),
        detail_fragment("high_detail_marks", 12),
    ]
}

fn clean_panel_fragment() -> RecipeFragment {
    box_assembly_fragment(
        "clean_panel_field",
        "panel_fields",
        &[BoxPart::new(
            90,
            91,
            "front clean panel field",
            [0.78, 0.24, 0.035],
            [0.0, 0.02, 0.64],
            true,
        )],
        0.018,
    )
}

fn shallow_panel_fragment() -> RecipeFragment {
    box_assembly_fragment(
        "shallow_panel_fields",
        "panel_fields",
        &[
            BoxPart::new(
                90,
                91,
                "left shallow panel field",
                [0.34, 0.24, 0.04],
                [-0.42, 0.03, 0.64],
                true,
            ),
            BoxPart::new(
                92,
                93,
                "right shallow panel field",
                [0.34, 0.24, 0.04],
                [0.42, 0.03, 0.64],
                true,
            ),
            BoxPart::new(
                94,
                95,
                "panel center divider",
                [0.035, 0.30, 0.045],
                [0.0, 0.03, 0.645],
                true,
            ),
        ],
        0.016,
    )
}

fn framed_panel_fragment() -> RecipeFragment {
    box_assembly_fragment(
        "framed_panel_fields",
        "panel_fields",
        &[
            BoxPart::new(
                90,
                91,
                "left framed panel field",
                [0.33, 0.20, 0.035],
                [-0.42, 0.02, 0.64],
                true,
            ),
            BoxPart::new(
                92,
                93,
                "right framed panel field",
                [0.33, 0.20, 0.035],
                [0.42, 0.02, 0.64],
                true,
            ),
            BoxPart::new(
                94,
                95,
                "upper panel frame rail",
                [0.86, 0.045, 0.035],
                [0.0, 0.275, 0.68],
                true,
            ),
            BoxPart::new(
                96,
                97,
                "lower panel frame rail",
                [0.86, 0.045, 0.035],
                [0.0, -0.245, 0.68],
                true,
            ),
            BoxPart::new(
                98,
                99,
                "left panel frame upright",
                [0.045, 0.27, 0.035],
                [-0.86, 0.02, 0.76],
                true,
            ),
            BoxPart::new(
                100,
                101,
                "right panel frame upright",
                [0.045, 0.27, 0.035],
                [0.86, 0.02, 0.76],
                true,
            ),
            BoxPart::new(
                102,
                103,
                "center panel frame upright",
                [0.04, 0.27, 0.035],
                [0.0, 0.02, 0.76],
                true,
            ),
        ],
        0.016,
    )
}

fn simple_trim_fragment() -> RecipeFragment {
    box_assembly_fragment(
        "simple_trim_band",
        "trim_bands",
        &[
            BoxPart::new(
                90,
                91,
                "front simple trim band",
                [0.94, 0.045, 0.03],
                [0.0, 0.25, 0.84],
                true,
            ),
            BoxPart::new(
                92,
                93,
                "rear simple trim band",
                [0.94, 0.045, 0.03],
                [0.0, 0.25, -0.84],
                true,
            ),
        ],
        0.014,
    )
}

fn reinforced_trim_fragment() -> RecipeFragment {
    box_assembly_fragment(
        "reinforced_trim_band",
        "trim_bands",
        &[
            BoxPart::new(
                90,
                91,
                "front upper reinforced trim band",
                [1.02, 0.055, 0.035],
                [0.0, 0.30, 0.85],
                true,
            ),
            BoxPart::new(
                92,
                93,
                "rear upper reinforced trim band",
                [1.02, 0.055, 0.035],
                [0.0, 0.30, -0.85],
                true,
            ),
            BoxPart::new(
                98,
                99,
                "front lower reinforced trim band",
                [1.00, 0.05, 0.035],
                [0.0, -0.28, 0.84],
                true,
            ),
            BoxPart::new(
                100,
                101,
                "rear lower reinforced trim band",
                [1.00, 0.05, 0.035],
                [0.0, -0.28, -0.84],
                true,
            ),
        ],
        0.014,
    )
}

fn cutout_grip_fragment() -> RecipeFragment {
    box_assembly_fragment(
        "cutout_grip_handle",
        "handles",
        &[
            BoxPart::new(
                90,
                91,
                "front cutout grip plate",
                [0.30, 0.045, 0.025],
                [0.0, 0.09, 0.92],
                true,
            ),
            BoxPart::new(
                92,
                93,
                "left cutout grip cheek",
                [0.045, 0.12, 0.025],
                [-0.37, 0.09, 0.925],
                true,
            ),
            BoxPart::new(
                94,
                95,
                "right cutout grip cheek",
                [0.045, 0.12, 0.025],
                [0.37, 0.09, 0.925],
                true,
            ),
        ],
        0.015,
    )
}

fn side_handle_fragment() -> RecipeFragment {
    box_assembly_fragment(
        "simple_side_handle",
        "handles",
        &[
            BoxPart::new(
                90,
                91,
                "left side handle grip",
                [0.045, 0.11, 0.025],
                [-0.64, 0.08, 0.925],
                true,
            ),
            BoxPart::new(
                92,
                93,
                "left side handle upper mount",
                [0.10, 0.045, 0.025],
                [-0.64, 0.245, 0.93],
                true,
            ),
            BoxPart::new(
                94,
                95,
                "left side handle lower mount",
                [0.10, 0.045, 0.025],
                [-0.64, -0.085, 0.93],
                true,
            ),
            BoxPart::new(
                96,
                97,
                "right side handle grip",
                [0.045, 0.11, 0.025],
                [0.64, 0.08, 0.925],
                true,
            ),
            BoxPart::new(
                98,
                99,
                "right side handle upper mount",
                [0.10, 0.045, 0.025],
                [0.64, 0.245, 0.93],
                true,
            ),
            BoxPart::new(
                100,
                101,
                "right side handle lower mount",
                [0.10, 0.045, 0.025],
                [0.64, -0.085, 0.93],
                true,
            ),
        ],
        0.015,
    )
}

fn simple_latch_fragment() -> RecipeFragment {
    box_assembly_fragment(
        "simple_latch",
        "latches",
        &[
            BoxPart::new(
                90,
                91,
                "center latch body",
                [0.16, 0.045, 0.025],
                [0.0, 0.36, 0.96],
                true,
            ),
            BoxPart::new(
                92,
                93,
                "center latch pull",
                [0.09, 0.025, 0.025],
                [0.0, 0.265, 0.965],
                true,
            ),
        ],
        0.012,
    )
}

fn double_latch_fragment() -> RecipeFragment {
    box_assembly_fragment(
        "double_latch",
        "latches",
        &[
            BoxPart::new(
                90,
                91,
                "left latch body",
                [0.15, 0.045, 0.025],
                [-0.44, 0.36, 0.96],
                true,
            ),
            BoxPart::new(
                92,
                93,
                "left latch pull",
                [0.08, 0.025, 0.025],
                [-0.44, 0.265, 0.965],
                true,
            ),
            BoxPart::new(
                94,
                95,
                "right latch body",
                [0.15, 0.045, 0.025],
                [0.44, 0.36, 0.96],
                true,
            ),
            BoxPart::new(
                96,
                97,
                "right latch pull",
                [0.08, 0.025, 0.025],
                [0.44, 0.265, 0.965],
                true,
            ),
        ],
        0.012,
    )
}

fn small_feet_fragment() -> RecipeFragment {
    box_assembly_fragment(
        "small_feet",
        "feet_or_skids",
        &[
            BoxPart::new(
                90,
                91,
                "front left small foot",
                [0.15, 0.06, 0.13],
                [-0.58, -BODY_HALF_HEIGHT - 0.06, 0.42],
                true,
            ),
            BoxPart::new(
                92,
                93,
                "front right small foot",
                [0.15, 0.06, 0.13],
                [0.58, -BODY_HALF_HEIGHT - 0.06, 0.42],
                true,
            ),
            BoxPart::new(
                94,
                95,
                "rear left small foot",
                [0.15, 0.06, 0.13],
                [-0.58, -BODY_HALF_HEIGHT - 0.06, -0.42],
                true,
            ),
            BoxPart::new(
                96,
                97,
                "rear right small foot",
                [0.15, 0.06, 0.13],
                [0.58, -BODY_HALF_HEIGHT - 0.06, -0.42],
                true,
            ),
        ],
        0.015,
    )
}

fn skid_fragment() -> RecipeFragment {
    box_assembly_fragment(
        "utility_skids",
        "feet_or_skids",
        &[
            BoxPart::new(
                90,
                91,
                "left utility skid",
                [0.16, 0.07, 0.58],
                [-0.48, -BODY_HALF_HEIGHT - 0.07, 0.0],
                true,
            ),
            BoxPart::new(
                92,
                93,
                "right utility skid",
                [0.16, 0.07, 0.58],
                [0.48, -BODY_HALF_HEIGHT - 0.07, 0.0],
                true,
            ),
        ],
        0.018,
    )
}

fn detail_fragment(id: &str, count: usize) -> RecipeFragment {
    let columns = if count <= 2 {
        2
    } else if count <= 6 {
        3
    } else {
        6
    };
    let rows = count.div_ceil(columns);
    let mut parts = Vec::with_capacity(count);
    for index in 0..count {
        let column = index % columns;
        let row = index / columns;
        let x = (column as f32 - (columns as f32 - 1.0) * 0.5) * 0.25;
        let y = -0.18 + row as f32 * 0.18 - (rows as f32 - 1.0) * 0.04;
        parts.push(BoxPart::new(
            90 + index as u64 * 2,
            91 + index as u64 * 2,
            "small clay detail mark",
            [0.045, 0.035, 0.032],
            [x, y, 1.02],
            true,
        ));
    }
    box_assembly_fragment(id, "detail_marks", &parts, 0.01)
}

fn disabled_fragment(id: &str, role_name: &str, name: &'static str) -> RecipeFragment {
    box_assembly_fragment(
        id,
        role_name,
        &[BoxPart::new(
            90,
            91,
            name,
            [0.01, 0.01, 0.01],
            [0.0, -1.2, 0.0],
            false,
        )],
        0.001,
    )
}

#[derive(Debug, Copy, Clone)]
struct BoxPart {
    definition: PartDefinitionId,
    instance: PartInstanceId,
    name: &'static str,
    half_extents: [f32; 3],
    translation: [f32; 3],
    enabled: bool,
}

impl BoxPart {
    const fn new(
        definition: u64,
        instance: u64,
        name: &'static str,
        half_extents: [f32; 3],
        translation: [f32; 3],
        enabled: bool,
    ) -> Self {
        Self {
            definition: PartDefinitionId(definition),
            instance: PartInstanceId(instance),
            name,
            half_extents,
            translation,
            enabled,
        }
    }
}

fn box_assembly_fragment(
    id: &str,
    role_name: &str,
    parts: &[BoxPart],
    radius: f32,
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
                        radius,
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
                parent: None,
                local_transform: Transform3 {
                    translation: part.translation,
                    ..Transform3::default()
                },
                attachment: None,
                enabled: part.enabled,
                tags: BTreeSet::from([role_name.to_owned(), format!("role:{role_name}")]),
                generated_by: None,
            },
        );
        for (path, label) in [
            ("geometry.rounded_box.half_extents.x", "width"),
            ("geometry.rounded_box.half_extents.y", "height"),
            ("geometry.rounded_box.half_extents.z", "depth"),
        ] {
            recipe.parameters.insert(
                shape_asset::ParameterId(recipe.next_ids.parameter),
                shape_family_compile::scalar_parameter(
                    recipe.next_ids.parameter,
                    definition_scalar_path(part.definition, path),
                    format!("{} {label}", part.name),
                    0.01,
                    5.0,
                    0.01,
                    false,
                ),
            );
            recipe.next_ids.parameter += 1;
        }
    }
    recipe
        .root_instances
        .extend(parts.iter().map(|part| part.instance));
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
