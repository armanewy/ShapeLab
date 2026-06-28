//! Stylized lamp headless foundry fixture.

use std::collections::{BTreeMap, BTreeSet};

use shape_asset::{
    AssetId, AssetRecipe, AttachmentMode, Frame3, GeometryRecipe, GeometrySource,
    ModelingOperationSpec, OperationId, ParameterId, PartDefinition, PartDefinitionId,
    PartInstance, PartInstanceId, SocketId, SocketSpec, Transform3, definition_scalar_path,
    instance_scalar_path, validate_asset_recipe,
};
use shape_family::{
    AllowedOperationKind, AttachmentRule, FamilyRuleExecutionPolicy, RoleMultiplicity,
};
use shape_family_compile::{
    FAMILY_IMPLEMENTATION_SCHEMA_VERSION, FragmentAttachmentBinding, FragmentAttachmentPairing,
    FragmentSocketPort, ParameterBinding, RECIPE_FRAGMENT_SCHEMA_VERSION, RecipeFragment,
    RecipeFragmentExports, RigidOffset, STYLE_IMPLEMENTATION_SCHEMA_VERSION, ScalarTransform,
    scalar_parameter,
};
use shape_foundry::{
    CandidateStrategy, ControlValue, FoundryPartGroupDescriptor,
    built_in_part_group_descriptors_for_profile,
};

use crate::{
    FamilySchemaSpec, FixtureCatalogSpec, FoundryFixtureCatalog, build_fixture_catalog,
    choice_control, choice_slot, continuous_control, family_implementation, family_schema,
    length_slot, ratio_slot, role, style_implementation, style_kit,
};

const LOCAL_DEFINITION: PartDefinitionId = PartDefinitionId(90);
const LOCAL_INSTANCE: PartInstanceId = PartInstanceId(91);
const LOCAL_SECOND_INSTANCE: PartInstanceId = PartInstanceId(92);
const LOCAL_TRIM_DEFINITION: PartDefinitionId = PartDefinitionId(93);
const LOCAL_TRIM_INSTANCE: PartInstanceId = PartInstanceId(94);
const LOCAL_BRACKET_DEFINITION: PartDefinitionId = PartDefinitionId(95);
const LOCAL_BRACKET_INSTANCE: PartInstanceId = PartInstanceId(96);
const LOCAL_BASE_SLAB_DEFINITION: PartDefinitionId = PartDefinitionId(97);
const LOCAL_BASE_SLAB_INSTANCE: PartInstanceId = PartInstanceId(98);
const LOCAL_SHADE_BODY_DEFINITION: PartDefinitionId = PartDefinitionId(99);
const LOCAL_SHADE_BODY_INSTANCE: PartInstanceId = PartInstanceId(100);
const LOCAL_UPPER_TRIM_INSTANCE: PartInstanceId = PartInstanceId(101);

const SOCKET_PRIMARY: SocketId = SocketId(7);
const SOCKET_SECONDARY: SocketId = SocketId(8);
const SOCKET_TERTIARY: SocketId = SocketId(9);

const OPERATION_BEVEL: OperationId = OperationId(1);
const OPERATION_TRIM_BEVEL: OperationId = OperationId(2);
const OPERATION_BRACKET_BEVEL: OperationId = OperationId(3);
const OPERATION_SHADE_MOUNT_BEVEL: OperationId = OperationId(4);

const SHADE_STYLE_VALUES: [&str; 6] = ["cone", "drum", "task", "wide", "minimal", "playful"];

/// Build the stylized lamp fixture catalog.
#[must_use]
pub fn fixture_catalog() -> FoundryFixtureCatalog {
    let mut family = family_schema(FamilySchemaSpec {
        id: "lamp",
        display_name: "Lamp",
        summary: "Theme-neutral stylized lamp with an explicit assembled base, stem, joints, and shade.",
        roles: vec![
            role("base", RoleMultiplicity::Single, true),
            role("stem", RoleMultiplicity::Single, true),
            role("joint", RoleMultiplicity::Single, true),
            role("shade", RoleMultiplicity::Single, true),
        ],
        allowed_operations: vec![
            AllowedOperationKind::Primitive,
            AllowedOperationKind::Transform,
            AllowedOperationKind::Bevel,
            AllowedOperationKind::Sweep,
            AllowedOperationKind::Lathe,
        ],
        parameter_slots: vec![
            length_slot(
                "overall_height",
                "Overall Height",
                "stem",
                1.1,
                2.2,
                0.05,
                1.55,
            ),
            ratio_slot("base_weight", "Base Weight", "base", 0.0, 1.0, 0.05, 0.55),
            ratio_slot(
                "stem_curvature",
                "Stem Curvature",
                "stem",
                0.0,
                1.0,
                0.05,
                0.42,
            ),
            ratio_slot("joint_size", "Joint Size", "joint", 0.0, 1.0, 0.05, 0.5),
            choice_slot(
                "shade_style",
                "Shade Style",
                "shade",
                SHADE_STYLE_VALUES
                    .iter()
                    .map(|value| (*value).to_owned())
                    .collect(),
            ),
            ratio_slot("shade_scale", "Shade Scale", "shade", 0.0, 1.0, 0.05, 0.55),
            ratio_slot(
                "edge_softness",
                "Edge Softness",
                "stem",
                0.0,
                1.0,
                0.05,
                0.45,
            ),
        ],
        compatible_style_kits: vec!["stylized_furniture".to_owned()],
        tags: vec![
            "lighting".to_owned(),
            "stylized".to_owned(),
            "lamp".to_owned(),
        ],
    });
    family.attachment_rules = lamp_attachment_rules();

    let mut style = style_kit(
        "stylized_furniture",
        "Stylized Furniture",
        "lamp",
        &[
            ("lathed_weighted_base", "Lathed weighted base", "base"),
            ("swept_curve_stem", "Swept curve stem", "stem"),
            ("pivot_disc_pair", "Pivot disc pair", "joint"),
            ("ribbed_cone_shade", "Ribbed cone shade", "shade"),
            ("banded_drum_shade", "Banded drum shade", "shade"),
            ("angled_task_shade", "Angled task shade", "shade"),
            ("wide_reading_shade", "Wide reading shade", "shade"),
            ("minimal_shade", "Minimal shade", "shade"),
            ("playful_tilt_shade", "Playful tilted shade", "shade"),
        ],
        vec![
            "furniture".to_owned(),
            "soft".to_owned(),
            "lighting".to_owned(),
        ],
    );
    tag_lamp_prototype_operations(&mut style);

    let mut family_impl = family_implementation(
        "lamp",
        "Stylized lamp base",
        vec![
            scalar_binding(
                "overall_height",
                "stem",
                definition_scalar_path(LOCAL_DEFINITION, "geometry.sweep.path.1.origin.y"),
                ScalarTransform::ScaleOffset {
                    scale: 0.52,
                    offset: -0.04,
                },
            ),
            scalar_binding(
                "overall_height",
                "stem",
                definition_scalar_path(LOCAL_DEFINITION, "geometry.sweep.path.2.origin.y"),
                ScalarTransform::ScaleOffset {
                    scale: 1.02,
                    offset: -0.25,
                },
            ),
            scalar_binding(
                "overall_height",
                "joint",
                instance_scalar_path(LOCAL_SECOND_INSTANCE, "transform.translation.y"),
                ScalarTransform::ScaleOffset {
                    scale: 1.02,
                    offset: -0.25,
                },
            ),
            scalar_binding(
                "base_weight",
                "base",
                instance_scalar_path(LOCAL_INSTANCE, "transform.scale.x"),
                ScalarTransform::Ratio {
                    minimum: 0.58,
                    maximum: 1.72,
                },
            ),
            scalar_binding(
                "base_weight",
                "base",
                instance_scalar_path(LOCAL_INSTANCE, "transform.scale.z"),
                ScalarTransform::Ratio {
                    minimum: 0.58,
                    maximum: 1.32,
                },
            ),
            scalar_binding(
                "base_weight",
                "base",
                instance_scalar_path(LOCAL_INSTANCE, "transform.scale.y"),
                ScalarTransform::Ratio {
                    minimum: 0.70,
                    maximum: 1.05,
                },
            ),
            scalar_binding(
                "base_weight",
                "base",
                definition_scalar_path(LOCAL_DEFINITION, "geometry.lathe.profile.1.x"),
                ScalarTransform::Ratio {
                    minimum: 0.42,
                    maximum: 0.96,
                },
            ),
            scalar_binding(
                "base_weight",
                "base",
                definition_scalar_path(LOCAL_DEFINITION, "geometry.lathe.profile.2.x"),
                ScalarTransform::Ratio {
                    minimum: 0.42,
                    maximum: 0.96,
                },
            ),
            scalar_binding(
                "base_weight",
                "base",
                definition_scalar_path(LOCAL_DEFINITION, "geometry.lathe.profile.3.x"),
                ScalarTransform::Ratio {
                    minimum: 0.32,
                    maximum: 0.82,
                },
            ),
            scalar_binding(
                "base_weight",
                "base",
                instance_scalar_path(LOCAL_BASE_SLAB_INSTANCE, "transform.scale.x"),
                ScalarTransform::Ratio {
                    minimum: 0.40,
                    maximum: 1.90,
                },
            ),
            scalar_binding(
                "base_weight",
                "base",
                instance_scalar_path(LOCAL_BASE_SLAB_INSTANCE, "transform.scale.z"),
                ScalarTransform::Ratio {
                    minimum: 0.42,
                    maximum: 1.25,
                },
            ),
            scalar_binding(
                "base_weight",
                "base",
                instance_scalar_path(LOCAL_BASE_SLAB_INSTANCE, "transform.scale.y"),
                ScalarTransform::Ratio {
                    minimum: 0.42,
                    maximum: 1.00,
                },
            ),
            scalar_binding(
                "stem_curvature",
                "stem",
                definition_scalar_path(LOCAL_DEFINITION, "geometry.sweep.path.1.origin.x"),
                ScalarTransform::Ratio {
                    minimum: 0.0,
                    maximum: 0.48,
                },
            ),
            scalar_binding(
                "stem_curvature",
                "stem",
                definition_scalar_path(LOCAL_DEFINITION, "geometry.sweep.path.2.origin.x"),
                ScalarTransform::Ratio {
                    minimum: 0.04,
                    maximum: 0.68,
                },
            ),
            scalar_binding(
                "stem_curvature",
                "joint",
                instance_scalar_path(LOCAL_SECOND_INSTANCE, "transform.translation.x"),
                ScalarTransform::Ratio {
                    minimum: 0.04,
                    maximum: 0.68,
                },
            ),
            scalar_binding(
                "joint_size",
                "joint",
                definition_scalar_path(LOCAL_DEFINITION, "geometry.cylinder.radius"),
                ScalarTransform::Ratio {
                    minimum: 0.055,
                    maximum: 0.16,
                },
            ),
            scalar_binding(
                "joint_size",
                "joint",
                definition_scalar_path(LOCAL_DEFINITION, "geometry.cylinder.height"),
                ScalarTransform::Ratio {
                    minimum: 0.11,
                    maximum: 0.36,
                },
            ),
            ParameterBinding::ChoiceToPrototype {
                slot: "shade_style".to_owned(),
                role: "shade".to_owned(),
                choices: BTreeMap::from([
                    ("cone".to_owned(), "ribbed_cone_shade".to_owned()),
                    ("drum".to_owned(), "banded_drum_shade".to_owned()),
                    ("task".to_owned(), "angled_task_shade".to_owned()),
                    ("wide".to_owned(), "wide_reading_shade".to_owned()),
                    ("minimal".to_owned(), "minimal_shade".to_owned()),
                    ("playful".to_owned(), "playful_tilt_shade".to_owned()),
                ]),
            },
            scalar_binding(
                "shade_scale",
                "shade",
                instance_scalar_path(LOCAL_SHADE_BODY_INSTANCE, "transform.scale.x"),
                ScalarTransform::Ratio {
                    minimum: 0.70,
                    maximum: 1.38,
                },
            ),
            scalar_binding(
                "shade_scale",
                "shade",
                instance_scalar_path(LOCAL_SHADE_BODY_INSTANCE, "transform.scale.z"),
                ScalarTransform::Ratio {
                    minimum: 0.70,
                    maximum: 1.38,
                },
            ),
            scalar_binding(
                "shade_scale",
                "shade",
                instance_scalar_path(LOCAL_SHADE_BODY_INSTANCE, "transform.scale.y"),
                ScalarTransform::Ratio {
                    minimum: 0.80,
                    maximum: 1.20,
                },
            ),
            scalar_binding(
                "edge_softness",
                "base",
                definition_scalar_path(LOCAL_BASE_SLAB_DEFINITION, "geometry.rounded_box.radius"),
                ScalarTransform::Ratio {
                    minimum: 0.006,
                    maximum: 0.08,
                },
            ),
            scalar_binding(
                "edge_softness",
                "base",
                definition_scalar_path(LOCAL_BASE_SLAB_DEFINITION, "operation.1.bevel.radius"),
                ScalarTransform::Ratio {
                    minimum: 0.0,
                    maximum: 0.03,
                },
            ),
            scalar_binding(
                "edge_softness",
                "stem",
                definition_scalar_path(LOCAL_DEFINITION, "operation.1.bevel.radius"),
                ScalarTransform::Ratio {
                    minimum: 0.0,
                    maximum: 0.015,
                },
            ),
            scalar_binding(
                "edge_softness",
                "joint",
                definition_scalar_path(LOCAL_DEFINITION, "operation.1.bevel.radius"),
                ScalarTransform::Ratio {
                    minimum: 0.0,
                    maximum: 0.035,
                },
            ),
            scalar_binding(
                "edge_softness",
                "shade",
                definition_scalar_path(LOCAL_SHADE_BODY_DEFINITION, "operation.1.bevel.radius"),
                ScalarTransform::Ratio {
                    minimum: 0.0,
                    maximum: 0.03,
                },
            ),
        ],
    );
    family_impl.attachment_bindings = lamp_attachment_bindings();

    let style_impl = style_implementation(
        "stylized_furniture",
        "lamp",
        BTreeMap::from([
            ("base".to_owned(), "lathed_weighted_base".to_owned()),
            ("stem".to_owned(), "swept_curve_stem".to_owned()),
            ("joint".to_owned(), "pivot_disc_pair".to_owned()),
            ("shade".to_owned(), "ribbed_cone_shade".to_owned()),
        ]),
        vec![
            lathed_base_fragment(),
            swept_stem_fragment(),
            joint_pair_fragment(),
            cone_shade_fragment(),
            drum_shade_fragment(),
            task_shade_fragment(),
            wide_shade_fragment(),
            minimal_shade_fragment(),
            playful_shade_fragment(),
        ],
    );
    debug_assert_eq!(
        style_impl.schema_version,
        STYLE_IMPLEMENTATION_SCHEMA_VERSION
    );
    debug_assert_eq!(
        family_impl.schema_version,
        FAMILY_IMPLEMENTATION_SCHEMA_VERSION
    );

    let mut profile = crate::customizer_profile(
        "lamp",
        "stylized_furniture",
        vec![
            continuous_control(
                "overall_height",
                "Overall Height",
                "overall_height",
                1.55,
                1.1,
                2.2,
            ),
            continuous_control("base_weight", "Base Weight", "base_weight", 0.55, 0.0, 1.0),
            continuous_control(
                "stem_curvature",
                "Stem Curvature",
                "stem_curvature",
                0.42,
                0.0,
                1.0,
            ),
            continuous_control("joint_size", "Joint Size", "joint_size", 0.5, 0.0, 1.0),
            choice_control(
                "shade_style",
                "Shade Style",
                "shade_style",
                &SHADE_STYLE_VALUES,
            ),
            continuous_control("shade_scale", "Shade Scale", "shade_scale", 0.55, 0.0, 1.0),
            continuous_control(
                "edge_softness",
                "Edge Softness",
                "edge_softness",
                0.45,
                0.0,
                1.0,
            ),
        ],
    );
    profile.candidate_strategies = lamp_candidate_strategies();

    build_fixture_catalog(FixtureCatalogSpec {
        slug: "stylized-lamp",
        document_id: "stylized-lamp-doc",
        family,
        style,
        family_implementation: family_impl,
        style_implementation: style_impl,
        customizer_profile: profile,
        control_state: BTreeMap::from([
            ("overall_height".to_owned(), ControlValue::Scalar(1.55)),
            ("base_weight".to_owned(), ControlValue::Scalar(0.55)),
            ("stem_curvature".to_owned(), ControlValue::Scalar(0.42)),
            ("joint_size".to_owned(), ControlValue::Scalar(0.5)),
            (
                "shade_style".to_owned(),
                ControlValue::Choice("cone".to_owned()),
            ),
            ("shade_scale".to_owned(), ControlValue::Scalar(0.55)),
            ("edge_softness".to_owned(), ControlValue::Scalar(0.45)),
        ]),
    })
}

/// Product-safe semantic part groups for the Stylized Furniture Lamp.
#[must_use]
pub fn part_group_descriptors() -> Vec<FoundryPartGroupDescriptor> {
    built_in_part_group_descriptors_for_profile("stylized-lamp")
}

fn lamp_attachment_rules() -> Vec<AttachmentRule> {
    vec![
        attachment_rule("stem_to_base", "stem", "base"),
        attachment_rule("joint_to_stem", "joint", "stem"),
        attachment_rule("shade_to_stem", "shade", "stem"),
    ]
}

fn attachment_rule(id: &str, from_role: &str, to_role: &str) -> AttachmentRule {
    AttachmentRule {
        id: id.to_owned(),
        from_role: from_role.to_owned(),
        to_role: to_role.to_owned(),
        anchor_role: None,
        compatibility_tags: vec!["lamp_mount".to_owned()],
        required: true,
        execution_policy: FamilyRuleExecutionPolicy::Required,
    }
}

fn lamp_attachment_bindings() -> Vec<FragmentAttachmentBinding> {
    vec![
        attachment_binding(
            "stem_to_base",
            "base",
            "stem_mount",
            "stem",
            "base_mount",
            FragmentAttachmentPairing::ByOccurrenceIndex,
            [0.0, 0.1, 0.0],
        ),
        attachment_binding(
            "joint_to_stem",
            "stem",
            "joint_mount",
            "joint",
            "stem_mount",
            FragmentAttachmentPairing::ByOccurrenceIndex,
            [0.0, 0.28, -0.28],
        ),
        attachment_binding(
            "shade_to_stem",
            "stem",
            "shade_mount",
            "shade",
            "stem_mount",
            FragmentAttachmentPairing::ByOccurrenceIndex,
            [1.05, 0.08, 0.0],
        ),
    ]
}

fn attachment_binding(
    family_attachment_rule: &str,
    parent_role: &str,
    parent_port: &str,
    child_role: &str,
    child_port: &str,
    pairing: FragmentAttachmentPairing,
    translation: [f32; 3],
) -> FragmentAttachmentBinding {
    FragmentAttachmentBinding {
        family_attachment_rule: family_attachment_rule.to_owned(),
        parent_role: parent_role.to_owned(),
        parent_port: parent_port.to_owned(),
        child_role: child_role.to_owned(),
        child_port: child_port.to_owned(),
        pairing,
        rigid_offset: RigidOffset {
            translation,
            ..RigidOffset::default()
        },
        attachment_mode: AttachmentMode::RigidSeparate,
    }
}

fn tag_lamp_prototype_operations(style: &mut shape_family::StyleKit) {
    let Some(facet) = style.family_facets.get_mut("lamp") else {
        return;
    };
    for prototype in &mut facet.part_prototypes {
        prototype.operation_tags = match prototype.id.as_str() {
            "lathed_weighted_base" => vec![
                AllowedOperationKind::Primitive,
                AllowedOperationKind::Lathe,
                AllowedOperationKind::Bevel,
            ],
            "swept_curve_stem" => vec![AllowedOperationKind::Sweep, AllowedOperationKind::Bevel],
            "pivot_disc_pair" => vec![AllowedOperationKind::Primitive, AllowedOperationKind::Bevel],
            "angled_task_shade" => vec![
                AllowedOperationKind::Primitive,
                AllowedOperationKind::Sweep,
                AllowedOperationKind::Bevel,
                AllowedOperationKind::Transform,
            ],
            "playful_tilt_shade" => vec![
                AllowedOperationKind::Primitive,
                AllowedOperationKind::Sweep,
                AllowedOperationKind::Bevel,
                AllowedOperationKind::Transform,
            ],
            "ribbed_cone_shade" | "banded_drum_shade" | "wide_reading_shade" | "minimal_shade" => {
                vec![AllowedOperationKind::Primitive, AllowedOperationKind::Bevel]
            }
            _ => prototype.operation_tags.clone(),
        };
    }
}

fn scalar_binding(
    slot: &str,
    role: &str,
    local_path: String,
    transform: ScalarTransform,
) -> ParameterBinding {
    ParameterBinding::Scalar {
        slot: slot.to_owned(),
        role: role.to_owned(),
        local_path,
        transform,
    }
}

fn lamp_candidate_strategies() -> Vec<CandidateStrategy> {
    vec![
        CandidateStrategy {
            id: "compact_task_lamp".to_owned(),
            label: "Compact Task Lamp".to_owned(),
            control_ids: ids(&[
                "overall_height",
                "base_weight",
                "stem_curvature",
                "shade_style",
                "shade_scale",
            ]),
        },
        CandidateStrategy {
            id: "tall_reading_lamp".to_owned(),
            label: "Tall Reading Lamp".to_owned(),
            control_ids: ids(&[
                "overall_height",
                "base_weight",
                "stem_curvature",
                "joint_size",
                "shade_style",
                "shade_scale",
            ]),
        },
        CandidateStrategy {
            id: "playful_curved_lamp".to_owned(),
            label: "Playful Curved Lamp".to_owned(),
            control_ids: ids(&[
                "overall_height",
                "base_weight",
                "stem_curvature",
                "joint_size",
                "shade_style",
                "shade_scale",
                "edge_softness",
            ]),
        },
        CandidateStrategy {
            id: "heavy_base".to_owned(),
            label: "Heavy Base Lamp".to_owned(),
            control_ids: ids(&[
                "overall_height",
                "base_weight",
                "stem_curvature",
                "joint_size",
                "shade_style",
                "shade_scale",
                "edge_softness",
            ]),
        },
        CandidateStrategy {
            id: "minimal_studio_lamp".to_owned(),
            label: "Minimal Studio Lamp".to_owned(),
            control_ids: ids(&[
                "overall_height",
                "base_weight",
                "stem_curvature",
                "shade_style",
                "shade_scale",
                "edge_softness",
            ]),
        },
        CandidateStrategy {
            id: "wide_shade_lamp".to_owned(),
            label: "Wide Shade Lamp".to_owned(),
            control_ids: ids(&[
                "overall_height",
                "base_weight",
                "stem_curvature",
                "shade_style",
                "shade_scale",
            ]),
        },
    ]
}

fn ids(values: &[&str]) -> Vec<String> {
    values.iter().map(|value| (*value).to_owned()).collect()
}

fn lathed_base_fragment() -> RecipeFragment {
    let source = GeometrySource::Lathe {
        profile: vec![
            [0.0, -0.12],
            [0.68, -0.12],
            [0.68, -0.06],
            [0.54, 0.04],
            [0.34, 0.16],
            [0.14, 0.23],
            [0.0, 0.23],
        ],
        segments: 48,
    };
    let mut recipe = single_definition_recipe(
        "lathed_weighted_base",
        "base",
        source,
        Vec::new(),
        [0.0, 0.0, 0.0],
        vec![socket("stem mount", SOCKET_PRIMARY, [0.0, 0.23, 0.0])],
    );
    recipe.definitions.insert(
        LOCAL_BASE_SLAB_DEFINITION,
        part_definition(
            LOCAL_BASE_SLAB_DEFINITION,
            "weighted slab foot definition",
            "base",
            GeometrySource::RoundedBox {
                half_extents: [0.56, 0.045, 0.34],
                radius: 0.035,
            },
            vec![bevel(OPERATION_BEVEL, 0.012, 1)],
            Vec::new(),
        ),
    );
    recipe.instances.insert(
        LOCAL_BASE_SLAB_INSTANCE,
        part_instance(
            LOCAL_BASE_SLAB_INSTANCE,
            LOCAL_BASE_SLAB_DEFINITION,
            "weighted slab foot",
            Some(LOCAL_INSTANCE),
            Transform3 {
                translation: [0.0, -0.18, 0.0],
                ..Transform3::default()
            },
        ),
    );
    add_parameters(
        &mut recipe,
        &[
            (
                definition_scalar_path(LOCAL_DEFINITION, "geometry.lathe.profile.1.x"),
                "Base outer foot radius",
                0.42,
                1.0,
                0.01,
            ),
            (
                definition_scalar_path(LOCAL_DEFINITION, "geometry.lathe.profile.2.x"),
                "Base lower shoulder radius",
                0.42,
                1.0,
                0.01,
            ),
            (
                definition_scalar_path(LOCAL_DEFINITION, "geometry.lathe.profile.3.x"),
                "Base upper shoulder radius",
                0.32,
                0.82,
                0.01,
            ),
            (
                instance_scalar_path(LOCAL_INSTANCE, "transform.scale.x"),
                "Base X weight scale",
                0.55,
                2.0,
                0.01,
            ),
            (
                instance_scalar_path(LOCAL_INSTANCE, "transform.scale.z"),
                "Base Z weight scale",
                0.55,
                2.0,
                0.01,
            ),
            (
                instance_scalar_path(LOCAL_INSTANCE, "transform.scale.y"),
                "Base height weight scale",
                0.7,
                1.5,
                0.01,
            ),
            (
                instance_scalar_path(LOCAL_BASE_SLAB_INSTANCE, "transform.scale.x"),
                "Slab foot width scale",
                0.35,
                2.0,
                0.01,
            ),
            (
                instance_scalar_path(LOCAL_BASE_SLAB_INSTANCE, "transform.scale.z"),
                "Slab foot depth scale",
                0.35,
                1.3,
                0.01,
            ),
            (
                instance_scalar_path(LOCAL_BASE_SLAB_INSTANCE, "transform.scale.y"),
                "Slab foot thickness scale",
                0.35,
                1.4,
                0.01,
            ),
            (
                definition_scalar_path(LOCAL_BASE_SLAB_DEFINITION, "geometry.rounded_box.radius"),
                "Slab corner softness",
                0.005,
                0.08,
                0.001,
            ),
            (
                definition_scalar_path(LOCAL_BASE_SLAB_DEFINITION, "operation.1.bevel.radius"),
                "Slab edge bevel",
                0.0,
                0.035,
                0.001,
            ),
        ],
    );
    fragment(
        "lathed_weighted_base",
        "base",
        recipe,
        vec![LOCAL_INSTANCE],
        vec![socket_port("stem_mount", LOCAL_INSTANCE, SOCKET_PRIMARY)],
    )
}

fn swept_stem_fragment() -> RecipeFragment {
    let source = GeometrySource::Sweep {
        profile: circle_profile(0.048),
        path: vec![
            frame([0.0, 0.0, 0.0]),
            frame([0.18, 0.65, 0.0]),
            frame([0.30, 1.25, 0.0]),
        ],
    };
    let mut recipe = single_definition_recipe(
        "swept_curve_stem",
        "stem",
        source,
        vec![bevel(OPERATION_BEVEL, 0.006, 1)],
        [0.0, 0.26, 0.0],
        vec![
            socket("base mount", SOCKET_PRIMARY, [0.0, 0.0, 0.0]),
            socket("shade mount", SOCKET_SECONDARY, [0.30, 1.25, 0.0]),
            socket("joint mount", SOCKET_TERTIARY, [0.0, 0.0, -0.16]),
        ],
    );
    add_parameters(
        &mut recipe,
        &[
            (
                definition_scalar_path(LOCAL_DEFINITION, "geometry.sweep.path.1.origin.y"),
                "Stem midpoint height",
                0.42,
                1.12,
                0.01,
            ),
            (
                definition_scalar_path(LOCAL_DEFINITION, "geometry.sweep.path.2.origin.y"),
                "Stem endpoint height",
                0.84,
                2.05,
                0.01,
            ),
            (
                definition_scalar_path(LOCAL_DEFINITION, "geometry.sweep.path.1.origin.x"),
                "Stem midpoint curve",
                0.0,
                0.48,
                0.01,
            ),
            (
                definition_scalar_path(LOCAL_DEFINITION, "geometry.sweep.path.2.origin.x"),
                "Stem endpoint curve",
                0.04,
                0.68,
                0.01,
            ),
            (
                definition_scalar_path(LOCAL_DEFINITION, "operation.1.bevel.radius"),
                "Stem edge softness",
                0.0,
                0.015,
                0.001,
            ),
        ],
    );
    fragment(
        "swept_curve_stem",
        "stem",
        recipe,
        vec![LOCAL_INSTANCE],
        vec![
            socket_port("base_mount", LOCAL_INSTANCE, SOCKET_PRIMARY),
            socket_port("shade_mount", LOCAL_INSTANCE, SOCKET_SECONDARY),
            socket_port("joint_mount", LOCAL_INSTANCE, SOCKET_TERTIARY),
        ],
    )
}

fn joint_pair_fragment() -> RecipeFragment {
    let mut recipe = AssetRecipe::new(AssetId(1), "pivot_disc_pair fragment");
    recipe.definitions.insert(
        LOCAL_DEFINITION,
        part_definition(
            LOCAL_DEFINITION,
            "pivot disc joint definition",
            "joint",
            GeometrySource::Cylinder {
                radius: 0.12,
                height: 0.22,
                radial_segments: 28,
            },
            vec![bevel(OPERATION_BEVEL, 0.016, 1)],
            vec![socket("stem mount", SOCKET_PRIMARY, [0.0, 0.0, 0.0])],
        ),
    );
    recipe.instances.insert(
        LOCAL_INSTANCE,
        part_instance(
            LOCAL_INSTANCE,
            LOCAL_DEFINITION,
            "lower pivot disc joint",
            None,
            Transform3 {
                translation: [0.0, 0.0, -0.16],
                rotation_degrees: [0.0, 0.0, 90.0],
                ..Transform3::default()
            },
        ),
    );
    recipe.instances.insert(
        LOCAL_SECOND_INSTANCE,
        part_instance(
            LOCAL_SECOND_INSTANCE,
            LOCAL_DEFINITION,
            "upper pivot disc joint",
            Some(LOCAL_INSTANCE),
            Transform3 {
                translation: [0.30, 1.25, -0.16],
                rotation_degrees: [0.0, 0.0, 90.0],
                ..Transform3::default()
            },
        ),
    );
    recipe.root_instances = vec![LOCAL_INSTANCE];
    add_parameters(
        &mut recipe,
        &[
            (
                definition_scalar_path(LOCAL_DEFINITION, "geometry.cylinder.radius"),
                "Joint radius",
                0.05,
                0.16,
                0.005,
            ),
            (
                definition_scalar_path(LOCAL_DEFINITION, "geometry.cylinder.height"),
                "Joint barrel width",
                0.10,
                0.36,
                0.005,
            ),
            (
                definition_scalar_path(LOCAL_DEFINITION, "operation.1.bevel.radius"),
                "Joint edge softness",
                0.0,
                0.035,
                0.001,
            ),
            (
                instance_scalar_path(LOCAL_SECOND_INSTANCE, "transform.translation.y"),
                "Upper joint height",
                0.84,
                2.05,
                0.01,
            ),
            (
                instance_scalar_path(LOCAL_SECOND_INSTANCE, "transform.translation.x"),
                "Upper joint curve",
                0.04,
                0.68,
                0.01,
            ),
        ],
    );
    finish_recipe_ids(&mut recipe);
    fragment(
        "pivot_disc_pair",
        "joint",
        recipe,
        vec![LOCAL_INSTANCE],
        vec![socket_port("stem_mount", LOCAL_INSTANCE, SOCKET_PRIMARY)],
    )
}

fn cone_shade_fragment() -> RecipeFragment {
    let mut recipe = shade_recipe(
        "ribbed_cone_shade",
        GeometrySource::Frustum {
            bottom_radius: 0.44,
            top_radius: 0.18,
            height: 0.44,
            radial_segments: 48,
        },
        Transform3 {
            rotation_degrees: [0.0, 0.0, -18.0],
            ..Transform3::default()
        },
    );
    add_trim_ring(&mut recipe, 0.48, 0.25, -0.255);
    shade_fragment("ribbed_cone_shade", recipe)
}

fn drum_shade_fragment() -> RecipeFragment {
    let mut recipe = shade_recipe(
        "banded_drum_shade",
        GeometrySource::Frustum {
            bottom_radius: 0.36,
            top_radius: 0.36,
            height: 0.50,
            radial_segments: 48,
        },
        Transform3 {
            rotation_degrees: [0.0, 0.0, -8.0],
            ..Transform3::default()
        },
    );
    add_trim_ring(&mut recipe, 0.38, 0.38, -0.285);
    add_trim_ring_instance(&mut recipe, LOCAL_UPPER_TRIM_INSTANCE, [0.0, 0.285, 0.0]);
    shade_fragment("banded_drum_shade", recipe)
}

fn task_shade_fragment() -> RecipeFragment {
    let mut recipe = shade_recipe(
        "angled_task_shade",
        GeometrySource::Frustum {
            bottom_radius: 0.34,
            top_radius: 0.16,
            height: 0.34,
            radial_segments: 40,
        },
        Transform3 {
            rotation_degrees: [0.0, 0.0, -28.0],
            ..Transform3::default()
        },
    );
    add_trim_ring(&mut recipe, 0.35, 0.28, -0.255);
    add_bracket(&mut recipe);
    shade_fragment("angled_task_shade", recipe)
}

fn wide_shade_fragment() -> RecipeFragment {
    let mut recipe = shade_recipe(
        "wide_reading_shade",
        GeometrySource::Frustum {
            bottom_radius: 0.62,
            top_radius: 0.40,
            height: 0.30,
            radial_segments: 56,
        },
        Transform3 {
            rotation_degrees: [0.0, 0.0, -10.0],
            ..Transform3::default()
        },
    );
    add_trim_ring(&mut recipe, 0.64, 0.42, -0.185);
    add_trim_ring_instance(&mut recipe, LOCAL_UPPER_TRIM_INSTANCE, [0.0, 0.185, 0.0]);
    shade_fragment("wide_reading_shade", recipe)
}

fn minimal_shade_fragment() -> RecipeFragment {
    let recipe = shade_recipe(
        "minimal_shade",
        GeometrySource::Frustum {
            bottom_radius: 0.30,
            top_radius: 0.23,
            height: 0.30,
            radial_segments: 32,
        },
        Transform3 {
            rotation_degrees: [0.0, 0.0, -12.0],
            ..Transform3::default()
        },
    );
    shade_fragment("minimal_shade", recipe)
}

fn playful_shade_fragment() -> RecipeFragment {
    let mut recipe = shade_recipe(
        "playful_tilt_shade",
        GeometrySource::Frustum {
            bottom_radius: 0.46,
            top_radius: 0.22,
            height: 0.38,
            radial_segments: 36,
        },
        Transform3 {
            rotation_degrees: [0.0, 0.0, -34.0],
            ..Transform3::default()
        },
    );
    add_trim_ring(&mut recipe, 0.50, 0.26, -0.235);
    add_trim_ring_instance(&mut recipe, LOCAL_UPPER_TRIM_INSTANCE, [0.0, 0.215, 0.0]);
    add_bracket(&mut recipe);
    shade_fragment("playful_tilt_shade", recipe)
}

fn shade_recipe(id: &str, source: GeometrySource, transform: Transform3) -> AssetRecipe {
    let mut recipe = AssetRecipe::new(AssetId(1), format!("{id} fragment"));
    recipe.definitions.insert(
        LOCAL_DEFINITION,
        part_definition(
            LOCAL_DEFINITION,
            &format!("{id} mount collar definition"),
            "shade",
            GeometrySource::Cylinder {
                radius: 0.055,
                height: 0.10,
                radial_segments: 24,
            },
            vec![bevel(OPERATION_SHADE_MOUNT_BEVEL, 0.006, 1)],
            vec![socket("stem mount", SOCKET_PRIMARY, [0.0, 0.0, 0.0])],
        ),
    );
    recipe.definitions.insert(
        LOCAL_SHADE_BODY_DEFINITION,
        part_definition(
            LOCAL_SHADE_BODY_DEFINITION,
            &format!("{id} body definition"),
            "shade",
            source,
            vec![bevel(OPERATION_BEVEL, 0.012, 1)],
            Vec::new(),
        ),
    );
    recipe.instances.insert(
        LOCAL_INSTANCE,
        part_instance(
            LOCAL_INSTANCE,
            LOCAL_DEFINITION,
            &format!("{id} shade mount collar"),
            None,
            Transform3::default(),
        ),
    );
    recipe.instances.insert(
        LOCAL_SHADE_BODY_INSTANCE,
        part_instance(
            LOCAL_SHADE_BODY_INSTANCE,
            LOCAL_SHADE_BODY_DEFINITION,
            &format!("{id} shade body"),
            Some(LOCAL_INSTANCE),
            transform,
        ),
    );
    recipe.root_instances.push(LOCAL_INSTANCE);
    add_parameters(
        &mut recipe,
        &[
            (
                definition_scalar_path(
                    LOCAL_SHADE_BODY_DEFINITION,
                    "geometry.frustum.bottom_radius",
                ),
                "Shade lower radius",
                0.22,
                0.72,
                0.01,
            ),
            (
                definition_scalar_path(LOCAL_SHADE_BODY_DEFINITION, "geometry.frustum.top_radius"),
                "Shade upper radius",
                0.16,
                0.45,
                0.01,
            ),
            (
                definition_scalar_path(LOCAL_SHADE_BODY_DEFINITION, "geometry.frustum.height"),
                "Shade height",
                0.24,
                0.62,
                0.01,
            ),
            (
                instance_scalar_path(LOCAL_SHADE_BODY_INSTANCE, "transform.scale.x"),
                "Shade width scale",
                0.7,
                1.6,
                0.01,
            ),
            (
                instance_scalar_path(LOCAL_SHADE_BODY_INSTANCE, "transform.scale.z"),
                "Shade depth scale",
                0.7,
                1.6,
                0.01,
            ),
            (
                instance_scalar_path(LOCAL_SHADE_BODY_INSTANCE, "transform.scale.y"),
                "Shade height scale",
                0.8,
                1.35,
                0.01,
            ),
            (
                definition_scalar_path(LOCAL_SHADE_BODY_DEFINITION, "operation.1.bevel.radius"),
                "Shade edge softness",
                0.0,
                0.03,
                0.001,
            ),
        ],
    );
    recipe
}

fn add_trim_ring(recipe: &mut AssetRecipe, bottom_radius: f32, top_radius: f32, y: f32) {
    recipe.definitions.insert(
        LOCAL_TRIM_DEFINITION,
        part_definition(
            LOCAL_TRIM_DEFINITION,
            "shade trim ring definition",
            "shade",
            GeometrySource::Frustum {
                bottom_radius,
                top_radius,
                height: 0.035,
                radial_segments: 48,
            },
            vec![bevel(OPERATION_TRIM_BEVEL, 0.006, 1)],
            Vec::new(),
        ),
    );
    recipe.instances.insert(
        LOCAL_TRIM_INSTANCE,
        part_instance(
            LOCAL_TRIM_INSTANCE,
            LOCAL_TRIM_DEFINITION,
            "shade lower trim ring",
            Some(LOCAL_SHADE_BODY_INSTANCE),
            Transform3 {
                translation: [0.0, y, 0.0],
                ..Transform3::default()
            },
        ),
    );
}

fn add_trim_ring_instance(
    recipe: &mut AssetRecipe,
    instance: PartInstanceId,
    translation: [f32; 3],
) {
    recipe.instances.insert(
        instance,
        part_instance(
            instance,
            LOCAL_TRIM_DEFINITION,
            "shade upper trim ring",
            Some(LOCAL_SHADE_BODY_INSTANCE),
            Transform3 {
                translation,
                ..Transform3::default()
            },
        ),
    );
}

fn add_bracket(recipe: &mut AssetRecipe) {
    recipe.definitions.insert(
        LOCAL_BRACKET_DEFINITION,
        part_definition(
            LOCAL_BRACKET_DEFINITION,
            "curved shade bracket definition",
            "shade",
            GeometrySource::Sweep {
                profile: circle_profile(0.026),
                path: vec![
                    frame([-0.23, -0.02, 0.0]),
                    frame([-0.15, -0.12, 0.0]),
                    frame([0.05, -0.18, 0.0]),
                ],
            },
            vec![bevel(OPERATION_BRACKET_BEVEL, 0.003, 1)],
            Vec::new(),
        ),
    );
    recipe.instances.insert(
        LOCAL_BRACKET_INSTANCE,
        part_instance(
            LOCAL_BRACKET_INSTANCE,
            LOCAL_BRACKET_DEFINITION,
            "curved shade support bracket",
            Some(LOCAL_SHADE_BODY_INSTANCE),
            Transform3 {
                translation: [0.0, 0.0, 0.49],
                ..Transform3::default()
            },
        ),
    );
}

fn shade_fragment(id: &str, mut recipe: AssetRecipe) -> RecipeFragment {
    finish_recipe_ids(&mut recipe);
    fragment(
        id,
        "shade",
        recipe,
        vec![LOCAL_INSTANCE],
        vec![socket_port("stem_mount", LOCAL_INSTANCE, SOCKET_PRIMARY)],
    )
}

fn single_definition_recipe(
    id: &str,
    role: &str,
    source: GeometrySource,
    operations: Vec<ModelingOperationSpec>,
    translation: [f32; 3],
    sockets: Vec<SocketSpec>,
) -> AssetRecipe {
    let mut recipe = AssetRecipe::new(AssetId(1), format!("{id} fragment"));
    recipe.definitions.insert(
        LOCAL_DEFINITION,
        part_definition(
            LOCAL_DEFINITION,
            &format!("{id} definition"),
            role,
            source,
            operations,
            sockets,
        ),
    );
    recipe.instances.insert(
        LOCAL_INSTANCE,
        part_instance(
            LOCAL_INSTANCE,
            LOCAL_DEFINITION,
            &format!("{id} {role}"),
            None,
            Transform3 {
                translation,
                ..Transform3::default()
            },
        ),
    );
    recipe.root_instances.push(LOCAL_INSTANCE);
    finish_recipe_ids(&mut recipe);
    recipe
}

fn part_definition(
    definition: PartDefinitionId,
    name: &str,
    role: &str,
    source: GeometrySource,
    operations: Vec<ModelingOperationSpec>,
    sockets: Vec<SocketSpec>,
) -> PartDefinition {
    PartDefinition {
        id: definition,
        name: name.to_owned(),
        tags: BTreeSet::from([role.to_owned(), format!("role:{role}")]),
        geometry: GeometryRecipe { source, operations },
        regions: BTreeMap::new(),
        sockets: sockets
            .into_iter()
            .map(|socket| (socket.id, socket))
            .collect(),
        local_pivot: Frame3::default(),
        variant_group: None,
        production_hints: None,
    }
}

fn part_instance(
    id: PartInstanceId,
    definition: PartDefinitionId,
    name: &str,
    parent: Option<PartInstanceId>,
    local_transform: Transform3,
) -> PartInstance {
    PartInstance {
        id,
        definition,
        name: name.to_owned(),
        parent,
        local_transform,
        attachment: None,
        enabled: true,
        tags: BTreeSet::from(["lamp".to_owned()]),
        generated_by: None,
    }
}

fn socket(name: &str, id: SocketId, origin: [f32; 3]) -> SocketSpec {
    SocketSpec {
        id,
        name: name.to_owned(),
        local_frame: Frame3 {
            origin,
            ..Frame3::default()
        },
        role: "lamp_mount".to_owned(),
        tags: BTreeSet::from(["lamp_mount".to_owned(), "socket".to_owned()]),
    }
}

fn socket_port(
    id: &str,
    local_occurrence_root: PartInstanceId,
    local_socket: SocketId,
) -> FragmentSocketPort {
    FragmentSocketPort {
        id: id.to_owned(),
        local_occurrence_root,
        local_socket,
        compatibility_tags: vec!["lamp_mount".to_owned()],
    }
}

fn fragment(
    id: &str,
    role: &str,
    recipe: AssetRecipe,
    role_occurrence_roots: Vec<PartInstanceId>,
    socket_ports: Vec<FragmentSocketPort>,
) -> RecipeFragment {
    let validation = validate_asset_recipe(&recipe);
    assert!(
        validation.is_valid(),
        "{id} fragment recipe should be valid: {:#?}",
        validation.issues
    );
    RecipeFragment {
        schema_version: RECIPE_FRAGMENT_SCHEMA_VERSION,
        id: id.to_owned(),
        provided_role: role.to_owned(),
        exports: RecipeFragmentExports {
            role_occurrence_roots,
            internal_roots: Vec::new(),
            socket_ports,
            surface_ports: Vec::new(),
        },
        recipe,
    }
}

fn bevel(operation: OperationId, radius: f32, segments: u32) -> ModelingOperationSpec {
    ModelingOperationSpec::SetBevelProfile {
        operation,
        radius,
        segments,
    }
}

fn circle_profile(radius: f32) -> Vec<[f32; 2]> {
    let diagonal = radius * std::f32::consts::FRAC_1_SQRT_2;
    vec![
        [radius, 0.0],
        [diagonal, diagonal],
        [0.0, radius],
        [-diagonal, diagonal],
        [-radius, 0.0],
        [-diagonal, -diagonal],
        [0.0, -radius],
        [diagonal, -diagonal],
    ]
}

fn frame(origin: [f32; 3]) -> Frame3 {
    Frame3 {
        origin,
        ..Frame3::default()
    }
}

fn add_parameters(recipe: &mut AssetRecipe, parameters: &[(String, &str, f32, f32, f32)]) {
    for (index, (path, label, minimum, maximum, step)) in parameters.iter().enumerate() {
        let id = ParameterId((index + 1) as u64);
        recipe.parameters.insert(
            id,
            scalar_parameter(
                id.0,
                path.clone(),
                (*label).to_owned(),
                *minimum,
                *maximum,
                *step,
                false,
            ),
        );
    }
    finish_recipe_ids(recipe);
}

fn finish_recipe_ids(recipe: &mut AssetRecipe) {
    recipe.next_ids.part_definition = next_definition_id(recipe);
    recipe.next_ids.part_instance = next_instance_id(recipe);
    recipe.next_ids.parameter = next_parameter_id(recipe);
    recipe.next_ids.operation = next_operation_id(recipe);
    recipe.next_ids.region = next_region_id(recipe);
    recipe.next_ids.socket = next_socket_id(recipe);
}

fn next_definition_id(recipe: &AssetRecipe) -> u64 {
    recipe.definitions.keys().map(|id| id.0).max().unwrap_or(0) + 1
}

fn next_instance_id(recipe: &AssetRecipe) -> u64 {
    recipe.instances.keys().map(|id| id.0).max().unwrap_or(0) + 1
}

fn next_parameter_id(recipe: &AssetRecipe) -> u64 {
    recipe.parameters.keys().map(|id| id.0).max().unwrap_or(0) + 1
}

fn next_operation_id(recipe: &AssetRecipe) -> u64 {
    recipe
        .definitions
        .values()
        .flat_map(|definition| definition.geometry.operations.iter())
        .map(ModelingOperationSpec::operation_id)
        .map(|id| id.0)
        .max()
        .unwrap_or(0)
        + 1
}

fn next_region_id(recipe: &AssetRecipe) -> u64 {
    recipe
        .definitions
        .values()
        .flat_map(|definition| definition.regions.keys())
        .map(|id| id.0)
        .max()
        .unwrap_or(0)
        + 1
}

fn next_socket_id(recipe: &AssetRecipe) -> u64 {
    recipe
        .definitions
        .values()
        .flat_map(|definition| definition.sockets.keys())
        .map(|id| id.0)
        .max()
        .unwrap_or(0)
        + 1
}
