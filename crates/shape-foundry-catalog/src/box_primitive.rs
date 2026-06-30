//! Box Primitive baseline fixture.

use std::collections::BTreeMap;

use shape_asset::{PartDefinitionId, definition_scalar_path};
use shape_family::{AllowedOperationKind, RoleMultiplicity};
use shape_family_compile::{ParameterBinding, RecipeFragment, ScalarTransform};
use shape_foundry::{CandidateStrategy, ControlValue};

use crate::{
    CatalogCurationMetadata, FamilySchemaSpec, FixtureCatalogSpec, FoundryFixtureCatalog,
    StarterTemplateQualityEvidence, build_fixture_catalog, choice_control, choice_slot,
    continuous_control, family_implementation, family_schema, role, rounded_box_fragment,
    starter_template_curation_state_from_quality, style_implementation, style_kit,
};

/// Box Primitive profile slug.
pub const BOX_PRIMITIVE_SLUG: &str = "box-primitive";
/// Box Primitive family ID.
pub const BOX_PRIMITIVE_FAMILY_ID: &str = "box_primitive";
/// Neutral clay style ID for Box Primitive.
pub const BOX_PRIMITIVE_STYLE_ID: &str = "box_primitive_clay";
/// Lidded Box preview profile slug.
pub const LIDDED_BOX_SLUG: &str = "lidded-box";
/// Lidded Box preview family ID.
pub const LIDDED_BOX_FAMILY_ID: &str = "lidded_box";
/// Neutral clay style ID for Lidded Box.
pub const LIDDED_BOX_STYLE_ID: &str = "lidded_box_clay";
/// Internal Lid Seam module ID.
pub const LID_SEAM_MODULE_ID: &str = "lid-seam";

#[derive(Debug, Copy, Clone)]
struct BoxProportion {
    choice: &'static str,
    provider: &'static str,
    half_extents: [f32; 3],
}

#[derive(Debug, Copy, Clone)]
struct LiddedBoxProportion {
    choice: &'static str,
    body_provider: &'static str,
    seam_provider: &'static str,
    body_half_extents: [f32; 3],
    lid_half_extents: [f32; 3],
    body_translation: [f32; 3],
    lid_translation: [f32; 3],
}

const PROPORTIONS: [BoxProportion; 4] = [
    BoxProportion {
        choice: "compact_box",
        provider: "compact_body",
        half_extents: [0.78, 0.48, 0.58],
    },
    BoxProportion {
        choice: "wide_box",
        provider: "wide_body",
        half_extents: [1.28, 0.42, 0.56],
    },
    BoxProportion {
        choice: "tall_box",
        provider: "tall_body",
        half_extents: [0.62, 0.78, 0.48],
    },
    BoxProportion {
        choice: "flat_box",
        provider: "flat_body",
        half_extents: [1.30, 0.26, 0.78],
    },
];

const LIDDED_PROPORTIONS: [LiddedBoxProportion; 4] = [
    LiddedBoxProportion {
        choice: "compact_box",
        body_provider: "compact_lidded_body",
        seam_provider: "compact_lid_seam",
        body_half_extents: [0.78, 0.48, 0.58],
        lid_half_extents: [0.64, 0.08, 0.055],
        body_translation: [0.0, 0.48, 0.0],
        lid_translation: [0.0, 0.74, 0.637],
    },
    LiddedBoxProportion {
        choice: "wide_box",
        body_provider: "wide_lidded_body",
        seam_provider: "wide_lid_seam",
        body_half_extents: [1.28, 0.42, 0.56],
        lid_half_extents: [1.08, 0.08, 0.055],
        body_translation: [0.0, 0.42, 0.0],
        lid_translation: [0.0, 0.65, 0.617],
    },
    LiddedBoxProportion {
        choice: "tall_box",
        body_provider: "tall_lidded_body",
        seam_provider: "tall_lid_seam",
        body_half_extents: [0.62, 0.78, 0.48],
        lid_half_extents: [0.50, 0.08, 0.055],
        body_translation: [0.0, 0.78, 0.0],
        lid_translation: [0.0, 1.20, 0.537],
    },
    LiddedBoxProportion {
        choice: "flat_box",
        body_provider: "flat_lidded_body",
        seam_provider: "flat_lid_seam",
        body_half_extents: [1.30, 0.26, 0.78],
        lid_half_extents: [1.10, 0.08, 0.055],
        body_translation: [0.0, 0.26, 0.0],
        lid_translation: [0.0, 0.40, 0.837],
    },
];

/// Quality evidence used to gate novice catalog exposure for Box Primitive.
#[must_use]
pub const fn quality_evidence() -> StarterTemplateQualityEvidence {
    StarterTemplateQualityEvidence {
        profile_slug: BOX_PRIMITIVE_SLUG,
        visible_idea_count: 6,
        distinct_visible_idea_count: 6,
        primary_control_count: 2,
        endpoint_reported_primary_control_count: 2,
        endpoint_readable_primary_control_count: 2,
        returned_too_subtle_candidate_count: 0,
        broken_or_floating_part_count: 0,
        export_conformance_clean: true,
        advanced_recipe_required: false,
        raw_technical_summary_count: 0,
    }
}

/// Curation metadata for Box Primitive.
#[must_use]
pub fn curation_metadata() -> CatalogCurationMetadata {
    CatalogCurationMetadata {
        profile_slug: BOX_PRIMITIVE_SLUG,
        state: starter_template_curation_state_from_quality(quality_evidence()),
        has_visual_direction_evidence: true,
        has_readable_control_evidence: true,
        has_human_showcase_review: false,
        note: "Box Primitive is the honest novice baseline: a pure-clay closed box with readable proportions and edge softness.",
    }
}

/// Build the Box Primitive fixture catalog.
#[must_use]
pub fn fixture_catalog() -> FoundryFixtureCatalog {
    let family = family_schema(FamilySchemaSpec {
        id: BOX_PRIMITIVE_FAMILY_ID,
        display_name: "Box Primitive",
        summary: "Pure clay closed box primitive with readable width, depth, height, and edge softness.",
        roles: vec![role("body", RoleMultiplicity::Single, true)],
        allowed_operations: vec![
            AllowedOperationKind::Primitive,
            AllowedOperationKind::Array,
            AllowedOperationKind::Transform,
            AllowedOperationKind::Bevel,
        ],
        parameter_slots: vec![
            choice_slot(
                "proportions",
                "Proportions",
                "body",
                PROPORTIONS
                    .iter()
                    .map(|proportion| proportion.choice.to_owned())
                    .collect(),
            ),
            crate::ratio_slot(
                "edge_softness",
                "Edge Softness",
                "body",
                0.0,
                1.0,
                0.05,
                0.35,
            ),
        ],
        compatible_style_kits: vec![BOX_PRIMITIVE_STYLE_ID.to_owned()],
        tags: vec![
            "box-primitive".to_owned(),
            "primitive-family".to_owned(),
            "clay".to_owned(),
        ],
    });

    let style = style_kit(
        BOX_PRIMITIVE_STYLE_ID,
        "Box Primitive Clay",
        BOX_PRIMITIVE_FAMILY_ID,
        &style_prototypes(),
        vec!["box-primitive".to_owned(), "clay".to_owned()],
    );

    let family_impl = family_implementation(
        BOX_PRIMITIVE_FAMILY_ID,
        "Box Primitive family",
        parameter_bindings(),
    );

    let style_impl = style_implementation(
        BOX_PRIMITIVE_STYLE_ID,
        BOX_PRIMITIVE_FAMILY_ID,
        default_provider_map(),
        recipe_fragments(),
    );

    let mut profile = crate::customizer_profile(
        BOX_PRIMITIVE_FAMILY_ID,
        BOX_PRIMITIVE_STYLE_ID,
        vec![
            choice_control(
                "proportions",
                "Proportions",
                "proportions",
                &["compact_box", "wide_box", "tall_box", "flat_box"],
            ),
            continuous_control(
                "edge_softness",
                "Edge Softness",
                "edge_softness",
                0.35,
                0.0,
                1.0,
            ),
        ],
    );
    profile.candidate_strategies = vec![
        strategy("compact-box", "Compact Box", &["proportions"]),
        strategy("wide-box", "Wide Box", &["proportions"]),
        strategy("tall-box", "Tall Box", &["proportions"]),
        strategy("flat-box", "Flat Box", &["proportions"]),
        strategy("soft-edged-box", "Soft-Edged Box", &["edge_softness"]),
        strategy("sharp-box", "Sharp Box", &["edge_softness"]),
    ];

    build_fixture_catalog(FixtureCatalogSpec {
        slug: BOX_PRIMITIVE_SLUG,
        document_id: "box-primitive-doc",
        family,
        style,
        family_implementation: family_impl,
        style_implementation: style_impl,
        customizer_profile: profile,
        control_state: BTreeMap::from([
            (
                "proportions".to_owned(),
                ControlValue::Choice("compact_box".to_owned()),
            ),
            ("edge_softness".to_owned(), ControlValue::Scalar(0.35)),
        ]),
    })
}

/// Build the preview Lidded Box fixture catalog.
///
/// This is not included in the default novice catalog until the Make baseline
/// gate explicitly surfaces it.
#[must_use]
pub fn lidded_box_fixture_catalog() -> FoundryFixtureCatalog {
    let family = family_schema(FamilySchemaSpec {
        id: LIDDED_BOX_FAMILY_ID,
        display_name: "Lidded Box",
        summary: "A simple box with a visible lid seam.",
        roles: vec![
            role("body", RoleMultiplicity::Single, true),
            role("lid_seam", RoleMultiplicity::Single, true),
        ],
        allowed_operations: vec![
            AllowedOperationKind::Primitive,
            AllowedOperationKind::Array,
            AllowedOperationKind::Transform,
            AllowedOperationKind::Bevel,
        ],
        parameter_slots: vec![
            choice_slot(
                "proportions",
                "Proportions",
                "body",
                LIDDED_PROPORTIONS
                    .iter()
                    .map(|proportion| proportion.choice.to_owned())
                    .collect(),
            ),
            crate::ratio_slot(
                "edge_softness",
                "Edge Softness",
                "body",
                0.0,
                1.0,
                0.05,
                0.35,
            ),
            crate::ratio_slot("lid_height", "Lid Seam", "lid_seam", 0.0, 1.0, 0.05, 0.35),
        ],
        compatible_style_kits: vec![LIDDED_BOX_STYLE_ID.to_owned()],
        tags: vec![
            "lidded-box".to_owned(),
            "lid-seam".to_owned(),
            "clay".to_owned(),
        ],
    });

    let style = style_kit(
        LIDDED_BOX_STYLE_ID,
        "Lidded Box Clay",
        LIDDED_BOX_FAMILY_ID,
        &lidded_style_prototypes(),
        vec![
            "lidded-box".to_owned(),
            "lid-seam".to_owned(),
            "clay".to_owned(),
        ],
    );

    let family_impl = family_implementation(
        LIDDED_BOX_FAMILY_ID,
        "Lidded Box family",
        lidded_parameter_bindings(),
    );

    let style_impl = style_implementation(
        LIDDED_BOX_STYLE_ID,
        LIDDED_BOX_FAMILY_ID,
        lidded_default_provider_map(),
        lidded_recipe_fragments(),
    );

    let mut profile = crate::customizer_profile(
        LIDDED_BOX_FAMILY_ID,
        LIDDED_BOX_STYLE_ID,
        vec![
            choice_control(
                "proportions",
                "Proportions",
                "proportions",
                &["compact_box", "wide_box", "tall_box", "flat_box"],
            ),
            continuous_control(
                "edge_softness",
                "Edge Softness",
                "edge_softness",
                0.35,
                0.0,
                1.0,
            ),
            continuous_control("lid_height", "Lid Seam", "lid_height", 0.35, 0.0, 1.0),
        ],
    );
    profile.candidate_strategies = vec![
        strategy("low-lid-box", "Low Lid Box", &["lid_height"]),
        strategy("raised-lid-box", "Raised Lid Box", &["lid_height"]),
        strategy("compact-lidded-box", "Compact Lidded Box", &["proportions"]),
        strategy("wide-lidded-box", "Wide Lidded Box", &["proportions"]),
        strategy("flat-storage-box", "Flat Storage Box", &["proportions"]),
        strategy(
            "soft-edged-lidded-box",
            "Soft-Edged Lidded Box",
            &["edge_softness"],
        ),
    ];

    build_fixture_catalog(FixtureCatalogSpec {
        slug: LIDDED_BOX_SLUG,
        document_id: "lidded-box-doc",
        family,
        style,
        family_implementation: family_impl,
        style_implementation: style_impl,
        customizer_profile: profile,
        control_state: BTreeMap::from([
            (
                "proportions".to_owned(),
                ControlValue::Choice("compact_box".to_owned()),
            ),
            ("edge_softness".to_owned(), ControlValue::Scalar(0.35)),
            ("lid_height".to_owned(), ControlValue::Scalar(0.35)),
        ]),
    })
}

fn style_prototypes() -> Vec<(&'static str, &'static str, &'static str)> {
    PROPORTIONS
        .into_iter()
        .map(|proportion| (proportion.provider, "Closed box body", "body"))
        .collect()
}

fn lidded_style_prototypes() -> Vec<(&'static str, &'static str, &'static str)> {
    LIDDED_PROPORTIONS
        .into_iter()
        .flat_map(|proportion| {
            [
                (proportion.body_provider, "Closed lower box body", "body"),
                (
                    proportion.seam_provider,
                    "Raised clay line for visible lid seam",
                    "lid_seam",
                ),
            ]
        })
        .collect()
}

fn default_provider_map() -> BTreeMap<String, String> {
    BTreeMap::from([("body".to_owned(), "compact_body".to_owned())])
}

fn lidded_default_provider_map() -> BTreeMap<String, String> {
    BTreeMap::from([
        ("body".to_owned(), "compact_lidded_body".to_owned()),
        ("lid_seam".to_owned(), "compact_lid_seam".to_owned()),
    ])
}

fn parameter_bindings() -> Vec<ParameterBinding> {
    vec![
        ParameterBinding::ChoiceToPrototype {
            slot: "proportions".to_owned(),
            role: "body".to_owned(),
            choices: PROPORTIONS
                .into_iter()
                .map(|proportion| (proportion.choice.to_owned(), proportion.provider.to_owned()))
                .collect(),
        },
        definition_binding(
            "edge_softness",
            "body",
            crate::LOCAL_DEFINITION,
            "geometry.rounded_box.radius",
            0.004,
            0.16,
        ),
    ]
}

fn lidded_parameter_bindings() -> Vec<ParameterBinding> {
    vec![
        ParameterBinding::ChoiceToPrototype {
            slot: "proportions".to_owned(),
            role: "body".to_owned(),
            choices: LIDDED_PROPORTIONS
                .into_iter()
                .map(|proportion| {
                    (
                        proportion.choice.to_owned(),
                        proportion.body_provider.to_owned(),
                    )
                })
                .collect(),
        },
        ParameterBinding::ChoiceToPrototype {
            slot: "proportions".to_owned(),
            role: "lid_seam".to_owned(),
            choices: LIDDED_PROPORTIONS
                .into_iter()
                .map(|proportion| {
                    (
                        proportion.choice.to_owned(),
                        proportion.seam_provider.to_owned(),
                    )
                })
                .collect(),
        },
        definition_binding(
            "edge_softness",
            "body",
            crate::LOCAL_DEFINITION,
            "geometry.rounded_box.radius",
            0.004,
            0.16,
        ),
        definition_binding(
            "edge_softness",
            "lid_seam",
            crate::LOCAL_DEFINITION,
            "geometry.rounded_box.radius",
            0.003,
            0.028,
        ),
        definition_binding(
            "lid_height",
            "lid_seam",
            crate::LOCAL_DEFINITION,
            "geometry.rounded_box.half_extents.y",
            0.055,
            0.145,
        ),
        definition_binding(
            "lid_height",
            "lid_seam",
            crate::LOCAL_DEFINITION,
            "geometry.rounded_box.half_extents.x",
            0.45,
            0.92,
        ),
    ]
}

fn definition_binding(
    slot: &str,
    role_name: &str,
    definition: PartDefinitionId,
    local_key: &str,
    minimum: f32,
    maximum: f32,
) -> ParameterBinding {
    ParameterBinding::Scalar {
        slot: slot.to_owned(),
        role: role_name.to_owned(),
        local_path: definition_scalar_path(definition, local_key),
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
    PROPORTIONS
        .into_iter()
        .map(|proportion| {
            rounded_box_fragment(
                proportion.provider,
                "body",
                proportion.half_extents,
                0.055,
                [0.0, proportion.half_extents[1], 0.0],
                Vec::new(),
            )
        })
        .collect()
}

fn lidded_recipe_fragments() -> Vec<RecipeFragment> {
    LIDDED_PROPORTIONS
        .into_iter()
        .flat_map(|proportion| {
            [
                rounded_box_fragment(
                    proportion.body_provider,
                    "body",
                    proportion.body_half_extents,
                    0.05,
                    proportion.body_translation,
                    Vec::new(),
                ),
                rounded_box_fragment(
                    proportion.seam_provider,
                    "lid_seam",
                    proportion.lid_half_extents,
                    0.035,
                    proportion.lid_translation,
                    Vec::new(),
                ),
            ]
        })
        .collect()
}
