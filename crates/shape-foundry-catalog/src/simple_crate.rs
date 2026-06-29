//! Simple Crate primitive-family fixture.

use std::collections::{BTreeMap, BTreeSet};

use shape_asset::{
    AssetId, AssetRecipe, Frame3, GeometryRecipe, GeometrySource, PartDefinition, PartDefinitionId,
    PartInstance, PartInstanceId, Transform3, definition_scalar_path, instance_scalar_path,
    validate_asset_recipe,
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

/// Simple Crate profile slug.
pub const SIMPLE_CRATE_SLUG: &str = "simple-crate";
/// Simple Crate family ID.
pub const SIMPLE_CRATE_FAMILY_ID: &str = "simple_crate";
/// Neutral primitive style ID for Simple Crate.
pub const SIMPLE_CRATE_STYLE_ID: &str = "simple_crate_primitive";

const BODY_HALF_HEIGHT: f32 = 0.45;
const LID_MIN_HALF_HEIGHT: f32 = 0.06;
const LID_MAX_HALF_HEIGHT: f32 = 0.46;

#[derive(Debug, Copy, Clone)]
struct CrateProportion {
    choice: &'static str,
    body_provider: &'static str,
    lid_provider: &'static str,
    seam_provider: &'static str,
    trim_provider: &'static str,
    body_half_extents: [f32; 3],
}

impl CrateProportion {
    const fn lid_half_extents(self) -> [f32; 3] {
        [
            self.body_half_extents[0] + 0.05,
            0.11,
            self.body_half_extents[2] + 0.05,
        ]
    }
}

const PROPORTIONS: [CrateProportion; 4] = [
    CrateProportion {
        choice: "compact_box",
        body_provider: "compact_body",
        lid_provider: "compact_lid",
        seam_provider: "compact_lid_seam",
        trim_provider: "compact_trim_band",
        body_half_extents: [0.82, BODY_HALF_HEIGHT, 0.58],
    },
    CrateProportion {
        choice: "wide_storage_crate",
        body_provider: "wide_body",
        lid_provider: "wide_lid",
        seam_provider: "wide_lid_seam",
        trim_provider: "wide_trim_band",
        body_half_extents: [1.32, BODY_HALF_HEIGHT, 0.62],
    },
    CrateProportion {
        choice: "tall_supply_crate",
        body_provider: "tall_body",
        lid_provider: "tall_lid",
        seam_provider: "tall_lid_seam",
        trim_provider: "tall_trim_band",
        body_half_extents: [0.72, BODY_HALF_HEIGHT, 0.48],
    },
    CrateProportion {
        choice: "low_flat_crate",
        body_provider: "low_flat_body",
        lid_provider: "low_flat_lid",
        seam_provider: "low_flat_lid_seam",
        trim_provider: "low_flat_trim_band",
        body_half_extents: [1.34, BODY_HALF_HEIGHT, 0.82],
    },
];

/// Quality evidence used to gate novice catalog exposure for Simple Crate.
#[must_use]
pub const fn quality_evidence() -> StarterTemplateQualityEvidence {
    StarterTemplateQualityEvidence {
        profile_slug: SIMPLE_CRATE_SLUG,
        visible_idea_count: 6,
        distinct_visible_idea_count: 6,
        primary_control_count: 5,
        endpoint_reported_primary_control_count: 5,
        endpoint_readable_primary_control_count: 5,
        returned_too_subtle_candidate_count: 0,
        broken_or_floating_part_count: 0,
        export_conformance_clean: true,
        advanced_recipe_required: false,
        raw_technical_summary_count: 0,
    }
}

/// Curation metadata for Simple Crate.
#[must_use]
pub fn curation_metadata() -> CatalogCurationMetadata {
    CatalogCurationMetadata {
        profile_slug: SIMPLE_CRATE_SLUG,
        state: starter_template_curation_state_from_quality(quality_evidence()),
        has_visual_direction_evidence: true,
        has_readable_control_evidence: true,
        has_human_showcase_review: false,
        note: "Simple Crate primitive v0 has clay dogfood evidence for visible directions and five readable controls.",
    }
}

/// Build the Simple Crate primitive fixture catalog.
#[must_use]
pub fn fixture_catalog() -> FoundryFixtureCatalog {
    let family = family_schema(FamilySchemaSpec {
        id: SIMPLE_CRATE_FAMILY_ID,
        display_name: "Simple Crate",
        summary: "Primitive clay crate family with a rectangular body, clear lid seam, soft edges, one trim band, and feet or skids.",
        roles: vec![
            role("body", RoleMultiplicity::Single, true),
            role("lid", RoleMultiplicity::Single, true),
            role("lid_seam", RoleMultiplicity::Repeated, true),
            role("trim_band", RoleMultiplicity::Repeated, true),
            role("feet_or_skids", RoleMultiplicity::Repeated, true),
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
                PROPORTIONS
                    .iter()
                    .map(|proportion| proportion.choice.to_owned())
                    .collect(),
            ),
            crate::ratio_slot("lid_height", "Lid Height", "lid", 0.0, 1.0, 0.05, 0.30),
            crate::ratio_slot(
                "edge_softness",
                "Edge Softness",
                "body",
                0.0,
                1.0,
                0.05,
                0.38,
            ),
            crate::ratio_slot(
                "trim_thickness",
                "Trim Thickness",
                "trim_band",
                0.0,
                1.0,
                0.05,
                0.36,
            ),
            choice_slot(
                "feet_style",
                "Feet Style",
                "feet_or_skids",
                vec![
                    "low_skids".to_owned(),
                    "block_feet".to_owned(),
                    "full_runners".to_owned(),
                ],
            ),
        ],
        compatible_style_kits: vec![SIMPLE_CRATE_STYLE_ID.to_owned()],
        tags: vec![
            "simple-crate".to_owned(),
            "primitive-family".to_owned(),
            "clay".to_owned(),
        ],
    });

    let style = style_kit(
        SIMPLE_CRATE_STYLE_ID,
        "Simple Crate Primitive",
        SIMPLE_CRATE_FAMILY_ID,
        &style_prototypes(),
        vec!["simple-crate".to_owned(), "clay".to_owned()],
    );

    let family_impl = family_implementation(
        SIMPLE_CRATE_FAMILY_ID,
        "Simple Crate primitive family",
        parameter_bindings(),
    );

    let style_impl = style_implementation(
        SIMPLE_CRATE_STYLE_ID,
        SIMPLE_CRATE_FAMILY_ID,
        default_provider_map(),
        recipe_fragments(),
    );

    let mut profile = crate::customizer_profile(
        SIMPLE_CRATE_FAMILY_ID,
        SIMPLE_CRATE_STYLE_ID,
        vec![
            choice_control(
                "proportions",
                "Proportions",
                "proportions",
                &[
                    "compact_box",
                    "wide_storage_crate",
                    "tall_supply_crate",
                    "low_flat_crate",
                ],
            ),
            continuous_control("lid_height", "Lid Height", "lid_height", 0.30, 0.0, 1.0),
            continuous_control(
                "edge_softness",
                "Edge Softness",
                "edge_softness",
                0.38,
                0.0,
                1.0,
            ),
            continuous_control(
                "trim_thickness",
                "Trim Thickness",
                "trim_thickness",
                0.36,
                0.0,
                1.0,
            ),
            choice_control(
                "feet_style",
                "Feet Style",
                "feet_style",
                &["low_skids", "block_feet", "full_runners"],
            ),
        ],
    );
    profile.candidate_strategies = vec![
        strategy("compact-box", "Compact Box", &["proportions", "lid_height"]),
        strategy(
            "wide-storage-crate",
            "Wide Storage Crate",
            &["proportions", "trim_thickness"],
        ),
        strategy(
            "tall-supply-crate",
            "Tall Supply Crate",
            &["proportions", "lid_height"],
        ),
        strategy(
            "low-flat-crate",
            "Low Flat Crate",
            &["proportions", "lid_height", "feet_style"],
        ),
        strategy(
            "reinforced-simple-crate",
            "Reinforced Simple Crate",
            &["trim_thickness", "feet_style", "edge_softness"],
        ),
        strategy(
            "clean-minimal-crate",
            "Clean Minimal Crate",
            &["trim_thickness", "edge_softness", "feet_style"],
        ),
    ];

    build_fixture_catalog(FixtureCatalogSpec {
        slug: SIMPLE_CRATE_SLUG,
        document_id: "simple-crate-doc",
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
            ("lid_height".to_owned(), ControlValue::Scalar(0.30)),
            ("edge_softness".to_owned(), ControlValue::Scalar(0.38)),
            ("trim_thickness".to_owned(), ControlValue::Scalar(0.36)),
            (
                "feet_style".to_owned(),
                ControlValue::Choice("low_skids".to_owned()),
            ),
        ]),
    })
}

fn style_prototypes() -> Vec<(&'static str, &'static str, &'static str)> {
    let mut prototypes = Vec::new();
    for proportion in PROPORTIONS {
        prototypes.push((proportion.body_provider, "Rectangular crate body", "body"));
        prototypes.push((proportion.lid_provider, "Raised lid", "lid"));
        prototypes.push((proportion.seam_provider, "Lid seam rails", "lid_seam"));
        prototypes.push((proportion.trim_provider, "Simple trim band", "trim_band"));
    }
    prototypes.extend([
        ("low_skids", "Low skids", "feet_or_skids"),
        ("block_feet", "Block feet", "feet_or_skids"),
        ("full_runners", "Full runners", "feet_or_skids"),
    ]);
    prototypes
}

fn default_provider_map() -> BTreeMap<String, String> {
    BTreeMap::from([
        ("body".to_owned(), "compact_body".to_owned()),
        ("lid".to_owned(), "compact_lid".to_owned()),
        ("lid_seam".to_owned(), "compact_lid_seam".to_owned()),
        ("trim_band".to_owned(), "compact_trim_band".to_owned()),
        ("feet_or_skids".to_owned(), "low_skids".to_owned()),
    ])
}

fn parameter_bindings() -> Vec<ParameterBinding> {
    let mut bindings = Vec::new();
    for (role_name, provider) in [
        ("body", "body"),
        ("lid", "lid"),
        ("lid_seam", "seam"),
        ("trim_band", "trim"),
    ] {
        bindings.push(choice_binding("proportions", role_name, provider));
    }
    bindings.push(choice_binding("feet_style", "feet_or_skids", "feet"));

    bindings.extend([
        definition_binding(
            "lid_height",
            "lid",
            crate::LOCAL_DEFINITION,
            "geometry.rounded_box.half_extents.y",
            LID_MIN_HALF_HEIGHT,
            LID_MAX_HALF_HEIGHT,
        ),
        instance_binding(
            "lid_height",
            "lid",
            crate::LOCAL_INSTANCE,
            "transform.translation.y",
            BODY_HALF_HEIGHT + LID_MIN_HALF_HEIGHT,
            BODY_HALF_HEIGHT + LID_MAX_HALF_HEIGHT,
        ),
        definition_binding(
            "edge_softness",
            "body",
            crate::LOCAL_DEFINITION,
            "geometry.rounded_box.radius",
            0.012,
            0.14,
        ),
        definition_binding(
            "edge_softness",
            "lid",
            crate::LOCAL_DEFINITION,
            "geometry.rounded_box.radius",
            0.01,
            0.055,
        ),
    ]);

    for definition in [PartDefinitionId(90), PartDefinitionId(92)] {
        bindings.push(definition_binding(
            "trim_thickness",
            "trim_band",
            definition,
            "geometry.rounded_box.half_extents.y",
            0.026,
            0.085,
        ));
        bindings.push(definition_binding(
            "trim_thickness",
            "trim_band",
            definition,
            "geometry.rounded_box.half_extents.z",
            0.018,
            0.06,
        ));
    }
    for definition in [PartDefinitionId(94), PartDefinitionId(96)] {
        bindings.push(definition_binding(
            "trim_thickness",
            "trim_band",
            definition,
            "geometry.rounded_box.half_extents.x",
            0.018,
            0.06,
        ));
        bindings.push(definition_binding(
            "trim_thickness",
            "trim_band",
            definition,
            "geometry.rounded_box.half_extents.y",
            0.026,
            0.085,
        ));
    }
    bindings
}

fn choice_binding(slot: &str, role_name: &str, provider_kind: &str) -> ParameterBinding {
    let choices = match provider_kind {
        "body" => PROPORTIONS
            .iter()
            .map(|proportion| {
                (
                    proportion.choice.to_owned(),
                    proportion.body_provider.to_owned(),
                )
            })
            .collect(),
        "lid" => PROPORTIONS
            .iter()
            .map(|proportion| {
                (
                    proportion.choice.to_owned(),
                    proportion.lid_provider.to_owned(),
                )
            })
            .collect(),
        "seam" => PROPORTIONS
            .iter()
            .map(|proportion| {
                (
                    proportion.choice.to_owned(),
                    proportion.seam_provider.to_owned(),
                )
            })
            .collect(),
        "trim" => PROPORTIONS
            .iter()
            .map(|proportion| {
                (
                    proportion.choice.to_owned(),
                    proportion.trim_provider.to_owned(),
                )
            })
            .collect(),
        "feet" => BTreeMap::from([
            ("low_skids".to_owned(), "low_skids".to_owned()),
            ("block_feet".to_owned(), "block_feet".to_owned()),
            ("full_runners".to_owned(), "full_runners".to_owned()),
        ]),
        _ => unreachable!("unknown Simple Crate provider kind"),
    };
    ParameterBinding::ChoiceToPrototype {
        slot: slot.to_owned(),
        role: role_name.to_owned(),
        choices,
    }
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

fn instance_binding(
    slot: &str,
    role_name: &str,
    instance: PartInstanceId,
    local_key: &str,
    minimum: f32,
    maximum: f32,
) -> ParameterBinding {
    ParameterBinding::Scalar {
        slot: slot.to_owned(),
        role: role_name.to_owned(),
        local_path: instance_scalar_path(instance, local_key),
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
    let mut fragments = Vec::new();
    for proportion in PROPORTIONS {
        fragments.push(body_fragment(proportion));
        fragments.push(lid_fragment(proportion));
        fragments.push(seam_fragment(proportion));
        fragments.push(trim_fragment(proportion));
    }
    fragments.extend([
        low_skid_fragment(),
        block_feet_fragment(),
        full_runner_fragment(),
    ]);
    fragments
}

fn body_fragment(proportion: CrateProportion) -> RecipeFragment {
    rounded_box_fragment(
        proportion.body_provider,
        "body",
        proportion.body_half_extents,
        0.06,
        [0.0, 0.0, 0.0],
        Vec::new(),
    )
}

fn lid_fragment(proportion: CrateProportion) -> RecipeFragment {
    let mut fragment = rounded_box_fragment(
        proportion.lid_provider,
        "lid",
        proportion.lid_half_extents(),
        0.032,
        [0.0, BODY_HALF_HEIGHT + 0.11, 0.0],
        Vec::new(),
    );
    add_instance_scalar_parameter(
        &mut fragment,
        crate::LOCAL_INSTANCE,
        "transform.translation.y",
        BODY_HALF_HEIGHT + LID_MIN_HALF_HEIGHT,
        BODY_HALF_HEIGHT + LID_MAX_HALF_HEIGHT,
        0.01,
    );
    fragment
}

fn seam_fragment(proportion: CrateProportion) -> RecipeFragment {
    let body = proportion.body_half_extents;
    let x = body[0] + 0.045;
    let z = body[2] + 0.03;
    let seam_y = BODY_HALF_HEIGHT - 0.027;
    box_assembly_fragment(
        proportion.seam_provider,
        "lid_seam",
        &[
            BoxPart::new(
                90,
                91,
                "front lid seam",
                [x, 0.018, 0.022],
                [0.0, seam_y, z],
            ),
            BoxPart::new(
                92,
                93,
                "rear lid seam",
                [x, 0.018, 0.022],
                [0.0, seam_y, -z],
            ),
            BoxPart::new(
                94,
                95,
                "left lid seam",
                [0.022, 0.018, body[2] - 0.02],
                [-body[0] - 0.03, seam_y, 0.0],
            ),
            BoxPart::new(
                96,
                97,
                "right lid seam",
                [0.022, 0.018, body[2] - 0.02],
                [body[0] + 0.03, seam_y, 0.0],
            ),
        ],
        0.01,
    )
}

fn trim_fragment(proportion: CrateProportion) -> RecipeFragment {
    let body = proportion.body_half_extents;
    let x = body[0] - 0.08;
    let z = body[2] - 0.08;
    box_assembly_fragment(
        proportion.trim_provider,
        "trim_band",
        &[
            BoxPart::new(
                90,
                91,
                "front trim band",
                [x, 0.048, 0.035],
                [0.0, -0.02, body[2] + 0.043],
            ),
            BoxPart::new(
                92,
                93,
                "rear trim band",
                [x, 0.048, 0.035],
                [0.0, -0.02, -body[2] - 0.043],
            ),
            BoxPart::new(
                94,
                95,
                "left trim band",
                [0.035, 0.048, z],
                [-body[0] - 0.043, -0.02, 0.0],
            ),
            BoxPart::new(
                96,
                97,
                "right trim band",
                [0.035, 0.048, z],
                [body[0] + 0.043, -0.02, 0.0],
            ),
        ],
        0.014,
    )
}

fn low_skid_fragment() -> RecipeFragment {
    box_assembly_fragment(
        "low_skids",
        "feet_or_skids",
        &[
            BoxPart::new(
                90,
                91,
                "left low skid",
                [0.16, 0.055, 0.52],
                [-0.42, -BODY_HALF_HEIGHT - 0.055, 0.0],
            ),
            BoxPart::new(
                92,
                93,
                "right low skid",
                [0.16, 0.055, 0.52],
                [0.42, -BODY_HALF_HEIGHT - 0.055, 0.0],
            ),
        ],
        0.018,
    )
}

fn block_feet_fragment() -> RecipeFragment {
    box_assembly_fragment(
        "block_feet",
        "feet_or_skids",
        &[
            BoxPart::new(
                90,
                91,
                "front left block foot",
                [0.18, 0.09, 0.16],
                [-0.5, -BODY_HALF_HEIGHT - 0.09, 0.42],
            ),
            BoxPart::new(
                92,
                93,
                "front right block foot",
                [0.18, 0.09, 0.16],
                [0.5, -BODY_HALF_HEIGHT - 0.09, 0.42],
            ),
            BoxPart::new(
                94,
                95,
                "rear left block foot",
                [0.18, 0.09, 0.16],
                [-0.5, -BODY_HALF_HEIGHT - 0.09, -0.42],
            ),
            BoxPart::new(
                96,
                97,
                "rear right block foot",
                [0.18, 0.09, 0.16],
                [0.5, -BODY_HALF_HEIGHT - 0.09, -0.42],
            ),
        ],
        0.018,
    )
}

fn full_runner_fragment() -> RecipeFragment {
    box_assembly_fragment(
        "full_runners",
        "feet_or_skids",
        &[
            BoxPart::new(
                90,
                91,
                "front full runner",
                [0.82, 0.07, 0.09],
                [0.0, -BODY_HALF_HEIGHT - 0.07, 0.48],
            ),
            BoxPart::new(
                92,
                93,
                "rear full runner",
                [0.82, 0.07, 0.09],
                [0.0, -BODY_HALF_HEIGHT - 0.07, -0.48],
            ),
            BoxPart::new(
                94,
                95,
                "center underside support",
                [0.16, 0.055, 0.48],
                [0.0, -BODY_HALF_HEIGHT - 0.055, 0.0],
            ),
        ],
        0.018,
    )
}

#[derive(Debug, Copy, Clone)]
struct BoxPart {
    definition: PartDefinitionId,
    instance: PartInstanceId,
    name: &'static str,
    half_extents: [f32; 3],
    translation: [f32; 3],
}

impl BoxPart {
    const fn new(
        definition: u64,
        instance: u64,
        name: &'static str,
        half_extents: [f32; 3],
        translation: [f32; 3],
    ) -> Self {
        Self {
            definition: PartDefinitionId(definition),
            instance: PartInstanceId(instance),
            name,
            half_extents,
            translation,
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
                enabled: true,
                tags: BTreeSet::from([role_name.to_owned(), format!("role:{role_name}")]),
                generated_by: None,
            },
        );
        recipe.parameters.insert(
            shape_asset::ParameterId(recipe.next_ids.parameter),
            shape_family_compile::scalar_parameter(
                recipe.next_ids.parameter,
                definition_scalar_path(part.definition, "geometry.rounded_box.half_extents.x"),
                format!("{} width", part.name),
                0.01,
                5.0,
                0.01,
                false,
            ),
        );
        recipe.next_ids.parameter += 1;
        recipe.parameters.insert(
            shape_asset::ParameterId(recipe.next_ids.parameter),
            shape_family_compile::scalar_parameter(
                recipe.next_ids.parameter,
                definition_scalar_path(part.definition, "geometry.rounded_box.half_extents.y"),
                format!("{} height", part.name),
                0.01,
                5.0,
                0.01,
                false,
            ),
        );
        recipe.next_ids.parameter += 1;
        recipe.parameters.insert(
            shape_asset::ParameterId(recipe.next_ids.parameter),
            shape_family_compile::scalar_parameter(
                recipe.next_ids.parameter,
                definition_scalar_path(part.definition, "geometry.rounded_box.half_extents.z"),
                format!("{} depth", part.name),
                0.01,
                5.0,
                0.01,
                false,
            ),
        );
        recipe.next_ids.parameter += 1;
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

fn add_instance_scalar_parameter(
    fragment: &mut RecipeFragment,
    instance: PartInstanceId,
    local_key: &str,
    minimum: f32,
    maximum: f32,
    step: f32,
) {
    let id = fragment.recipe.next_ids.parameter;
    fragment.recipe.parameters.insert(
        shape_asset::ParameterId(id),
        shape_family_compile::scalar_parameter(
            id,
            instance_scalar_path(instance, local_key),
            format!("{} {local_key}", fragment.id),
            minimum,
            maximum,
            step,
            false,
        ),
    );
    fragment.recipe.next_ids.parameter += 1;
    assert!(
        validate_asset_recipe(&fragment.recipe).is_valid(),
        "{} instance parameter should validate",
        fragment.id
    );
}
