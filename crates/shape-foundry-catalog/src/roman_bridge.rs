//! Roman bridge headless foundry fixture.

use std::collections::{BTreeMap, BTreeSet};

use shape_asset::{
    AssetId, AssetRecipe, AttachmentMode, Frame3, GeometryRecipe, GeometrySource,
    ModelingOperationSpec, OperationId, PartDefinition, PartDefinitionId, PartInstance,
    PartInstanceId, SocketId, SocketSpec, Transform3, definition_scalar_path,
    validate_asset_recipe,
};
use shape_caesar_assets::style_kits::roman_timber_engineering_style_kit;
use shape_family::{
    AllowedOperationKind, AttachmentRule, FamilyRuleExecutionPolicy, ParameterExecutionPolicy,
    PartPrototype, RoleMultiplicity, StyleKit,
};
use shape_family_compile::{
    FragmentAttachmentBinding, FragmentAttachmentPairing, FragmentSocketPort, ParameterBinding,
    RECIPE_FRAGMENT_SCHEMA_VERSION, RecipeFragment, RecipeFragmentExports, RigidOffset,
    ScalarTransform, scalar_parameter,
};
use shape_foundry::{
    CandidateStrategy, ChoiceOption, ClosedInterval, ControlKind, ControlSlotBinding,
    ControlTopologyBehavior, ControlValue, CustomizerControl, CustomizerProfile,
    DomainCertification, FeasibleControlDomain, FoundryPartGroupDescriptor, ProviderOption,
    ResponseCurve, WholeModelPreviewRef, built_in_part_group_descriptors_for_profile,
};

use crate::{
    FamilySchemaSpec, FixtureCatalogSpec, FoundryFixtureCatalog, build_fixture_catalog,
    choice_slot, family_implementation, family_schema, length_slot, ratio_slot, role,
    style_implementation,
};

const BRIDGE_FAMILY_ID: &str = "bridge";
const ROMAN_TIMBER_STYLE_ID: &str = "roman_timber_engineering";
const LOCAL_DEFINITION: PartDefinitionId = PartDefinitionId(90);
const LOCAL_SOCKET: SocketId = SocketId(7);
const FIRST_INSTANCE: u64 = 91;

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
enum BridgeQuality {
    Standard,
    Hq,
}

/// Build the Roman bridge fixture catalog.
#[must_use]
pub fn fixture_catalog() -> FoundryFixtureCatalog {
    fixture_catalog_for(BridgeQuality::Standard)
}

/// Build the HQ Roman bridge fixture catalog.
#[must_use]
pub fn hq_fixture_catalog() -> FoundryFixtureCatalog {
    fixture_catalog_for(BridgeQuality::Hq)
}

/// Product-safe semantic part groups for Roman Timber Bridge profiles.
#[must_use]
pub fn part_group_descriptors() -> Vec<FoundryPartGroupDescriptor> {
    built_in_part_group_descriptors_for_profile("roman-bridge")
}

fn fixture_catalog_for(quality: BridgeQuality) -> FoundryFixtureCatalog {
    let family = bridge_family_schema(quality);
    let style = roman_bridge_style_kit(quality);
    let family_impl = bridge_family_implementation(quality);
    let default_role_providers = match quality {
        BridgeQuality::Standard => BTreeMap::from([
            ("support".to_owned(), "tight_pile_bents".to_owned()),
            ("span".to_owned(), "hewn_span_beam".to_owned()),
            ("deck".to_owned(), "lashed_deck_plank".to_owned()),
            ("brace".to_owned(), "cross_brace_beam".to_owned()),
            ("ramp".to_owned(), "bank_ramp_planks".to_owned()),
            ("rail".to_owned(), "guard_rail".to_owned()),
        ]),
        BridgeQuality::Hq => BTreeMap::from([
            ("support".to_owned(), "stone_pier_blocks".to_owned()),
            ("span".to_owned(), "hewn_span_beam".to_owned()),
            ("deck".to_owned(), "segmented_deck_planks".to_owned()),
            ("brace".to_owned(), "minimal_under_ties".to_owned()),
            ("ramp".to_owned(), "bank_ramp_planks".to_owned()),
            ("rail".to_owned(), "guard_rail_courses".to_owned()),
            ("connector".to_owned(), "bolted_joinery_detail".to_owned()),
        ]),
    };
    let mut fragments = vec![
        support_fragment("pointed_round_pile", 0.13, 4, 1.1),
        support_fragment("tight_pile_bents", 0.145, 6, 0.66),
        support_fragment("marching_pile_bents", 0.16, 8, 0.48),
        span_fragment(),
        deck_fragment(
            "lashed_deck_plank",
            [3.6, 1.05],
            0.09,
            0.018,
            [0.0, 0.23, 0.0],
            1,
        ),
        deck_fragment(
            "capped_deck_edge",
            [3.55, 1.18],
            0.12,
            0.028,
            [0.0, 0.25, 0.0],
            1,
        ),
        deck_fragment(
            "notched_deck_edge",
            [3.45, 0.94],
            0.105,
            0.012,
            [0.0, 0.245, 0.0],
            1,
        ),
        brace_fragment(
            "straight_under_braces",
            [1.45, 0.042, 0.048],
            &[FragmentOccurrence::at([0.0, 0.0, 0.0])],
        ),
        brace_fragment(
            "cross_brace_beam",
            [1.5, 0.052, 0.052],
            &[FragmentOccurrence::at([0.0, 0.0, 0.0])],
        ),
        brace_fragment(
            "trussed_brace_beam",
            [1.55, 0.066, 0.06],
            &[FragmentOccurrence::at([0.0, 0.0, 0.0])],
        ),
        ramp_fragment(),
        rail_fragment("curb_rail", [1.7, 0.042, 0.052], 0.34, 0.57, 1),
        rail_fragment("guard_rail", [1.7, 0.04, 0.046], 0.62, 0.62, 2),
        rail_fragment("watch_rail", [1.72, 0.052, 0.058], 0.76, 0.66, 3),
    ];
    if quality == BridgeQuality::Hq {
        fragments.extend([
            support_box_fragment(
                "round_pile_supports",
                GeometrySource::RoundedBox {
                    half_extents: [0.085, 0.28, 0.085],
                    radius: 0.08,
                },
                vec![
                    bevel(1, 0.012),
                    linear_array(2, 7, [0.55, 0.0, 0.0]),
                    linear_array(3, 2, [0.0, 0.0, 0.82]),
                ],
            ),
            support_box_fragment(
                "squared_post_supports",
                GeometrySource::RoundedBox {
                    half_extents: [0.13, 0.28, 0.13],
                    radius: 0.012,
                },
                vec![
                    bevel(1, 0.018),
                    linear_array(2, 4, [1.18, 0.0, 0.0]),
                    linear_array(3, 2, [0.0, 0.0, 0.9]),
                ],
            ),
            support_box_fragment(
                "stone_pier_blocks",
                GeometrySource::RoundedBox {
                    half_extents: [0.34, 0.2, 0.34],
                    radius: 0.012,
                },
                vec![
                    bevel(1, 0.014),
                    linear_array(2, 4, [0.0, -0.44, 0.0]),
                    linear_array(3, 3, [1.7, 0.0, 0.0]),
                ],
            ),
            support_box_fragment(
                "trestle_frame_supports",
                GeometrySource::RoundedBox {
                    half_extents: [0.105, 0.28, 0.105],
                    radius: 0.014,
                },
                vec![
                    bevel(1, 0.016),
                    linear_array(2, 5, [0.78, 0.0, 0.0]),
                    linear_array(3, 2, [0.0, 0.0, 0.9]),
                ],
            ),
            deck_fragment(
                "segmented_deck_planks",
                [3.65, 0.18],
                0.082,
                0.014,
                [0.0, 0.255, 0.0],
                6,
            ),
            deck_fragment(
                "wide_plank_deck",
                [3.8, 0.2],
                0.1,
                0.018,
                [0.0, 0.26, 0.0],
                7,
            ),
            brace_fragment_with_operations(
                "minimal_under_ties",
                [1.55, 0.034, 0.036],
                vec![bevel(1, 0.012), linear_array(2, 2, [0.0, 0.0, 0.72])],
                &[FragmentOccurrence::at([0.0, -0.05, -0.36])],
            ),
            brace_fragment_with_operations(
                "x_brace_beam",
                [1.12, 0.052, 0.042],
                vec![
                    bevel(1, 0.014),
                    transform_geometry(2, [0.0, 0.0, 0.0], [0.0, 0.0, 24.0]),
                    linear_array(3, 2, [0.0, 0.18, 0.0]),
                    linear_array(4, 2, [0.0, 0.0, 0.68]),
                ],
                &[FragmentOccurrence::at([0.0, 0.0, -0.34])],
            ),
            brace_fragment_with_operations(
                "k_brace_beam",
                [0.82, 0.052, 0.042],
                vec![
                    bevel(1, 0.014),
                    transform_geometry(2, [0.18, 0.0, 0.0], [0.0, 0.0, -30.0]),
                    linear_array(3, 2, [0.0, 0.18, 0.0]),
                    linear_array(4, 2, [0.0, 0.0, 0.68]),
                ],
                &[FragmentOccurrence::at([0.0, 0.0, -0.34])],
            ),
            brace_fragment_with_operations(
                "heavy_reinforced_brace",
                [1.35, 0.07, 0.055],
                vec![
                    bevel(1, 0.014),
                    linear_array(2, 3, [0.0, 0.18, 0.0]),
                    linear_array(3, 2, [0.0, 0.0, 0.72]),
                ],
                &[FragmentOccurrence::at([0.0, 0.0, -0.36])],
            ),
            rail_fragment("low_curb_rail", [1.7, 0.038, 0.046], 0.38, 0.64, 1),
            rail_fragment("guard_rail_courses", [1.75, 0.044, 0.05], 0.78, 0.73, 3),
            rail_fragment("lookout_rail_courses", [1.8, 0.056, 0.06], 1.18, 0.82, 4),
            connector_fragment(
                "clean_joinery_detail",
                [0.09, 0.018, 0.44],
                vec![bevel(1, 0.006), linear_array(2, 3, [0.95, 0.0, 0.0])],
            ),
            connector_fragment(
                "bolted_joinery_detail",
                [0.055, 0.02, 0.045],
                vec![
                    bevel(1, 0.007),
                    linear_array(2, 7, [0.36, 0.0, 0.0]),
                    linear_array(3, 2, [0.0, 0.0, 0.94]),
                ],
            ),
            connector_fragment(
                "dense_weathered_joinery",
                [0.064, 0.024, 0.052],
                vec![
                    bevel(1, 0.008),
                    linear_array(2, 11, [0.22, 0.0, 0.0]),
                    linear_array(3, 3, [0.0, 0.0, 0.43]),
                ],
            ),
        ]);
    }
    let style_impl = style_implementation(
        ROMAN_TIMBER_STYLE_ID,
        BRIDGE_FAMILY_ID,
        default_role_providers,
        fragments,
    );
    let profile = customizer_profile(quality);
    let (slug, document_id, control_state) = match quality {
        BridgeQuality::Standard => (
            "roman-bridge",
            "roman-bridge-doc",
            BTreeMap::from([
                ("span_length".to_owned(), ControlValue::Scalar(3.6)),
                ("deck_width".to_owned(), ControlValue::Scalar(1.08)),
                ("structural_heft".to_owned(), ControlValue::Scalar(0.55)),
                (
                    "support_rhythm".to_owned(),
                    ControlValue::Provider("tight_pile_bents".to_owned()),
                ),
                (
                    "bracing_style".to_owned(),
                    ControlValue::Choice("cross_brace_beam".to_owned()),
                ),
                (
                    "railing".to_owned(),
                    ControlValue::Provider("guard_rail".to_owned()),
                ),
                (
                    "edge_finish".to_owned(),
                    ControlValue::Provider("lashed_deck_plank".to_owned()),
                ),
            ]),
        ),
        BridgeQuality::Hq => (
            "roman-bridge-hq",
            "roman-bridge-hq-doc",
            BTreeMap::from([
                ("span_length".to_owned(), ControlValue::Scalar(3.8)),
                ("deck_width".to_owned(), ControlValue::Scalar(1.14)),
                ("structural_heft".to_owned(), ControlValue::Scalar(0.62)),
                (
                    "support_style".to_owned(),
                    ControlValue::Provider("stone_pier_blocks".to_owned()),
                ),
                (
                    "bracing_style".to_owned(),
                    ControlValue::Choice("minimal_under_ties".to_owned()),
                ),
                (
                    "railing_style".to_owned(),
                    ControlValue::Provider("guard_rail_courses".to_owned()),
                ),
                (
                    "detail_density".to_owned(),
                    ControlValue::Provider("bolted_joinery_detail".to_owned()),
                ),
            ]),
        ),
    };

    build_fixture_catalog(FixtureCatalogSpec {
        slug,
        document_id,
        family,
        style,
        family_implementation: family_impl,
        style_implementation: style_impl,
        customizer_profile: profile,
        control_state,
    })
}

fn bridge_family_schema(quality: BridgeQuality) -> shape_family::AssetFamilySchema {
    let bracing_choices = match quality {
        BridgeQuality::Standard => vec![
            "straight_under_braces".to_owned(),
            "cross_brace_beam".to_owned(),
            "trussed_brace_beam".to_owned(),
        ],
        BridgeQuality::Hq => vec![
            "minimal_under_ties".to_owned(),
            "x_brace_beam".to_owned(),
            "k_brace_beam".to_owned(),
            "heavy_reinforced_brace".to_owned(),
        ],
    };
    let connector_role = match quality {
        BridgeQuality::Standard => role("connector", RoleMultiplicity::Optional, false),
        BridgeQuality::Hq => role("connector", RoleMultiplicity::Repeated, true),
    };
    let (span_min, span_max, span_default, deck_width_min, deck_width_max, deck_width_default) =
        match quality {
            BridgeQuality::Standard => (2.4, 5.2, 3.6, 0.78, 1.55, 1.08),
            BridgeQuality::Hq => (2.8, 5.4, 3.8, 0.86, 1.58, 1.14),
        };
    let heft_default = match quality {
        BridgeQuality::Standard => 0.55,
        BridgeQuality::Hq => 0.62,
    };
    let mut family = family_schema(FamilySchemaSpec {
        id: BRIDGE_FAMILY_ID,
        display_name: "Bridge",
        summary: "Theme-neutral crossing structure with attached load paths and walkable deck.",
        roles: vec![
            role("support", RoleMultiplicity::Repeated, true),
            role("span", RoleMultiplicity::Single, true),
            role("deck", RoleMultiplicity::Single, true),
            role("brace", RoleMultiplicity::Repeated, true),
            role("ramp", RoleMultiplicity::Repeated, true),
            role("rail", RoleMultiplicity::Repeated, true),
            connector_role,
        ],
        allowed_operations: vec![
            AllowedOperationKind::Primitive,
            AllowedOperationKind::Array,
            AllowedOperationKind::Transform,
            AllowedOperationKind::Bevel,
            AllowedOperationKind::Lathe,
        ],
        parameter_slots: vec![
            length_slot(
                "span_length",
                "Span Length",
                "span",
                span_min,
                span_max,
                0.1,
                span_default,
            ),
            length_slot(
                "deck_length",
                "Deck Length",
                "deck",
                span_min,
                span_max,
                0.1,
                span_default,
            ),
            length_slot(
                "deck_width",
                "Deck Width",
                "deck",
                deck_width_min,
                deck_width_max,
                0.05,
                deck_width_default,
            ),
            ratio_slot(
                "support_heft",
                "Support Heft",
                "support",
                0.0,
                1.0,
                0.05,
                heft_default,
            ),
            ratio_slot(
                "span_heft_y",
                "Span Depth",
                "span",
                0.0,
                1.0,
                0.05,
                heft_default,
            ),
            ratio_slot(
                "span_heft_z",
                "Span Breadth",
                "span",
                0.0,
                1.0,
                0.05,
                heft_default,
            ),
            ratio_slot(
                "brace_heft",
                "Brace Heft",
                "brace",
                0.0,
                1.0,
                0.05,
                heft_default,
            ),
            ratio_slot(
                "deck_heft",
                "Deck Heft",
                "deck",
                0.0,
                1.0,
                0.05,
                heft_default,
            ),
            choice_slot("bracing_style", "Bracing Style", "brace", bracing_choices),
        ],
        compatible_style_kits: vec![ROMAN_TIMBER_STYLE_ID.to_owned()],
        tags: vec!["roman".to_owned(), "bridge".to_owned(), "timber".to_owned()],
    });
    family.attachment_rules = vec![
        attachment_rule("support_to_span", "support", "span", "load_path"),
        attachment_rule("deck_to_span", "deck", "span", "deck_mount"),
        attachment_rule("brace_to_span", "brace", "span", "brace_mount"),
        attachment_rule("ramp_to_deck", "ramp", "deck", "walkway"),
        attachment_rule("rail_to_deck", "rail", "deck", "rail_mount"),
    ];
    if quality == BridgeQuality::Hq {
        family.attachment_rules.push(AttachmentRule {
            id: "connector_to_deck".to_owned(),
            from_role: "connector".to_owned(),
            to_role: "deck".to_owned(),
            anchor_role: None,
            compatibility_tags: vec!["deck_detail".to_owned()],
            required: false,
            execution_policy: FamilyRuleExecutionPolicy::Advisory,
        });
    }
    family
}

fn attachment_rule(id: &str, from_role: &str, to_role: &str, tag: &str) -> AttachmentRule {
    AttachmentRule {
        id: id.to_owned(),
        from_role: from_role.to_owned(),
        to_role: to_role.to_owned(),
        anchor_role: None,
        compatibility_tags: vec![tag.to_owned()],
        required: true,
        execution_policy: FamilyRuleExecutionPolicy::Required,
    }
}

fn roman_bridge_style_kit(quality: BridgeQuality) -> StyleKit {
    let mut style = roman_timber_engineering_style_kit();
    let facet = style
        .family_facets
        .get_mut(BRIDGE_FAMILY_ID)
        .expect("roman timber style kit declares bridge facet");
    facet.part_prototypes.extend([
        prototype(
            "tight_pile_bents",
            "Tight pile bents",
            "support",
            &["timber", "foundation", "rhythmic"],
        ),
        prototype(
            "marching_pile_bents",
            "Marching pile bents",
            "support",
            &["timber", "foundation", "reinforced"],
        ),
        prototype(
            "straight_under_braces",
            "Straight under braces",
            "brace",
            &["timber", "light", "reinforcement"],
        ),
        prototype(
            "trussed_brace_beam",
            "Trussed brace beam",
            "brace",
            &["timber", "truss", "reinforcement"],
        ),
        prototype(
            "capped_deck_edge",
            "Capped deck edge",
            "deck",
            &["timber", "capped", "walkable"],
        ),
        prototype(
            "notched_deck_edge",
            "Notched deck edge",
            "deck",
            &["timber", "notched", "walkable"],
        ),
        prototype(
            "bank_ramp_planks",
            "Bank ramp planks",
            "ramp",
            &["timber", "approach", "walkable"],
        ),
        prototype("curb_rail", "Curb rail", "rail", &["timber", "low_rail"]),
        prototype(
            "guard_rail",
            "Guard rail",
            "rail",
            &["timber", "guard_rail"],
        ),
        prototype("watch_rail", "Watch rail", "rail", &["timber", "tall_rail"]),
    ]);
    if quality == BridgeQuality::Hq {
        facet.part_prototypes.extend([
            prototype(
                "round_pile_supports",
                "Round pile supports",
                "support",
                &["timber", "foundation", "piles"],
            ),
            prototype(
                "squared_post_supports",
                "Squared post supports",
                "support",
                &["timber", "foundation", "squared"],
            ),
            prototype(
                "stone_pier_blocks",
                "Stone pier blocks",
                "support",
                &["stone", "foundation", "masonry"],
            ),
            prototype(
                "trestle_frame_supports",
                "Trestle frame supports",
                "support",
                &["timber", "foundation", "trestle"],
            ),
            prototype(
                "segmented_deck_planks",
                "Segmented deck planks",
                "deck",
                &["timber", "planked", "walkable"],
            ),
            prototype(
                "wide_plank_deck",
                "Wide plank deck",
                "deck",
                &["timber", "wide", "walkable"],
            ),
            prototype(
                "minimal_under_ties",
                "Minimal under ties",
                "brace",
                &["timber", "minimal", "reinforcement"],
            ),
            prototype(
                "x_brace_beam",
                "X brace beam",
                "brace",
                &["timber", "x_brace", "reinforcement"],
            ),
            prototype(
                "k_brace_beam",
                "K brace beam",
                "brace",
                &["timber", "k_brace", "reinforcement"],
            ),
            prototype(
                "heavy_reinforced_brace",
                "Heavy reinforced brace",
                "brace",
                &["timber", "heavy", "reinforcement"],
            ),
            prototype(
                "low_curb_rail",
                "Low curb rail",
                "rail",
                &["timber", "low_rail"],
            ),
            prototype(
                "guard_rail_courses",
                "Guard rail courses",
                "rail",
                &["timber", "guard_rail"],
            ),
            prototype(
                "lookout_rail_courses",
                "Lookout rail courses",
                "rail",
                &["timber", "tall_rail"],
            ),
            prototype(
                "clean_joinery_detail",
                "Clean joinery detail",
                "connector",
                &["timber", "joinery", "clean"],
            ),
            prototype(
                "bolted_joinery_detail",
                "Bolted joinery detail",
                "connector",
                &["timber", "joinery", "bolted"],
            ),
            prototype(
                "dense_weathered_joinery",
                "Dense weathered joinery",
                "connector",
                &["timber", "joinery", "weathered"],
            ),
        ]);
    }
    style
}

fn prototype(id: &str, label: &str, role: &str, style_tags: &[&str]) -> PartPrototype {
    PartPrototype {
        id: id.to_owned(),
        display_name: label.to_owned(),
        role: role.to_owned(),
        operation_tags: vec![
            AllowedOperationKind::Primitive,
            AllowedOperationKind::Array,
            AllowedOperationKind::Transform,
            AllowedOperationKind::Bevel,
        ],
        style_tags: style_tags.iter().map(|tag| (*tag).to_owned()).collect(),
    }
}

fn bridge_family_implementation(
    quality: BridgeQuality,
) -> shape_family_compile::FamilyImplementation {
    let bracing_choices = match quality {
        BridgeQuality::Standard => BTreeMap::from([
            (
                "straight_under_braces".to_owned(),
                "straight_under_braces".to_owned(),
            ),
            ("cross_brace_beam".to_owned(), "cross_brace_beam".to_owned()),
            (
                "trussed_brace_beam".to_owned(),
                "trussed_brace_beam".to_owned(),
            ),
        ]),
        BridgeQuality::Hq => BTreeMap::from([
            (
                "minimal_under_ties".to_owned(),
                "minimal_under_ties".to_owned(),
            ),
            ("x_brace_beam".to_owned(), "x_brace_beam".to_owned()),
            ("k_brace_beam".to_owned(), "k_brace_beam".to_owned()),
            (
                "heavy_reinforced_brace".to_owned(),
                "heavy_reinforced_brace".to_owned(),
            ),
        ]),
    };
    let mut parameter_bindings = vec![
        ParameterBinding::Scalar {
            slot: "span_length".to_owned(),
            role: "span".to_owned(),
            local_path: definition_scalar_path(
                LOCAL_DEFINITION,
                "geometry.rounded_box.half_extents.x",
            ),
            transform: ScalarTransform::ScaleOffset {
                scale: 0.5,
                offset: 0.0,
            },
        },
        ParameterBinding::Scalar {
            slot: "deck_length".to_owned(),
            role: "deck".to_owned(),
            local_path: definition_scalar_path(LOCAL_DEFINITION, "geometry.plate.size.x"),
            transform: ScalarTransform::Direct,
        },
    ];
    match quality {
        BridgeQuality::Standard => parameter_bindings.extend([
            ParameterBinding::Scalar {
                slot: "deck_width".to_owned(),
                role: "deck".to_owned(),
                local_path: definition_scalar_path(LOCAL_DEFINITION, "geometry.plate.size.y"),
                transform: ScalarTransform::Direct,
            },
            ParameterBinding::Scalar {
                slot: "support_heft".to_owned(),
                role: "support".to_owned(),
                local_path: definition_scalar_path(LOCAL_DEFINITION, "geometry.cylinder.radius"),
                transform: ScalarTransform::Ratio {
                    minimum: 0.105,
                    maximum: 0.21,
                },
            },
        ]),
        BridgeQuality::Hq => parameter_bindings.extend([
            ParameterBinding::Scalar {
                slot: "deck_width".to_owned(),
                role: "deck".to_owned(),
                local_path: definition_scalar_path(LOCAL_DEFINITION, "geometry.plate.size.y"),
                transform: ScalarTransform::ScaleOffset {
                    scale: 0.16,
                    offset: 0.0,
                },
            },
            ParameterBinding::Scalar {
                slot: "deck_width".to_owned(),
                role: "deck".to_owned(),
                local_path: definition_scalar_path(
                    LOCAL_DEFINITION,
                    "operation.2.linear_array.offset.z",
                ),
                transform: ScalarTransform::ScaleOffset {
                    scale: 0.18,
                    offset: 0.02,
                },
            },
            ParameterBinding::Scalar {
                slot: "support_heft".to_owned(),
                role: "support".to_owned(),
                local_path: definition_scalar_path(
                    LOCAL_DEFINITION,
                    "geometry.rounded_box.half_extents.x",
                ),
                transform: ScalarTransform::Ratio {
                    minimum: 0.07,
                    maximum: 0.22,
                },
            },
            ParameterBinding::Scalar {
                slot: "support_heft".to_owned(),
                role: "support".to_owned(),
                local_path: definition_scalar_path(
                    LOCAL_DEFINITION,
                    "geometry.rounded_box.half_extents.z",
                ),
                transform: ScalarTransform::Ratio {
                    minimum: 0.07,
                    maximum: 0.22,
                },
            },
        ]),
    }
    parameter_bindings.extend([
        ParameterBinding::Scalar {
            slot: "span_heft_y".to_owned(),
            role: "span".to_owned(),
            local_path: definition_scalar_path(
                LOCAL_DEFINITION,
                "geometry.rounded_box.half_extents.y",
            ),
            transform: ScalarTransform::Ratio {
                minimum: 0.075,
                maximum: 0.18,
            },
        },
        ParameterBinding::Scalar {
            slot: "span_heft_z".to_owned(),
            role: "span".to_owned(),
            local_path: definition_scalar_path(
                LOCAL_DEFINITION,
                "geometry.rounded_box.half_extents.z",
            ),
            transform: ScalarTransform::Ratio {
                minimum: 0.1,
                maximum: 0.24,
            },
        },
        ParameterBinding::Scalar {
            slot: "brace_heft".to_owned(),
            role: "brace".to_owned(),
            local_path: definition_scalar_path(
                LOCAL_DEFINITION,
                "geometry.rounded_box.half_extents.y",
            ),
            transform: ScalarTransform::Ratio {
                minimum: 0.032,
                maximum: 0.052,
            },
        },
        ParameterBinding::Scalar {
            slot: "deck_heft".to_owned(),
            role: "deck".to_owned(),
            local_path: definition_scalar_path(LOCAL_DEFINITION, "geometry.plate.thickness"),
            transform: ScalarTransform::Ratio {
                minimum: 0.07,
                maximum: 0.16,
            },
        },
        ParameterBinding::ChoiceToPrototype {
            slot: "bracing_style".to_owned(),
            role: "brace".to_owned(),
            choices: bracing_choices,
        },
    ]);
    let mut family_impl =
        family_implementation(BRIDGE_FAMILY_ID, "Roman bridge base", parameter_bindings);
    let support_offset = match quality {
        BridgeQuality::Standard => [-1.65, -0.76, -0.01],
        BridgeQuality::Hq => [-1.65, -0.5, -0.01],
    };
    let deck_offset = match quality {
        BridgeQuality::Standard => [0.0, 0.25, 0.45],
        BridgeQuality::Hq => [0.0, 0.3, 0.45],
    };
    let rail_offset = match quality {
        BridgeQuality::Standard => [0.0, 0.37, -0.62],
        BridgeQuality::Hq => [0.0, 0.5, -0.62],
    };
    let brace_offset = match quality {
        BridgeQuality::Standard => [0.0, -0.32, 0.45],
        BridgeQuality::Hq => [0.0, -0.62, 0.45],
    };
    family_impl.attachment_bindings = vec![
        attachment_binding(
            "support_to_span",
            "span",
            "span_joint",
            "support",
            "support_joint",
            FragmentAttachmentPairing::AllPairs,
            support_offset,
        ),
        attachment_binding(
            "deck_to_span",
            "span",
            "span_joint",
            "deck",
            "deck_joint",
            FragmentAttachmentPairing::ByOccurrenceIndex,
            deck_offset,
        ),
        attachment_binding(
            "brace_to_span",
            "span",
            "span_joint",
            "brace",
            "brace_joint",
            FragmentAttachmentPairing::AllPairs,
            brace_offset,
        ),
        attachment_binding(
            "ramp_to_deck",
            "deck",
            "deck_joint",
            "ramp",
            "ramp_joint",
            FragmentAttachmentPairing::AllPairs,
            [-2.15, 0.18, 0.0],
        ),
        attachment_binding(
            "rail_to_deck",
            "deck",
            "deck_joint",
            "rail",
            "rail_joint",
            FragmentAttachmentPairing::AllPairs,
            rail_offset,
        ),
    ];
    if quality == BridgeQuality::Hq {
        family_impl.attachment_bindings.push(attachment_binding(
            "connector_to_deck",
            "deck",
            "deck_joint",
            "connector",
            "connector_joint",
            FragmentAttachmentPairing::AllPairs,
            [-1.08, 0.24, -0.46],
        ));
    }
    family_impl
}

fn attachment_binding(
    rule_id: &str,
    parent_role: &str,
    parent_port: &str,
    child_role: &str,
    child_port: &str,
    pairing: FragmentAttachmentPairing,
    offset: [f32; 3],
) -> FragmentAttachmentBinding {
    FragmentAttachmentBinding {
        family_attachment_rule: rule_id.to_owned(),
        parent_role: parent_role.to_owned(),
        parent_port: parent_port.to_owned(),
        child_role: child_role.to_owned(),
        child_port: child_port.to_owned(),
        pairing,
        rigid_offset: RigidOffset {
            translation: offset,
            ..RigidOffset::default()
        },
        attachment_mode: AttachmentMode::RigidSeparate,
    }
}

fn customizer_profile(quality: BridgeQuality) -> CustomizerProfile {
    let (controls, candidate_strategies) = match quality {
        BridgeQuality::Standard => (
            vec![
                continuous_multi_control(
                    "span_length",
                    "Span Length",
                    3.6,
                    2.4,
                    5.2,
                    &["span_length", "deck_length"],
                ),
                continuous_multi_control(
                    "deck_width",
                    "Deck Width",
                    1.08,
                    0.78,
                    1.55,
                    &["deck_width"],
                ),
                continuous_multi_control(
                    "structural_heft",
                    "Structural Heft",
                    0.55,
                    0.0,
                    1.0,
                    &[
                        "support_heft",
                        "span_heft_y",
                        "span_heft_z",
                        "brace_heft",
                        "deck_heft",
                    ],
                ),
                provider_gallery_control(
                    "support_rhythm",
                    "Support Rhythm",
                    "support",
                    &[
                        ("pointed_round_pile", "Light Piles"),
                        ("tight_pile_bents", "Balanced Bents"),
                        ("marching_pile_bents", "Reinforced Bents"),
                    ],
                ),
                choice_gallery_control(
                    "bracing_style",
                    "Bracing Style",
                    "bracing_style",
                    &[
                        ("straight_under_braces", "Straight Under Braces"),
                        ("cross_brace_beam", "X Brace"),
                        ("trussed_brace_beam", "Truss Brace"),
                    ],
                ),
                provider_gallery_control(
                    "railing",
                    "Railing",
                    "rail",
                    &[
                        ("curb_rail", "Curb"),
                        ("guard_rail", "Guard"),
                        ("watch_rail", "Watch"),
                    ],
                ),
                provider_gallery_control(
                    "edge_finish",
                    "Edge Finish",
                    "deck",
                    &[
                        ("lashed_deck_plank", "Lashed"),
                        ("capped_deck_edge", "Capped"),
                        ("notched_deck_edge", "Notched"),
                    ],
                ),
            ],
            vec![
                strategy(
                    "light",
                    "Light",
                    &[
                        "structural_heft",
                        "support_rhythm",
                        "bracing_style",
                        "railing",
                        "edge_finish",
                    ],
                ),
                strategy(
                    "balanced",
                    "Balanced",
                    &[
                        "span_length",
                        "deck_width",
                        "structural_heft",
                        "support_rhythm",
                        "bracing_style",
                        "railing",
                        "edge_finish",
                    ],
                ),
                strategy(
                    "reinforced",
                    "Reinforced",
                    &[
                        "structural_heft",
                        "support_rhythm",
                        "bracing_style",
                        "railing",
                        "edge_finish",
                    ],
                ),
                strategy(
                    "wide_crossing",
                    "Wide Crossing",
                    &["span_length", "deck_width", "support_rhythm", "railing"],
                ),
            ],
        ),
        BridgeQuality::Hq => (
            vec![
                continuous_multi_control(
                    "span_length",
                    "Span Length",
                    3.8,
                    2.8,
                    5.4,
                    &["span_length", "deck_length"],
                ),
                continuous_multi_control(
                    "deck_width",
                    "Deck Width",
                    1.14,
                    0.86,
                    1.58,
                    &["deck_width"],
                ),
                continuous_multi_control(
                    "structural_heft",
                    "Structural Heft",
                    0.62,
                    0.18,
                    0.82,
                    &[
                        "support_heft",
                        "span_heft_y",
                        "span_heft_z",
                        "brace_heft",
                        "deck_heft",
                    ],
                ),
                provider_gallery_control(
                    "support_style",
                    "Support Style",
                    "support",
                    &[
                        ("round_pile_supports", "Round Piles"),
                        ("squared_post_supports", "Squared Posts"),
                        ("stone_pier_blocks", "Stone Piers"),
                        ("trestle_frame_supports", "Trestle Frames"),
                    ],
                ),
                choice_gallery_control(
                    "bracing_style",
                    "Bracing Style",
                    "bracing_style",
                    &[
                        ("minimal_under_ties", "Minimal Ties"),
                        ("x_brace_beam", "X Brace"),
                        ("k_brace_beam", "K Brace"),
                        ("heavy_reinforced_brace", "Heavy Reinforced"),
                    ],
                ),
                provider_gallery_control(
                    "railing_style",
                    "Railing Style",
                    "rail",
                    &[
                        ("low_curb_rail", "Low Curb"),
                        ("guard_rail_courses", "Guard Rails"),
                        ("lookout_rail_courses", "Lookout Rails"),
                    ],
                ),
                provider_gallery_control(
                    "detail_density",
                    "Detail Density",
                    "connector",
                    &[
                        ("clean_joinery_detail", "Clean"),
                        ("bolted_joinery_detail", "Bolted"),
                        ("dense_weathered_joinery", "Dense"),
                    ],
                ),
            ],
            vec![
                strategy(
                    "reinforced",
                    "Reinforced",
                    &[
                        "span_length",
                        "structural_heft",
                        "support_style",
                        "bracing_style",
                        "detail_density",
                    ],
                ),
                strategy(
                    "light_crossing",
                    "Light Crossing",
                    &[
                        "span_length",
                        "structural_heft",
                        "support_style",
                        "bracing_style",
                    ],
                ),
                strategy(
                    "wide_deck",
                    "Wide Crossing",
                    &["deck_width", "railing_style", "detail_density"],
                ),
                strategy(
                    "compact_span",
                    "Compact Span",
                    &[
                        "span_length",
                        "deck_width",
                        "structural_heft",
                        "bracing_style",
                    ],
                ),
                strategy(
                    "stone_pier_outpost",
                    "Stone-Pier Outpost",
                    &["support_style", "railing_style", "detail_density"],
                ),
                strategy(
                    "detailed_timberwork",
                    "Detailed Timberwork",
                    &[
                        "structural_heft",
                        "bracing_style",
                        "railing_style",
                        "detail_density",
                    ],
                ),
                strategy(
                    "minimal_clean_span",
                    "Minimal Span",
                    &["span_length", "bracing_style", "railing_style"],
                ),
            ],
        ),
    };
    CustomizerProfile {
        schema_version: shape_foundry::CUSTOMIZER_PROFILE_SCHEMA_VERSION,
        family_id: BRIDGE_FAMILY_ID.to_owned(),
        style_id: Some(ROMAN_TIMBER_STYLE_ID.to_owned()),
        sections: Vec::new(),
        controls,
        candidate_strategies,
        maximum_primary_controls: 7,
    }
}

fn continuous_multi_control(
    id: &str,
    label: &str,
    default: f32,
    minimum: f32,
    maximum: f32,
    slots: &[&str],
) -> CustomizerControl {
    CustomizerControl {
        id: id.to_owned(),
        label: label.to_owned(),
        section: None,
        primary: true,
        visible: true,
        kind: ControlKind::ContinuousAxis { default },
        bindings: slots
            .iter()
            .map(|slot| ControlSlotBinding {
                slot: (*slot).to_owned(),
                slot_policy: ParameterExecutionPolicy::RequiredBinding,
                response: ResponseCurve::Linear,
            })
            .collect(),
        domain: FeasibleControlDomain {
            continuous_intervals: vec![ClosedInterval { minimum, maximum }],
            discrete_values: Vec::new(),
            unavailable_options: BTreeMap::new(),
            certification: DomainCertification::CertifiedContinuous,
        },
        topology_behavior: ControlTopologyBehavior::TopologyPreserving,
        divergence: shape_foundry::ControlDivergence::Synced,
    }
}

fn provider_gallery_control(
    id: &str,
    label: &str,
    role: &str,
    providers: &[(&str, &str)],
) -> CustomizerControl {
    CustomizerControl {
        id: id.to_owned(),
        label: label.to_owned(),
        section: None,
        primary: true,
        visible: true,
        kind: ControlKind::ProviderGallery {
            role: role.to_owned(),
            options: providers
                .iter()
                .map(|(provider_id, label)| ProviderOption {
                    provider_id: (*provider_id).to_owned(),
                    label: (*label).to_owned(),
                    preview: WholeModelPreviewRef {
                        preview_id: format!("{id}-{provider_id}"),
                        artifact_fingerprint: None,
                    },
                })
                .collect(),
        },
        bindings: Vec::new(),
        domain: FeasibleControlDomain {
            continuous_intervals: Vec::new(),
            discrete_values: providers
                .iter()
                .map(|(provider_id, _)| ControlValue::Provider((*provider_id).to_owned()))
                .collect(),
            unavailable_options: BTreeMap::new(),
            certification: DomainCertification::DiscreteSamples,
        },
        topology_behavior: ControlTopologyBehavior::TopologyChanging,
        divergence: shape_foundry::ControlDivergence::Synced,
    }
}

fn choice_gallery_control(
    id: &str,
    label: &str,
    slot: &str,
    values: &[(&str, &str)],
) -> CustomizerControl {
    CustomizerControl {
        id: id.to_owned(),
        label: label.to_owned(),
        section: None,
        primary: true,
        visible: true,
        kind: ControlKind::ChoiceGallery {
            options: values
                .iter()
                .map(|(value, label)| ChoiceOption {
                    value: (*value).to_owned(),
                    label: (*label).to_owned(),
                    preview: WholeModelPreviewRef {
                        preview_id: format!("{id}-{value}"),
                        artifact_fingerprint: None,
                    },
                })
                .collect(),
        },
        bindings: vec![ControlSlotBinding {
            slot: slot.to_owned(),
            slot_policy: ParameterExecutionPolicy::RequiredBinding,
            response: ResponseCurve::Linear,
        }],
        domain: FeasibleControlDomain {
            continuous_intervals: Vec::new(),
            discrete_values: values
                .iter()
                .map(|(value, _)| ControlValue::Choice((*value).to_owned()))
                .collect(),
            unavailable_options: BTreeMap::new(),
            certification: DomainCertification::DiscreteSamples,
        },
        topology_behavior: ControlTopologyBehavior::TopologyChanging,
        divergence: shape_foundry::ControlDivergence::Synced,
    }
}

fn strategy(id: &str, label: &str, controls: &[&str]) -> CandidateStrategy {
    CandidateStrategy {
        id: id.to_owned(),
        label: label.to_owned(),
        control_ids: controls
            .iter()
            .map(|control| (*control).to_owned())
            .collect(),
    }
}

#[derive(Debug, Copy, Clone)]
struct FragmentOccurrence {
    translation: [f32; 3],
    rotation_degrees: [f32; 3],
}

impl FragmentOccurrence {
    fn at(translation: [f32; 3]) -> Self {
        Self {
            translation,
            rotation_degrees: [0.0, 0.0, 0.0],
        }
    }
}

fn support_fragment(id: &str, radius: f32, count: u32, spacing: f32) -> RecipeFragment {
    let mut operations = vec![bevel(1, 0.018)];
    if count > 1 {
        operations.push(linear_array(2, count, [spacing, 0.0, 0.0]));
    }
    bridge_fragment(FragmentSpec {
        id,
        role: "support",
        source: GeometrySource::Cylinder {
            radius,
            height: 1.25,
            radial_segments: 18,
        },
        operations,
        occurrences: vec![FragmentOccurrence::at([0.0, 0.0, 0.0])],
        port_id: "support_joint",
        compatibility_tags: vec!["load_path"],
        scalar_paths: vec![
            ("geometry.cylinder.radius", 0.08, 0.26, 0.01),
            ("geometry.cylinder.height", 0.7, 1.8, 0.05),
        ],
    })
}

fn span_fragment() -> RecipeFragment {
    bridge_fragment(FragmentSpec {
        id: "hewn_span_beam",
        role: "span",
        source: GeometrySource::RoundedBox {
            half_extents: [1.8, 0.12, 0.15],
            radius: 0.025,
        },
        operations: vec![bevel(1, 0.024), linear_array(2, 2, [0.0, 0.0, 0.9])],
        occurrences: vec![FragmentOccurrence::at([0.0, 0.0, -0.45])],
        port_id: "span_joint",
        compatibility_tags: vec!["load_path", "deck_mount", "brace_mount"],
        scalar_paths: rounded_box_scalar_paths(),
    })
}

fn deck_fragment(
    id: &str,
    size: [f32; 2],
    thickness: f32,
    bevel_radius: f32,
    translation: [f32; 3],
    courses: u32,
) -> RecipeFragment {
    let mut operations = vec![bevel(1, bevel_radius)];
    if courses > 1 {
        operations.push(linear_array(2, courses, [0.0, 0.0, size[1] * 1.12]));
    }
    let mut scalar_paths = vec![
        ("geometry.plate.size.x", 2.0, 5.8, 0.05),
        ("geometry.plate.size.y", 0.05, 1.7, 0.05),
        ("geometry.plate.thickness", 0.04, 0.22, 0.01),
    ];
    if courses > 1 {
        scalar_paths.push(("operation.2.linear_array.offset.z", 0.05, 0.4, 0.01));
    }
    bridge_fragment(FragmentSpec {
        id,
        role: "deck",
        source: GeometrySource::Plate { size, thickness },
        operations,
        occurrences: vec![FragmentOccurrence::at(translation)],
        port_id: "deck_joint",
        compatibility_tags: vec!["deck_mount", "walkway", "rail_mount", "deck_detail"],
        scalar_paths,
    })
}

fn support_box_fragment(
    id: &str,
    source: GeometrySource,
    operations: Vec<ModelingOperationSpec>,
) -> RecipeFragment {
    bridge_fragment(FragmentSpec {
        id,
        role: "support",
        source,
        operations,
        occurrences: vec![FragmentOccurrence::at([0.0, 0.0, 0.0])],
        port_id: "support_joint",
        compatibility_tags: vec!["load_path"],
        scalar_paths: rounded_box_scalar_paths(),
    })
}

fn brace_fragment(
    id: &str,
    half_extents: [f32; 3],
    occurrences: &[FragmentOccurrence],
) -> RecipeFragment {
    brace_fragment_with_operations(
        id,
        half_extents,
        vec![bevel(1, 0.014), linear_array(2, 2, [0.0, 0.0, 0.16])],
        occurrences,
    )
}

fn brace_fragment_with_operations(
    id: &str,
    half_extents: [f32; 3],
    operations: Vec<ModelingOperationSpec>,
    occurrences: &[FragmentOccurrence],
) -> RecipeFragment {
    bridge_fragment(FragmentSpec {
        id,
        role: "brace",
        source: GeometrySource::RoundedBox {
            half_extents,
            radius: 0.018,
        },
        operations,
        occurrences: occurrences.to_vec(),
        port_id: "brace_joint",
        compatibility_tags: vec!["brace_mount"],
        scalar_paths: rounded_box_scalar_paths(),
    })
}

fn connector_fragment(
    id: &str,
    half_extents: [f32; 3],
    operations: Vec<ModelingOperationSpec>,
) -> RecipeFragment {
    bridge_fragment(FragmentSpec {
        id,
        role: "connector",
        source: GeometrySource::RoundedBox {
            half_extents,
            radius: 0.008,
        },
        operations,
        occurrences: vec![FragmentOccurrence::at([0.0, 0.0, 0.0])],
        port_id: "connector_joint",
        compatibility_tags: vec!["deck_detail"],
        scalar_paths: connector_scalar_paths(),
    })
}

fn ramp_fragment() -> RecipeFragment {
    bridge_fragment(FragmentSpec {
        id: "bank_ramp_planks",
        role: "ramp",
        source: GeometrySource::Plate {
            size: [0.92, 1.0],
            thickness: 0.075,
        },
        operations: vec![bevel(1, 0.012), linear_array(2, 2, [4.3, 0.0, 0.0])],
        occurrences: vec![FragmentOccurrence::at([0.0, 0.0, 0.0])],
        port_id: "ramp_joint",
        compatibility_tags: vec!["walkway"],
        scalar_paths: Vec::new(),
    })
}

fn rail_fragment(
    id: &str,
    half_extents: [f32; 3],
    height: f32,
    side_offset: f32,
    rail_courses: u32,
) -> RecipeFragment {
    let mut operations = vec![bevel(1, 0.012)];
    if rail_courses > 1 {
        operations.push(linear_array(2, rail_courses, [0.0, -0.2, 0.0]));
    }
    operations.push(linear_array(3, 2, [0.0, 0.0, side_offset * 2.0]));
    bridge_fragment(FragmentSpec {
        id,
        role: "rail",
        source: GeometrySource::RoundedBox {
            half_extents,
            radius: 0.018,
        },
        operations,
        occurrences: vec![FragmentOccurrence::at([0.0, height, -side_offset])],
        port_id: "rail_joint",
        compatibility_tags: vec!["rail_mount"],
        scalar_paths: Vec::new(),
    })
}

struct FragmentSpec<'a> {
    id: &'a str,
    role: &'a str,
    source: GeometrySource,
    operations: Vec<ModelingOperationSpec>,
    occurrences: Vec<FragmentOccurrence>,
    port_id: &'a str,
    compatibility_tags: Vec<&'a str>,
    scalar_paths: Vec<(&'a str, f32, f32, f32)>,
}

fn bridge_fragment(spec: FragmentSpec<'_>) -> RecipeFragment {
    let mut recipe = AssetRecipe::new(AssetId(1), format!("{} fragment", spec.id));
    let socket_tags = spec
        .compatibility_tags
        .iter()
        .map(|tag| (*tag).to_owned())
        .chain([spec.role.to_owned()])
        .collect::<BTreeSet<_>>();
    recipe.definitions.insert(
        LOCAL_DEFINITION,
        PartDefinition {
            id: LOCAL_DEFINITION,
            name: format!("{} definition", spec.id),
            tags: BTreeSet::from([spec.role.to_owned(), format!("role:{}", spec.role)]),
            geometry: GeometryRecipe {
                source: spec.source,
                operations: spec.operations,
            },
            regions: BTreeMap::new(),
            sockets: BTreeMap::from([(
                LOCAL_SOCKET,
                SocketSpec {
                    id: LOCAL_SOCKET,
                    name: format!("{} joint", spec.role),
                    local_frame: Frame3::default(),
                    role: spec.role.to_owned(),
                    tags: socket_tags,
                },
            )]),
            local_pivot: Frame3::default(),
            variant_group: None,
            production_hints: None,
        },
    );

    let mut role_roots = Vec::with_capacity(spec.occurrences.len());
    for (index, occurrence) in spec.occurrences.iter().enumerate() {
        let instance_id = PartInstanceId(FIRST_INSTANCE + index as u64);
        recipe.instances.insert(
            instance_id,
            PartInstance {
                id: instance_id,
                definition: LOCAL_DEFINITION,
                name: format!("{} {} {}", spec.id, spec.role, index + 1),
                parent: None,
                local_transform: Transform3 {
                    translation: occurrence.translation,
                    rotation_degrees: occurrence.rotation_degrees,
                    ..Transform3::default()
                },
                attachment: None,
                enabled: true,
                tags: BTreeSet::from([spec.role.to_owned(), format!("role:{}", spec.role)]),
                generated_by: None,
            },
        );
        role_roots.push(instance_id);
    }
    recipe.root_instances = role_roots.clone();

    for (index, (path, minimum, maximum, step)) in spec.scalar_paths.iter().enumerate() {
        let parameter_id = (index + 1) as u64;
        recipe.parameters.insert(
            shape_asset::ParameterId(parameter_id),
            scalar_parameter(
                parameter_id,
                definition_scalar_path(LOCAL_DEFINITION, path),
                format!("{} {}", spec.id, path),
                *minimum,
                *maximum,
                *step,
                false,
            ),
        );
    }

    let next_operation = recipe
        .definitions
        .get(&LOCAL_DEFINITION)
        .expect("definition exists")
        .geometry
        .operations
        .iter()
        .map(ModelingOperationSpec::operation_id)
        .map(|id| id.0)
        .max()
        .unwrap_or(0)
        + 1;
    recipe.next_ids.part_definition = LOCAL_DEFINITION.0 + 1;
    recipe.next_ids.part_instance = FIRST_INSTANCE + role_roots.len() as u64;
    recipe.next_ids.parameter = spec.scalar_paths.len() as u64 + 1;
    recipe.next_ids.operation = next_operation;
    recipe.next_ids.socket = LOCAL_SOCKET.0 + 1;
    let validation = validate_asset_recipe(&recipe);
    assert!(
        validation.is_valid(),
        "{} fragment failed validation: {:#?}",
        spec.id,
        validation.issues
    );

    RecipeFragment {
        schema_version: RECIPE_FRAGMENT_SCHEMA_VERSION,
        id: spec.id.to_owned(),
        provided_role: spec.role.to_owned(),
        exports: RecipeFragmentExports {
            role_occurrence_roots: role_roots.clone(),
            internal_roots: Vec::new(),
            socket_ports: vec![FragmentSocketPort {
                id: spec.port_id.to_owned(),
                local_occurrence_root: role_roots[0],
                local_socket: LOCAL_SOCKET,
                compatibility_tags: spec
                    .compatibility_tags
                    .iter()
                    .map(|tag| (*tag).to_owned())
                    .collect(),
            }],
            surface_ports: Vec::new(),
        },
        recipe,
    }
}

fn rounded_box_scalar_paths() -> Vec<(&'static str, f32, f32, f32)> {
    vec![
        ("geometry.rounded_box.half_extents.x", 0.05, 5.0, 0.05),
        ("geometry.rounded_box.half_extents.y", 0.025, 1.0, 0.01),
        ("geometry.rounded_box.half_extents.z", 0.025, 0.5, 0.01),
        ("geometry.rounded_box.radius", 0.0, 0.2, 0.01),
    ]
}

fn connector_scalar_paths() -> Vec<(&'static str, f32, f32, f32)> {
    vec![
        ("geometry.rounded_box.half_extents.x", 0.01, 0.25, 0.005),
        ("geometry.rounded_box.half_extents.y", 0.005, 0.1, 0.005),
        ("geometry.rounded_box.half_extents.z", 0.01, 0.5, 0.005),
        ("geometry.rounded_box.radius", 0.0, 0.05, 0.005),
    ]
}

fn bevel(operation: u64, radius: f32) -> ModelingOperationSpec {
    ModelingOperationSpec::SetBevelProfile {
        operation: OperationId(operation),
        radius,
        segments: 1,
    }
}

fn transform_geometry(
    operation: u64,
    translation: [f32; 3],
    rotation_degrees: [f32; 3],
) -> ModelingOperationSpec {
    ModelingOperationSpec::TransformGeometry {
        operation: OperationId(operation),
        transform: Transform3 {
            translation,
            rotation_degrees,
            ..Transform3::default()
        },
    }
}

fn linear_array(operation: u64, count: u32, offset: [f32; 3]) -> ModelingOperationSpec {
    ModelingOperationSpec::LinearArray {
        operation: OperationId(operation),
        count,
        offset,
    }
}
