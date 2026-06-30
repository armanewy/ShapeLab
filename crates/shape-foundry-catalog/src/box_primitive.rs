//! Box Primitive baseline fixture.

use std::collections::BTreeMap;

use shape_asset::{PartDefinitionId, definition_scalar_path};
use shape_family::{AllowedOperationKind, RoleMultiplicity};
use shape_family_compile::{ParameterBinding, RecipeFragment, ScalarTransform};
use shape_foundry::{CandidateStrategy, ControlValue};

use crate::{
    CatalogCurationMetadata, FamilySchemaSpec, FixtureCatalogSpec, FoundryFixtureCatalog,
    StarterTemplateQualityEvidence, build_fixture_catalog, choice_control, choice_slot,
    continuous_control, family_implementation, family_schema, length_slot, role,
    rounded_box_fragment, starter_template_curation_state_from_quality, style_implementation,
    style_kit,
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
/// Trimmed Box internal preview profile slug.
pub const TRIMMED_BOX_SLUG: &str = "trimmed-box";
/// Trimmed Box internal preview family ID.
pub const TRIMMED_BOX_FAMILY_ID: &str = "trimmed_box";
/// Neutral clay style ID for Trimmed Box.
pub const TRIMMED_BOX_STYLE_ID: &str = "trimmed_box_clay";
/// Internal Lid Seam module ID.
pub const LID_SEAM_MODULE_ID: &str = "lid-seam";
/// Internal Trim Band module ID.
pub const TRIM_BAND_MODULE_ID: &str = "trim-band";

#[derive(Debug, Copy, Clone)]
struct BoxProportion {
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

#[derive(Debug, Copy, Clone)]
struct TrimmedBoxProportion {
    choice: &'static str,
    body_provider: &'static str,
    seam_provider: &'static str,
    trim_provider: &'static str,
    body_half_extents: [f32; 3],
    lid_half_extents: [f32; 3],
    trim_half_extents: [f32; 3],
    body_translation: [f32; 3],
    lid_translation: [f32; 3],
    trim_translation: [f32; 3],
}

const PROPORTIONS: [BoxProportion; 4] = [
    BoxProportion {
        provider: "compact_body",
        half_extents: [0.78, 0.48, 0.58],
    },
    BoxProportion {
        provider: "wide_body",
        half_extents: [1.28, 0.42, 0.56],
    },
    BoxProportion {
        provider: "tall_body",
        half_extents: [0.62, 0.78, 0.48],
    },
    BoxProportion {
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

const TRIMMED_PROPORTIONS: [TrimmedBoxProportion; 4] = [
    TrimmedBoxProportion {
        choice: "compact_box",
        body_provider: "compact_trimmed_body",
        seam_provider: "compact_trimmed_lid_seam",
        trim_provider: "compact_trim_band",
        body_half_extents: [0.78, 0.48, 0.58],
        lid_half_extents: [0.64, 0.08, 0.055],
        trim_half_extents: [0.82, 0.065, 0.06],
        body_translation: [0.0, 0.48, 0.0],
        lid_translation: [0.0, 0.74, 0.637],
        trim_translation: [0.0, 0.38, 0.644],
    },
    TrimmedBoxProportion {
        choice: "wide_box",
        body_provider: "wide_trimmed_body",
        seam_provider: "wide_trimmed_lid_seam",
        trim_provider: "wide_trim_band",
        body_half_extents: [1.28, 0.42, 0.56],
        lid_half_extents: [1.08, 0.08, 0.055],
        trim_half_extents: [1.34, 0.06, 0.06],
        body_translation: [0.0, 0.42, 0.0],
        lid_translation: [0.0, 0.65, 0.617],
        trim_translation: [0.0, 0.34, 0.624],
    },
    TrimmedBoxProportion {
        choice: "tall_box",
        body_provider: "tall_trimmed_body",
        seam_provider: "tall_trimmed_lid_seam",
        trim_provider: "tall_trim_band",
        body_half_extents: [0.62, 0.78, 0.48],
        lid_half_extents: [0.50, 0.08, 0.055],
        trim_half_extents: [0.68, 0.075, 0.06],
        body_translation: [0.0, 0.78, 0.0],
        lid_translation: [0.0, 1.20, 0.537],
        trim_translation: [0.0, 0.56, 0.544],
    },
    TrimmedBoxProportion {
        choice: "flat_box",
        body_provider: "flat_trimmed_body",
        seam_provider: "flat_trimmed_lid_seam",
        trim_provider: "flat_trim_band",
        body_half_extents: [1.30, 0.26, 0.78],
        lid_half_extents: [1.10, 0.08, 0.055],
        trim_half_extents: [1.36, 0.055, 0.065],
        body_translation: [0.0, 0.26, 0.0],
        lid_translation: [0.0, 0.40, 0.837],
        trim_translation: [0.0, 0.22, 0.849],
    },
];

/// Quality evidence used to gate novice catalog exposure for Box Primitive.
#[must_use]
pub const fn quality_evidence() -> StarterTemplateQualityEvidence {
    StarterTemplateQualityEvidence {
        profile_slug: BOX_PRIMITIVE_SLUG,
        visible_idea_count: 6,
        distinct_visible_idea_count: 6,
        primary_control_count: 4,
        endpoint_reported_primary_control_count: 4,
        endpoint_readable_primary_control_count: 4,
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

/// Quality evidence used to gate novice catalog exposure for Lidded Box.
#[must_use]
pub const fn lidded_box_quality_evidence() -> StarterTemplateQualityEvidence {
    StarterTemplateQualityEvidence {
        profile_slug: LIDDED_BOX_SLUG,
        visible_idea_count: 6,
        distinct_visible_idea_count: 6,
        primary_control_count: 3,
        endpoint_reported_primary_control_count: 3,
        endpoint_readable_primary_control_count: 3,
        returned_too_subtle_candidate_count: 0,
        broken_or_floating_part_count: 0,
        export_conformance_clean: true,
        advanced_recipe_required: false,
        raw_technical_summary_count: 0,
    }
}

/// Curation metadata for the Lidded Box Make baseline.
#[must_use]
pub fn lidded_box_curation_metadata() -> CatalogCurationMetadata {
    CatalogCurationMetadata {
        profile_slug: LIDDED_BOX_SLUG,
        state: starter_template_curation_state_from_quality(lidded_box_quality_evidence()),
        has_visual_direction_evidence: true,
        has_readable_control_evidence: true,
        has_human_showcase_review: false,
        note: "Lidded Box is Box Primitive plus one visible Lid Seam feature, with pure-clay evidence and no crate claim.",
    }
}

/// Quality evidence for the internal Trimmed Box feature-module gate.
#[must_use]
pub const fn trimmed_box_quality_evidence() -> StarterTemplateQualityEvidence {
    StarterTemplateQualityEvidence {
        profile_slug: TRIMMED_BOX_SLUG,
        visible_idea_count: 6,
        distinct_visible_idea_count: 6,
        primary_control_count: 4,
        endpoint_reported_primary_control_count: 4,
        endpoint_readable_primary_control_count: 4,
        returned_too_subtle_candidate_count: 0,
        broken_or_floating_part_count: 0,
        export_conformance_clean: true,
        advanced_recipe_required: false,
        raw_technical_summary_count: 0,
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
            length_slot("width", "Width", "body", 0.4, 4.0, 0.05, 2.0),
            length_slot("depth", "Depth", "body", 0.3, 3.2, 0.05, 1.4),
            length_slot("height", "Height", "body", 0.3, 3.0, 0.05, 1.0),
            crate::ratio_slot(
                "edge_softness",
                "Edge Softness",
                "body",
                0.0,
                0.35,
                0.01,
                0.08,
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
            continuous_control("width", "Width", "width", 2.0, 0.4, 4.0),
            continuous_control("depth", "Depth", "depth", 1.4, 0.3, 3.2),
            continuous_control("height", "Height", "height", 1.0, 0.3, 3.0),
            continuous_control(
                "edge_softness",
                "Edge Softness",
                "edge_softness",
                0.08,
                0.0,
                0.35,
            ),
        ],
    );
    profile.candidate_strategies = Vec::new();

    build_fixture_catalog(FixtureCatalogSpec {
        slug: BOX_PRIMITIVE_SLUG,
        document_id: "box-primitive-doc",
        family,
        style,
        family_implementation: family_impl,
        style_implementation: style_impl,
        customizer_profile: profile,
        control_state: BTreeMap::from([
            ("width".to_owned(), ControlValue::Scalar(2.0)),
            ("depth".to_owned(), ControlValue::Scalar(1.4)),
            ("height".to_owned(), ControlValue::Scalar(1.0)),
            ("edge_softness".to_owned(), ControlValue::Scalar(0.08)),
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

/// Build the internal Trimmed Box fixture catalog.
///
/// This fixture proves exactly one feature after Lidded Box: a visible trim
/// band. It is not surfaced in the Make loop until a later integration gate.
#[must_use]
pub fn trimmed_box_fixture_catalog() -> FoundryFixtureCatalog {
    let family = family_schema(FamilySchemaSpec {
        id: TRIMMED_BOX_FAMILY_ID,
        display_name: "Trimmed Box",
        summary: "A simple lidded box with a visible trim band.",
        roles: vec![
            role("body", RoleMultiplicity::Single, true),
            role("lid_seam", RoleMultiplicity::Single, true),
            role("trim_band", RoleMultiplicity::Single, true),
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
                TRIMMED_PROPORTIONS
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
            crate::ratio_slot(
                "trim_thickness",
                "Trim Thickness",
                "trim_band",
                0.0,
                1.0,
                0.05,
                0.35,
            ),
        ],
        compatible_style_kits: vec![TRIMMED_BOX_STYLE_ID.to_owned()],
        tags: vec![
            "trimmed-box".to_owned(),
            "lid-seam".to_owned(),
            "trim-band".to_owned(),
            "clay".to_owned(),
        ],
    });

    let style = style_kit(
        TRIMMED_BOX_STYLE_ID,
        "Trimmed Box Clay",
        TRIMMED_BOX_FAMILY_ID,
        &trimmed_style_prototypes(),
        vec![
            "trimmed-box".to_owned(),
            "lid-seam".to_owned(),
            "trim-band".to_owned(),
            "clay".to_owned(),
        ],
    );

    let family_impl = family_implementation(
        TRIMMED_BOX_FAMILY_ID,
        "Trimmed Box family",
        trimmed_parameter_bindings(),
    );

    let style_impl = style_implementation(
        TRIMMED_BOX_STYLE_ID,
        TRIMMED_BOX_FAMILY_ID,
        trimmed_default_provider_map(),
        trimmed_recipe_fragments(),
    );

    let mut profile = crate::customizer_profile(
        TRIMMED_BOX_FAMILY_ID,
        TRIMMED_BOX_STYLE_ID,
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
            continuous_control(
                "trim_thickness",
                "Trim Thickness",
                "trim_thickness",
                0.35,
                0.0,
                1.0,
            ),
        ],
    );
    profile.candidate_strategies = vec![
        strategy(
            "clean-trimmed-box",
            "Clean Trimmed Box",
            &["trim_thickness"],
        ),
        strategy(
            "reinforced-trimmed-box",
            "Reinforced Trimmed Box",
            &["trim_thickness"],
        ),
        strategy(
            "compact-trimmed-box",
            "Compact Trimmed Box",
            &["proportions"],
        ),
        strategy("wide-trimmed-box", "Wide Trimmed Box", &["proportions"]),
        strategy("low-trim-box", "Low Trim Box", &["lid_height"]),
        strategy("soft-trimmed-box", "Soft Trimmed Box", &["edge_softness"]),
    ];

    build_fixture_catalog(FixtureCatalogSpec {
        slug: TRIMMED_BOX_SLUG,
        document_id: "trimmed-box-doc",
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
            ("trim_thickness".to_owned(), ControlValue::Scalar(0.35)),
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

fn trimmed_style_prototypes() -> Vec<(&'static str, &'static str, &'static str)> {
    TRIMMED_PROPORTIONS
        .into_iter()
        .flat_map(|proportion| {
            [
                (proportion.body_provider, "Closed lower box body", "body"),
                (
                    proportion.seam_provider,
                    "Raised clay line for visible lid seam",
                    "lid_seam",
                ),
                (
                    proportion.trim_provider,
                    "Raised clay geometry for visible trim band",
                    "trim_band",
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

fn trimmed_default_provider_map() -> BTreeMap<String, String> {
    BTreeMap::from([
        ("body".to_owned(), "compact_trimmed_body".to_owned()),
        ("lid_seam".to_owned(), "compact_trimmed_lid_seam".to_owned()),
        ("trim_band".to_owned(), "compact_trim_band".to_owned()),
    ])
}

fn parameter_bindings() -> Vec<ParameterBinding> {
    vec![
        half_extent_binding(
            "width",
            "body",
            crate::LOCAL_DEFINITION,
            "geometry.rounded_box.half_extents.x",
        ),
        half_extent_binding(
            "depth",
            "body",
            crate::LOCAL_DEFINITION,
            "geometry.rounded_box.half_extents.z",
        ),
        half_extent_binding(
            "height",
            "body",
            crate::LOCAL_DEFINITION,
            "geometry.rounded_box.half_extents.y",
        ),
        scaled_definition_binding(
            "edge_softness",
            "body",
            crate::LOCAL_DEFINITION,
            "geometry.rounded_box.radius",
            0.45,
            0.0,
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

fn trimmed_parameter_bindings() -> Vec<ParameterBinding> {
    vec![
        ParameterBinding::ChoiceToPrototype {
            slot: "proportions".to_owned(),
            role: "body".to_owned(),
            choices: TRIMMED_PROPORTIONS
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
            choices: TRIMMED_PROPORTIONS
                .into_iter()
                .map(|proportion| {
                    (
                        proportion.choice.to_owned(),
                        proportion.seam_provider.to_owned(),
                    )
                })
                .collect(),
        },
        ParameterBinding::ChoiceToPrototype {
            slot: "proportions".to_owned(),
            role: "trim_band".to_owned(),
            choices: TRIMMED_PROPORTIONS
                .into_iter()
                .map(|proportion| {
                    (
                        proportion.choice.to_owned(),
                        proportion.trim_provider.to_owned(),
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
            "edge_softness",
            "trim_band",
            crate::LOCAL_DEFINITION,
            "geometry.rounded_box.radius",
            0.006,
            0.032,
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
        definition_binding(
            "trim_thickness",
            "trim_band",
            crate::LOCAL_DEFINITION,
            "geometry.rounded_box.half_extents.y",
            0.05,
            0.14,
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

fn half_extent_binding(
    slot: &str,
    role_name: &str,
    definition: PartDefinitionId,
    local_key: &str,
) -> ParameterBinding {
    scaled_definition_binding(slot, role_name, definition, local_key, 0.5, 0.0)
}

fn scaled_definition_binding(
    slot: &str,
    role_name: &str,
    definition: PartDefinitionId,
    local_key: &str,
    scale: f32,
    offset: f32,
) -> ParameterBinding {
    ParameterBinding::Scalar {
        slot: slot.to_owned(),
        role: role_name.to_owned(),
        local_path: definition_scalar_path(definition, local_key),
        transform: ScalarTransform::ScaleOffset { scale, offset },
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

fn trimmed_recipe_fragments() -> Vec<RecipeFragment> {
    TRIMMED_PROPORTIONS
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
                rounded_box_fragment(
                    proportion.trim_provider,
                    "trim_band",
                    proportion.trim_half_extents,
                    0.024,
                    proportion.trim_translation,
                    Vec::new(),
                ),
            ]
        })
        .collect()
}
