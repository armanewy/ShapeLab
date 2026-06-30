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

#[derive(Debug, Copy, Clone)]
struct BoxProportion {
    choice: &'static str,
    provider: &'static str,
    half_extents: [f32; 3],
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

fn style_prototypes() -> Vec<(&'static str, &'static str, &'static str)> {
    PROPORTIONS
        .into_iter()
        .map(|proportion| (proportion.provider, "Closed box body", "body"))
        .collect()
}

fn default_provider_map() -> BTreeMap<String, String> {
    BTreeMap::from([("body".to_owned(), "compact_body".to_owned())])
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
