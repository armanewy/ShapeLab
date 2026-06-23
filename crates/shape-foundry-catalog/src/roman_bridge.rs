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
    CandidateStrategy, ClosedInterval, ControlKind, ControlSlotBinding, ControlTopologyBehavior,
    ControlValue, CustomizerControl, CustomizerProfile, DomainCertification, FeasibleControlDomain,
    ProviderOption, ResponseCurve, WholeModelPreviewRef,
};

use crate::{
    FamilySchemaSpec, FixtureCatalogSpec, FoundryFixtureCatalog, build_fixture_catalog,
    choice_control, choice_slot, family_implementation, family_schema, length_slot, ratio_slot,
    role, style_implementation,
};

const BRIDGE_FAMILY_ID: &str = "bridge";
const ROMAN_TIMBER_STYLE_ID: &str = "roman_timber_engineering";
const LOCAL_DEFINITION: PartDefinitionId = PartDefinitionId(90);
const LOCAL_SOCKET: SocketId = SocketId(7);
const FIRST_INSTANCE: u64 = 91;

/// Build the Roman bridge fixture catalog.
#[must_use]
pub fn fixture_catalog() -> FoundryFixtureCatalog {
    let family = bridge_family_schema();
    let style = roman_bridge_style_kit();
    let family_impl = bridge_family_implementation();
    let style_impl = style_implementation(
        ROMAN_TIMBER_STYLE_ID,
        BRIDGE_FAMILY_ID,
        BTreeMap::from([
            ("support".to_owned(), "tight_pile_bents".to_owned()),
            ("span".to_owned(), "hewn_span_beam".to_owned()),
            ("deck".to_owned(), "lashed_deck_plank".to_owned()),
            ("brace".to_owned(), "cross_brace_beam".to_owned()),
            ("ramp".to_owned(), "bank_ramp_planks".to_owned()),
            ("rail".to_owned(), "guard_rail".to_owned()),
        ]),
        vec![
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
                2,
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
        ],
    );
    let profile = customizer_profile();

    build_fixture_catalog(FixtureCatalogSpec {
        slug: "roman-bridge",
        document_id: "roman-bridge-doc",
        family,
        style,
        family_implementation: family_impl,
        style_implementation: style_impl,
        customizer_profile: profile,
        control_state: BTreeMap::from([
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
    })
}

fn bridge_family_schema() -> shape_family::AssetFamilySchema {
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
            role("connector", RoleMultiplicity::Optional, false),
        ],
        allowed_operations: vec![
            AllowedOperationKind::Primitive,
            AllowedOperationKind::Array,
            AllowedOperationKind::Transform,
            AllowedOperationKind::Bevel,
            AllowedOperationKind::Lathe,
        ],
        parameter_slots: vec![
            length_slot("span_length", "Span Length", "span", 2.4, 5.2, 0.1, 3.6),
            length_slot("deck_length", "Deck Length", "deck", 2.4, 5.2, 0.1, 3.6),
            length_slot("deck_width", "Deck Width", "deck", 0.78, 1.55, 0.05, 1.08),
            ratio_slot(
                "support_heft",
                "Support Heft",
                "support",
                0.0,
                1.0,
                0.05,
                0.55,
            ),
            ratio_slot("span_heft_y", "Span Depth", "span", 0.0, 1.0, 0.05, 0.55),
            ratio_slot("span_heft_z", "Span Breadth", "span", 0.0, 1.0, 0.05, 0.55),
            ratio_slot("brace_heft", "Brace Heft", "brace", 0.0, 1.0, 0.05, 0.55),
            ratio_slot("deck_heft", "Deck Heft", "deck", 0.0, 1.0, 0.05, 0.55),
            choice_slot(
                "bracing_style",
                "Bracing Style",
                "brace",
                vec![
                    "straight_under_braces".to_owned(),
                    "cross_brace_beam".to_owned(),
                    "trussed_brace_beam".to_owned(),
                ],
            ),
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

fn roman_bridge_style_kit() -> StyleKit {
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

fn bridge_family_implementation() -> shape_family_compile::FamilyImplementation {
    let mut family_impl = family_implementation(
        BRIDGE_FAMILY_ID,
        "Roman bridge base",
        vec![
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
                    minimum: 0.035,
                    maximum: 0.09,
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
                choices: BTreeMap::from([
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
            },
        ],
    );
    family_impl.attachment_bindings = vec![
        attachment_binding(
            "support_to_span",
            "span",
            "span_joint",
            "support",
            "support_joint",
            FragmentAttachmentPairing::AllPairs,
            [-1.65, -0.76, -0.01],
        ),
        attachment_binding(
            "deck_to_span",
            "span",
            "span_joint",
            "deck",
            "deck_joint",
            FragmentAttachmentPairing::ByOccurrenceIndex,
            [0.0, 0.25, 0.45],
        ),
        attachment_binding(
            "brace_to_span",
            "span",
            "span_joint",
            "brace",
            "brace_joint",
            FragmentAttachmentPairing::AllPairs,
            [0.0, -0.32, 0.45],
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
            [0.0, 0.37, -0.62],
        ),
    ];
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

fn customizer_profile() -> CustomizerProfile {
    CustomizerProfile {
        schema_version: shape_foundry::CUSTOMIZER_PROFILE_SCHEMA_VERSION,
        family_id: BRIDGE_FAMILY_ID.to_owned(),
        style_id: Some(ROMAN_TIMBER_STYLE_ID.to_owned()),
        sections: Vec::new(),
        controls: vec![
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
            choice_control(
                "bracing_style",
                "Bracing Style",
                "bracing_style",
                &[
                    "straight_under_braces",
                    "cross_brace_beam",
                    "trussed_brace_beam",
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
        candidate_strategies: vec![
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
    _courses: u32,
) -> RecipeFragment {
    bridge_fragment(FragmentSpec {
        id,
        role: "deck",
        source: GeometrySource::Plate { size, thickness },
        operations: vec![bevel(1, bevel_radius)],
        occurrences: vec![FragmentOccurrence::at(translation)],
        port_id: "deck_joint",
        compatibility_tags: vec!["deck_mount", "walkway", "rail_mount"],
        scalar_paths: vec![
            ("geometry.plate.size.x", 2.0, 5.8, 0.05),
            ("geometry.plate.size.y", 0.65, 1.7, 0.05),
            ("geometry.plate.thickness", 0.04, 0.22, 0.01),
        ],
    })
}

fn brace_fragment(
    id: &str,
    half_extents: [f32; 3],
    occurrences: &[FragmentOccurrence],
) -> RecipeFragment {
    bridge_fragment(FragmentSpec {
        id,
        role: "brace",
        source: GeometrySource::RoundedBox {
            half_extents,
            radius: 0.018,
        },
        operations: vec![bevel(1, 0.014), linear_array(2, 2, [0.0, 0.0, 0.16])],
        occurrences: occurrences.to_vec(),
        port_id: "brace_joint",
        compatibility_tags: vec!["brace_mount"],
        scalar_paths: rounded_box_scalar_paths(),
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
    assert!(validate_asset_recipe(&recipe).is_valid());

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
        ("geometry.rounded_box.half_extents.y", 0.025, 0.5, 0.01),
        ("geometry.rounded_box.half_extents.z", 0.025, 0.5, 0.01),
        ("geometry.rounded_box.radius", 0.0, 0.2, 0.01),
    ]
}

fn bevel(operation: u64, radius: f32) -> ModelingOperationSpec {
    ModelingOperationSpec::SetBevelProfile {
        operation: OperationId(operation),
        radius,
        segments: 1,
    }
}

fn linear_array(operation: u64, count: u32, offset: [f32; 3]) -> ModelingOperationSpec {
    ModelingOperationSpec::LinearArray {
        operation: OperationId(operation),
        count,
        offset,
    }
}
