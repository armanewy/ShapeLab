//! Flat Panel Primitive fixture.

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

/// Flat Panel Primitive profile slug.
pub const FLAT_PANEL_PRIMITIVE_SLUG: &str = "flat-panel-primitive";
/// Flat Panel Primitive family ID.
pub const FLAT_PANEL_PRIMITIVE_FAMILY_ID: &str = "flat_panel_primitive";
/// Neutral clay style ID for Flat Panel Primitive.
pub const FLAT_PANEL_PRIMITIVE_STYLE_ID: &str = "flat_panel_primitive_clay";

#[derive(Debug, Copy, Clone)]
struct FlatPanelProportion {
    choice: &'static str,
    provider: &'static str,
    half_extents: [f32; 3],
}

const PANEL_PROPORTIONS: [FlatPanelProportion; 4] = [
    FlatPanelProportion {
        choice: "narrow_panel",
        provider: "narrow_panel_body",
        half_extents: [0.42, 0.92, 0.055],
    },
    FlatPanelProportion {
        choice: "wide_panel",
        provider: "wide_panel_body",
        half_extents: [1.10, 0.70, 0.06],
    },
    FlatPanelProportion {
        choice: "tall_panel",
        provider: "tall_panel_body",
        half_extents: [0.58, 1.24, 0.055],
    },
    FlatPanelProportion {
        choice: "short_panel",
        provider: "short_panel_body",
        half_extents: [0.98, 0.46, 0.06],
    },
];

/// Quality evidence used to gate novice catalog exposure for Flat Panel Primitive.
#[must_use]
pub const fn quality_evidence() -> StarterTemplateQualityEvidence {
    StarterTemplateQualityEvidence {
        profile_slug: FLAT_PANEL_PRIMITIVE_SLUG,
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
            choice_slot(
                "proportions",
                "Proportions",
                "panel_body",
                PANEL_PROPORTIONS
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
        ],
    );
    profile.candidate_strategies = vec![
        strategy("narrow-panel", "Narrow Panel", &["proportions"]),
        strategy("wide-panel", "Wide Panel", &["proportions"]),
        strategy("tall-panel", "Tall Panel", &["proportions"]),
        strategy("short-panel", "Short Panel", &["proportions"]),
        strategy("soft-edged-panel", "Soft-Edged Panel", &["edge_softness"]),
        strategy("sharp-panel", "Sharp Panel", &["edge_softness"]),
    ];

    build_fixture_catalog(FixtureCatalogSpec {
        slug: FLAT_PANEL_PRIMITIVE_SLUG,
        document_id: "flat-panel-primitive-doc",
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
        ]),
    })
}

fn style_prototypes() -> Vec<(&'static str, &'static str, &'static str)> {
    PANEL_PROPORTIONS
        .into_iter()
        .map(|proportion| (proportion.provider, "Upright panel body", "panel_body"))
        .collect()
}

fn default_provider_map() -> BTreeMap<String, String> {
    BTreeMap::from([("panel_body".to_owned(), "narrow_panel_body".to_owned())])
}

fn parameter_bindings() -> Vec<ParameterBinding> {
    vec![
        ParameterBinding::ChoiceToPrototype {
            slot: "proportions".to_owned(),
            role: "panel_body".to_owned(),
            choices: PANEL_PROPORTIONS
                .into_iter()
                .map(|proportion| (proportion.choice.to_owned(), proportion.provider.to_owned()))
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
