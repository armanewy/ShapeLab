//! Flat Panel Primitive fixture.

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

/// Flat Panel Primitive profile slug.
pub const FLAT_PANEL_PRIMITIVE_SLUG: &str = "flat-panel-primitive";
/// Flat Panel Primitive family ID.
pub const FLAT_PANEL_PRIMITIVE_FAMILY_ID: &str = "flat_panel_primitive";
/// Neutral clay style ID for Flat Panel Primitive.
pub const FLAT_PANEL_PRIMITIVE_STYLE_ID: &str = "flat_panel_primitive_clay";
/// Hinged Panel internal preview profile slug.
pub const HINGED_PANEL_SLUG: &str = "hinged-panel";
/// Hinged Panel internal preview family ID.
pub const HINGED_PANEL_FAMILY_ID: &str = "hinged_panel";
/// Neutral clay style ID for Hinged Panel.
pub const HINGED_PANEL_STYLE_ID: &str = "hinged_panel_clay";
/// Internal Hinge Edge module ID.
pub const HINGE_EDGE_MODULE_ID: &str = "hinge-edge";
/// Handled Panel internal preview profile slug.
pub const HANDLED_PANEL_SLUG: &str = "handled-panel";
/// Handled Panel internal preview family ID.
pub const HANDLED_PANEL_FAMILY_ID: &str = "handled_panel";
/// Neutral clay style ID for Handled Panel.
pub const HANDLED_PANEL_STYLE_ID: &str = "handled_panel_clay";
/// Internal Handle / Knob module ID.
pub const HANDLE_KNOB_MODULE_ID: &str = "handle-knob";

#[derive(Debug, Copy, Clone)]
struct FlatPanelProportion {
    provider: &'static str,
    half_extents: [f32; 3],
}

#[derive(Debug, Copy, Clone)]
struct HingedPanelProportion {
    choice: &'static str,
    body_provider: &'static str,
    hinge_provider: &'static str,
    handle_provider: &'static str,
    body_half_extents: [f32; 3],
    hinge_half_extents: [f32; 3],
    handle_half_extents: [f32; 3],
    body_translation: [f32; 3],
    hinge_translation: [f32; 3],
    handle_translation: [f32; 3],
}

const PANEL_PROPORTIONS: [FlatPanelProportion; 4] = [
    FlatPanelProportion {
        provider: "narrow_panel_body",
        half_extents: [0.42, 0.92, 0.055],
    },
    FlatPanelProportion {
        provider: "wide_panel_body",
        half_extents: [1.10, 0.70, 0.06],
    },
    FlatPanelProportion {
        provider: "tall_panel_body",
        half_extents: [0.58, 1.24, 0.055],
    },
    FlatPanelProportion {
        provider: "short_panel_body",
        half_extents: [0.98, 0.46, 0.06],
    },
];

const HINGED_PANEL_PROPORTIONS: [HingedPanelProportion; 4] = [
    HingedPanelProportion {
        choice: "narrow_panel",
        body_provider: "narrow_hinged_panel_body",
        hinge_provider: "narrow_hinge_edge",
        handle_provider: "narrow_handle_knob",
        body_half_extents: [0.42, 0.92, 0.055],
        hinge_half_extents: [0.055, 0.92, 0.095],
        handle_half_extents: [0.160, 0.16, 0.140],
        body_translation: [0.0, 0.92, 0.0],
        hinge_translation: [-0.475, 0.92, 0.0],
        handle_translation: [0.22, 0.92, 0.195],
    },
    HingedPanelProportion {
        choice: "wide_panel",
        body_provider: "wide_hinged_panel_body",
        hinge_provider: "wide_hinge_edge",
        handle_provider: "wide_handle_knob",
        body_half_extents: [1.10, 0.70, 0.06],
        hinge_half_extents: [0.065, 0.70, 0.105],
        handle_half_extents: [0.160, 0.17, 0.140],
        body_translation: [0.0, 0.70, 0.0],
        hinge_translation: [-1.165, 0.70, 0.0],
        handle_translation: [0.68, 0.70, 0.200],
    },
    HingedPanelProportion {
        choice: "tall_panel",
        body_provider: "tall_hinged_panel_body",
        hinge_provider: "tall_hinge_edge",
        handle_provider: "tall_handle_knob",
        body_half_extents: [0.58, 1.24, 0.055],
        hinge_half_extents: [0.055, 1.24, 0.095],
        handle_half_extents: [0.160, 0.18, 0.140],
        body_translation: [0.0, 1.24, 0.0],
        hinge_translation: [-0.635, 1.24, 0.0],
        handle_translation: [0.35, 1.24, 0.195],
    },
    HingedPanelProportion {
        choice: "short_panel",
        body_provider: "short_hinged_panel_body",
        hinge_provider: "short_hinge_edge",
        handle_provider: "short_handle_knob",
        body_half_extents: [0.98, 0.46, 0.06],
        hinge_half_extents: [0.065, 0.46, 0.105],
        handle_half_extents: [0.160, 0.14, 0.140],
        body_translation: [0.0, 0.46, 0.0],
        hinge_translation: [-1.045, 0.46, 0.0],
        handle_translation: [0.60, 0.46, 0.200],
    },
];

/// Quality evidence used to gate novice catalog exposure for Flat Panel Primitive.
#[must_use]
pub const fn quality_evidence() -> StarterTemplateQualityEvidence {
    StarterTemplateQualityEvidence {
        profile_slug: FLAT_PANEL_PRIMITIVE_SLUG,
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

/// Quality evidence used for the internal Hinged Panel visual gate.
#[must_use]
pub const fn hinged_panel_quality_evidence() -> StarterTemplateQualityEvidence {
    StarterTemplateQualityEvidence {
        profile_slug: HINGED_PANEL_SLUG,
        visible_idea_count: 5,
        distinct_visible_idea_count: 5,
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

/// Quality evidence used for the internal Handled Panel visual gate.
#[must_use]
pub const fn handled_panel_quality_evidence() -> StarterTemplateQualityEvidence {
    StarterTemplateQualityEvidence {
        profile_slug: HANDLED_PANEL_SLUG,
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

/// Curation metadata for Flat Panel Primitive.
#[must_use]
pub fn curation_metadata() -> CatalogCurationMetadata {
    CatalogCurationMetadata {
        profile_slug: FLAT_PANEL_PRIMITIVE_SLUG,
        state: starter_template_curation_state_from_quality(quality_evidence()),
        has_visual_direction_evidence: true,
        has_readable_control_evidence: true,
        has_human_showcase_review: false,
        note: "Flat Panel Primitive is the second kernel proof: one upright clay panel with no Door, hinge, material, rigging, or animation claim.",
    }
}

/// Curation metadata for Hinged Panel. Prompt 4 decides app visibility.
#[must_use]
pub fn hinged_panel_curation_metadata() -> CatalogCurationMetadata {
    CatalogCurationMetadata {
        profile_slug: HINGED_PANEL_SLUG,
        state: starter_template_curation_state_from_quality(hinged_panel_quality_evidence()),
        has_visual_direction_evidence: true,
        has_readable_control_evidence: true,
        has_human_showcase_review: false,
        note: "Hinged Panel is Flat Panel plus one visible Hinge Edge feature. It does not claim Door naming, open/close motion, materials, rigging, or animation.",
    }
}

/// Curation metadata for Handled Panel. Integration owns app visibility.
#[must_use]
pub fn handled_panel_curation_metadata() -> CatalogCurationMetadata {
    CatalogCurationMetadata {
        profile_slug: HANDLED_PANEL_SLUG,
        state: starter_template_curation_state_from_quality(handled_panel_quality_evidence()),
        has_visual_direction_evidence: true,
        has_readable_control_evidence: true,
        has_human_showcase_review: false,
        note: "Handled Panel is Hinged Panel plus one visible Handle / Knob feature. It does not claim Door naming, open/close motion, materials, rigging, or animation.",
    }
}

/// Build the Flat Panel Primitive fixture catalog.
#[must_use]
pub fn fixture_catalog() -> FoundryFixtureCatalog {
    let family = family_schema(FamilySchemaSpec {
        id: FLAT_PANEL_PRIMITIVE_FAMILY_ID,
        display_name: "Flat Panel Primitive",
        summary: "One upright clay panel with readable width, height, thickness, and edge softness.",
        roles: vec![role("panel_body", RoleMultiplicity::Single, true)],
        allowed_operations: vec![
            AllowedOperationKind::Primitive,
            AllowedOperationKind::Array,
            AllowedOperationKind::Transform,
            AllowedOperationKind::Bevel,
        ],
        parameter_slots: vec![
            length_slot("width", "Width", "panel_body", 0.4, 4.0, 0.05, 1.8),
            length_slot("height", "Height", "panel_body", 0.5, 4.0, 0.05, 2.6),
            length_slot(
                "thickness",
                "Thickness",
                "panel_body",
                0.05,
                1.2,
                0.01,
                0.18,
            ),
            crate::ratio_slot(
                "edge_softness",
                "Edge Softness",
                "panel_body",
                0.0,
                0.30,
                0.01,
                0.05,
            ),
        ],
        compatible_style_kits: vec![FLAT_PANEL_PRIMITIVE_STYLE_ID.to_owned()],
        tags: vec![
            "flat-panel-primitive".to_owned(),
            "panel".to_owned(),
            "clay".to_owned(),
        ],
    });

    let style = style_kit(
        FLAT_PANEL_PRIMITIVE_STYLE_ID,
        "Flat Panel Primitive Clay",
        FLAT_PANEL_PRIMITIVE_FAMILY_ID,
        &style_prototypes(),
        vec![
            "flat-panel-primitive".to_owned(),
            "panel".to_owned(),
            "clay".to_owned(),
        ],
    );

    let family_impl = family_implementation(
        FLAT_PANEL_PRIMITIVE_FAMILY_ID,
        "Flat Panel Primitive family",
        parameter_bindings(),
    );

    let style_impl = style_implementation(
        FLAT_PANEL_PRIMITIVE_STYLE_ID,
        FLAT_PANEL_PRIMITIVE_FAMILY_ID,
        default_provider_map(),
        recipe_fragments(),
    );

    let mut profile = crate::customizer_profile(
        FLAT_PANEL_PRIMITIVE_FAMILY_ID,
        FLAT_PANEL_PRIMITIVE_STYLE_ID,
        vec![
            continuous_control("width", "Width", "width", 1.8, 0.4, 4.0),
            continuous_control("height", "Height", "height", 2.6, 0.5, 4.0),
            continuous_control("thickness", "Thickness", "thickness", 0.18, 0.05, 1.2),
            continuous_control(
                "edge_softness",
                "Edge Softness",
                "edge_softness",
                0.05,
                0.0,
                0.30,
            ),
        ],
    );
    profile.candidate_strategies = Vec::new();

    build_fixture_catalog(FixtureCatalogSpec {
        slug: FLAT_PANEL_PRIMITIVE_SLUG,
        document_id: "flat-panel-primitive-doc",
        family,
        style,
        family_implementation: family_impl,
        style_implementation: style_impl,
        customizer_profile: profile,
        control_state: BTreeMap::from([
            ("width".to_owned(), ControlValue::Scalar(1.8)),
            ("height".to_owned(), ControlValue::Scalar(2.6)),
            ("thickness".to_owned(), ControlValue::Scalar(0.18)),
            ("edge_softness".to_owned(), ControlValue::Scalar(0.05)),
        ]),
    })
}

/// Build the internal Hinged Panel fixture catalog.
///
/// This fixture proves exactly one feature after Flat Panel Primitive: a
/// visible hinge-side clay edge. It is not surfaced in the Make loop until a
/// later gate approves it.
#[must_use]
pub fn hinged_panel_fixture_catalog() -> FoundryFixtureCatalog {
    let family = family_schema(FamilySchemaSpec {
        id: HINGED_PANEL_FAMILY_ID,
        display_name: "Hinged Panel",
        summary: "A simple upright clay panel with a visible hinge edge.",
        roles: vec![
            role("panel_body", RoleMultiplicity::Single, true),
            role("hinge_edge", RoleMultiplicity::Single, true),
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
                "panel_body",
                HINGED_PANEL_PROPORTIONS
                    .iter()
                    .map(|proportion| proportion.choice.to_owned())
                    .collect(),
            ),
            crate::ratio_slot(
                "edge_softness",
                "Edge Softness",
                "panel_body",
                0.0,
                1.0,
                0.05,
                0.35,
            ),
            crate::ratio_slot(
                "hinge_edge_style",
                "Hinge Edge",
                "hinge_edge",
                0.0,
                1.0,
                0.05,
                0.40,
            ),
        ],
        compatible_style_kits: vec![HINGED_PANEL_STYLE_ID.to_owned()],
        tags: vec![
            "hinged-panel".to_owned(),
            "hinge-edge".to_owned(),
            "clay".to_owned(),
        ],
    });

    let style = style_kit(
        HINGED_PANEL_STYLE_ID,
        "Hinged Panel Clay",
        HINGED_PANEL_FAMILY_ID,
        &hinged_panel_style_prototypes(),
        vec![
            "hinged-panel".to_owned(),
            "hinge-edge".to_owned(),
            "clay".to_owned(),
        ],
    );

    let family_impl = family_implementation(
        HINGED_PANEL_FAMILY_ID,
        "Hinged Panel family",
        hinged_panel_parameter_bindings(),
    );

    let style_impl = style_implementation(
        HINGED_PANEL_STYLE_ID,
        HINGED_PANEL_FAMILY_ID,
        hinged_panel_default_provider_map(),
        hinged_panel_recipe_fragments(),
    );

    let mut profile = crate::customizer_profile(
        HINGED_PANEL_FAMILY_ID,
        HINGED_PANEL_STYLE_ID,
        vec![
            choice_control(
                "proportions",
                "Proportions",
                "proportions",
                &["narrow_panel", "wide_panel", "tall_panel", "short_panel"],
            ),
            continuous_control(
                "edge_softness",
                "Edge Softness",
                "edge_softness",
                0.30,
                0.0,
                1.0,
            ),
            continuous_control(
                "hinge_edge_style",
                "Hinge Edge",
                "hinge_edge_style",
                0.40,
                0.0,
                1.0,
            ),
        ],
    );
    profile.candidate_strategies = vec![
        strategy(
            "clean-hinged-panel",
            "Clean Hinged Panel",
            &["hinge_edge_style"],
        ),
        strategy("wide-hinged-panel", "Wide Hinged Panel", &["proportions"]),
        strategy("tall-hinged-panel", "Tall Hinged Panel", &["proportions"]),
        strategy(
            "heavy-edge-panel",
            "Heavy Edge Panel",
            &["hinge_edge_style"],
        ),
        strategy(
            "minimal-hinged-panel",
            "Minimal Hinged Panel",
            &["edge_softness", "hinge_edge_style"],
        ),
    ];

    build_fixture_catalog(FixtureCatalogSpec {
        slug: HINGED_PANEL_SLUG,
        document_id: "hinged-panel-doc",
        family,
        style,
        family_implementation: family_impl,
        style_implementation: style_impl,
        customizer_profile: profile,
        control_state: BTreeMap::from([
            (
                "proportions".to_owned(),
                ControlValue::Choice("narrow_panel".to_owned()),
            ),
            ("edge_softness".to_owned(), ControlValue::Scalar(0.30)),
            ("hinge_edge_style".to_owned(), ControlValue::Scalar(0.40)),
        ]),
    })
}

/// Build the internal Handled Panel fixture catalog.
///
/// This fixture adds exactly one feature to Hinged Panel: visible Handle / Knob
/// clay geometry. It has no open/close behavior, rigging, animation, materials,
/// latch system, frame, or inset panel.
#[must_use]
pub fn handled_panel_fixture_catalog() -> FoundryFixtureCatalog {
    let family = family_schema(FamilySchemaSpec {
        id: HANDLED_PANEL_FAMILY_ID,
        display_name: "Handled Panel",
        summary: "A simple upright clay panel with a visible hinge edge and handle.",
        roles: vec![
            role("panel_body", RoleMultiplicity::Single, true),
            role("hinge_edge", RoleMultiplicity::Single, true),
            role("handle_knob", RoleMultiplicity::Single, true),
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
                "panel_body",
                HINGED_PANEL_PROPORTIONS
                    .iter()
                    .map(|proportion| proportion.choice.to_owned())
                    .collect(),
            ),
            crate::ratio_slot(
                "edge_softness",
                "Edge Softness",
                "panel_body",
                0.0,
                1.0,
                0.05,
                0.35,
            ),
            crate::ratio_slot(
                "hinge_edge_style",
                "Hinge Edge",
                "hinge_edge",
                0.0,
                1.0,
                0.05,
                0.40,
            ),
            crate::ratio_slot(
                "handle_knob_style",
                "Handle / Knob Style",
                "handle_knob",
                0.0,
                1.0,
                0.05,
                0.45,
            ),
        ],
        compatible_style_kits: vec![HANDLED_PANEL_STYLE_ID.to_owned()],
        tags: vec![
            "handled-panel".to_owned(),
            "hinge-edge".to_owned(),
            "handle-knob".to_owned(),
            "clay".to_owned(),
        ],
    });

    let style = style_kit(
        HANDLED_PANEL_STYLE_ID,
        "Handled Panel Clay",
        HANDLED_PANEL_FAMILY_ID,
        &handled_panel_style_prototypes(),
        vec![
            "handled-panel".to_owned(),
            "hinge-edge".to_owned(),
            "handle-knob".to_owned(),
            "clay".to_owned(),
        ],
    );

    let family_impl = family_implementation(
        HANDLED_PANEL_FAMILY_ID,
        "Handled Panel family",
        handled_panel_parameter_bindings(),
    );

    let style_impl = style_implementation(
        HANDLED_PANEL_STYLE_ID,
        HANDLED_PANEL_FAMILY_ID,
        handled_panel_default_provider_map(),
        handled_panel_recipe_fragments(),
    );

    let mut profile = crate::customizer_profile(
        HANDLED_PANEL_FAMILY_ID,
        HANDLED_PANEL_STYLE_ID,
        vec![
            choice_control(
                "proportions",
                "Proportions",
                "proportions",
                &["narrow_panel", "wide_panel", "tall_panel", "short_panel"],
            ),
            continuous_control(
                "edge_softness",
                "Edge Softness",
                "edge_softness",
                0.30,
                0.0,
                1.0,
            ),
            continuous_control(
                "hinge_edge_style",
                "Hinge Edge",
                "hinge_edge_style",
                0.40,
                0.0,
                1.0,
            ),
            continuous_control(
                "handle_knob_style",
                "Handle / Knob Style",
                "handle_knob_style",
                0.45,
                0.0,
                1.0,
            ),
        ],
    );
    profile.candidate_strategies = vec![
        strategy("knob-panel", "Knob Panel", &["handle_knob_style"]),
        strategy(
            "pull-handle-panel",
            "Pull Handle Panel",
            &["handle_knob_style"],
        ),
        strategy("wide-handled-panel", "Wide Handled Panel", &["proportions"]),
        strategy("tall-handled-panel", "Tall Handled Panel", &["proportions"]),
        strategy(
            "clean-handled-panel",
            "Clean Handled Panel",
            &["edge_softness", "handle_knob_style"],
        ),
        strategy(
            "heavy-edge-handled-panel",
            "Heavy Edge Handled Panel",
            &["hinge_edge_style"],
        ),
    ];

    build_fixture_catalog(FixtureCatalogSpec {
        slug: HANDLED_PANEL_SLUG,
        document_id: "handled-panel-doc",
        family,
        style,
        family_implementation: family_impl,
        style_implementation: style_impl,
        customizer_profile: profile,
        control_state: BTreeMap::from([
            (
                "proportions".to_owned(),
                ControlValue::Choice("narrow_panel".to_owned()),
            ),
            ("edge_softness".to_owned(), ControlValue::Scalar(0.30)),
            ("hinge_edge_style".to_owned(), ControlValue::Scalar(0.40)),
            ("handle_knob_style".to_owned(), ControlValue::Scalar(0.45)),
        ]),
    })
}

fn style_prototypes() -> Vec<(&'static str, &'static str, &'static str)> {
    PANEL_PROPORTIONS
        .into_iter()
        .map(|proportion| (proportion.provider, "Upright panel body", "panel_body"))
        .collect()
}

fn hinged_panel_style_prototypes() -> Vec<(&'static str, &'static str, &'static str)> {
    HINGED_PANEL_PROPORTIONS
        .into_iter()
        .flat_map(|proportion| {
            [
                (proportion.body_provider, "Upright panel body", "panel_body"),
                (
                    proportion.hinge_provider,
                    "Raised clay geometry for visible hinge-side edge",
                    "hinge_edge",
                ),
            ]
        })
        .collect()
}

fn handled_panel_style_prototypes() -> Vec<(&'static str, &'static str, &'static str)> {
    HINGED_PANEL_PROPORTIONS
        .into_iter()
        .flat_map(|proportion| {
            [
                (proportion.body_provider, "Upright panel body", "panel_body"),
                (
                    proportion.hinge_provider,
                    "Raised clay geometry for visible hinge-side edge",
                    "hinge_edge",
                ),
                (
                    proportion.handle_provider,
                    "Shallow clay geometry for visible handle or knob",
                    "handle_knob",
                ),
            ]
        })
        .collect()
}

fn default_provider_map() -> BTreeMap<String, String> {
    BTreeMap::from([("panel_body".to_owned(), "narrow_panel_body".to_owned())])
}

fn hinged_panel_default_provider_map() -> BTreeMap<String, String> {
    BTreeMap::from([
        (
            "panel_body".to_owned(),
            "narrow_hinged_panel_body".to_owned(),
        ),
        ("hinge_edge".to_owned(), "narrow_hinge_edge".to_owned()),
    ])
}

fn handled_panel_default_provider_map() -> BTreeMap<String, String> {
    BTreeMap::from([
        (
            "panel_body".to_owned(),
            "narrow_hinged_panel_body".to_owned(),
        ),
        ("hinge_edge".to_owned(), "narrow_hinge_edge".to_owned()),
        ("handle_knob".to_owned(), "narrow_handle_knob".to_owned()),
    ])
}

fn parameter_bindings() -> Vec<ParameterBinding> {
    vec![
        half_extent_binding(
            "width",
            "panel_body",
            crate::LOCAL_DEFINITION,
            "geometry.rounded_box.half_extents.x",
        ),
        half_extent_binding(
            "height",
            "panel_body",
            crate::LOCAL_DEFINITION,
            "geometry.rounded_box.half_extents.y",
        ),
        half_extent_binding(
            "thickness",
            "panel_body",
            crate::LOCAL_DEFINITION,
            "geometry.rounded_box.half_extents.z",
        ),
        scaled_definition_binding(
            "edge_softness",
            "panel_body",
            crate::LOCAL_DEFINITION,
            "geometry.rounded_box.radius",
            0.25,
            0.0,
        ),
    ]
}

fn hinged_panel_parameter_bindings() -> Vec<ParameterBinding> {
    vec![
        ParameterBinding::ChoiceToPrototype {
            slot: "proportions".to_owned(),
            role: "panel_body".to_owned(),
            choices: HINGED_PANEL_PROPORTIONS
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
            role: "hinge_edge".to_owned(),
            choices: HINGED_PANEL_PROPORTIONS
                .into_iter()
                .map(|proportion| {
                    (
                        proportion.choice.to_owned(),
                        proportion.hinge_provider.to_owned(),
                    )
                })
                .collect(),
        },
        definition_binding(
            "edge_softness",
            "panel_body",
            crate::LOCAL_DEFINITION,
            "geometry.rounded_box.radius",
            0.002,
            0.075,
        ),
        definition_binding(
            "hinge_edge_style",
            "hinge_edge",
            crate::LOCAL_DEFINITION,
            "geometry.rounded_box.half_extents.z",
            0.07,
            0.18,
        ),
    ]
}

fn handled_panel_parameter_bindings() -> Vec<ParameterBinding> {
    vec![
        ParameterBinding::ChoiceToPrototype {
            slot: "proportions".to_owned(),
            role: "panel_body".to_owned(),
            choices: HINGED_PANEL_PROPORTIONS
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
            role: "hinge_edge".to_owned(),
            choices: HINGED_PANEL_PROPORTIONS
                .into_iter()
                .map(|proportion| {
                    (
                        proportion.choice.to_owned(),
                        proportion.hinge_provider.to_owned(),
                    )
                })
                .collect(),
        },
        ParameterBinding::ChoiceToPrototype {
            slot: "proportions".to_owned(),
            role: "handle_knob".to_owned(),
            choices: HINGED_PANEL_PROPORTIONS
                .into_iter()
                .map(|proportion| {
                    (
                        proportion.choice.to_owned(),
                        proportion.handle_provider.to_owned(),
                    )
                })
                .collect(),
        },
        definition_binding(
            "edge_softness",
            "panel_body",
            crate::LOCAL_DEFINITION,
            "geometry.rounded_box.radius",
            0.002,
            0.075,
        ),
        definition_binding(
            "hinge_edge_style",
            "hinge_edge",
            crate::LOCAL_DEFINITION,
            "geometry.rounded_box.half_extents.z",
            0.07,
            0.18,
        ),
        definition_binding(
            "handle_knob_style",
            "handle_knob",
            crate::LOCAL_DEFINITION,
            "geometry.rounded_box.half_extents.y",
            0.055,
            0.460,
        ),
        definition_binding(
            "handle_knob_style",
            "handle_knob",
            crate::LOCAL_DEFINITION,
            "geometry.rounded_box.half_extents.x",
            0.060,
            0.200,
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
    PANEL_PROPORTIONS
        .into_iter()
        .map(|proportion| {
            rounded_box_fragment(
                proportion.provider,
                "panel_body",
                proportion.half_extents,
                0.018,
                [0.0, proportion.half_extents[1], 0.0],
                Vec::new(),
            )
        })
        .collect()
}

fn hinged_panel_recipe_fragments() -> Vec<RecipeFragment> {
    HINGED_PANEL_PROPORTIONS
        .into_iter()
        .flat_map(|proportion| {
            [
                rounded_box_fragment(
                    proportion.body_provider,
                    "panel_body",
                    proportion.body_half_extents,
                    0.018,
                    proportion.body_translation,
                    Vec::new(),
                ),
                rounded_box_fragment(
                    proportion.hinge_provider,
                    "hinge_edge",
                    proportion.hinge_half_extents,
                    0.012,
                    proportion.hinge_translation,
                    Vec::new(),
                ),
            ]
        })
        .collect()
}

fn handled_panel_recipe_fragments() -> Vec<RecipeFragment> {
    HINGED_PANEL_PROPORTIONS
        .into_iter()
        .flat_map(|proportion| {
            [
                rounded_box_fragment(
                    proportion.body_provider,
                    "panel_body",
                    proportion.body_half_extents,
                    0.018,
                    proportion.body_translation,
                    Vec::new(),
                ),
                rounded_box_fragment(
                    proportion.hinge_provider,
                    "hinge_edge",
                    proportion.hinge_half_extents,
                    0.012,
                    proportion.hinge_translation,
                    Vec::new(),
                ),
                rounded_box_fragment(
                    proportion.handle_provider,
                    "handle_knob",
                    proportion.handle_half_extents,
                    0.018,
                    proportion.handle_translation,
                    Vec::new(),
                ),
            ]
        })
        .collect()
}
