//! Panel with Knob composition prototype fixture.

use std::collections::BTreeMap;

use shape_asset::{
    PartDefinitionId, PartInstanceId, PositionRule, definition_scalar_path, instance_scalar_path,
};
use shape_family::{AllowedOperationKind, RoleMultiplicity};
use shape_family_compile::{ParameterBinding, RecipeFragment, ScalarTransform};
use shape_foundry::{
    ControlValue, PrimitiveAttachment, PrimitiveAttachmentOffsetPolicy,
    PrimitiveAttachmentOrientationPolicy, PrimitiveAttachmentScalePolicy,
    PrimitiveCompositionDocument, PrimitiveKind, PrimitiveNode, PrimitiveNodeVisibility,
    PrimitivePropertyValue,
};

use crate::{
    CatalogCurationMetadata, FamilySchemaSpec, FixtureCatalogSpec, FoundryFixtureCatalog,
    StarterTemplateQualityEvidence, build_fixture_catalog, continuous_control,
    family_implementation, family_schema, lathe_fragment, length_slot, role, rounded_box_fragment,
    starter_template_curation_state_from_quality, style_implementation, style_kit,
};

/// Panel with Knob composition prototype slug.
pub const PANEL_KNOB_SLUG: &str = "panel-with-knob";
/// Panel with Knob family ID.
pub const PANEL_KNOB_FAMILY_ID: &str = "panel_with_knob";
/// Neutral clay style ID for Panel with Knob.
pub const PANEL_KNOB_STYLE_ID: &str = "panel_with_knob_clay";

const PANEL_BODY_ROLE: &str = "panel_body";
const KNOB_FORM_ROLE: &str = "knob_form";
const PANEL_BODY_PROVIDER: &str = "panel_knob_panel_body";
const KNOB_FORM_PROVIDER: &str = "panel_knob_knob_form";

/// Quality evidence used to gate novice catalog exposure for Panel with Knob.
#[must_use]
pub const fn quality_evidence() -> StarterTemplateQualityEvidence {
    StarterTemplateQualityEvidence {
        profile_slug: PANEL_KNOB_SLUG,
        visible_idea_count: 4,
        distinct_visible_idea_count: 4,
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

/// Curation metadata for Panel with Knob.
#[must_use]
pub fn curation_metadata() -> CatalogCurationMetadata {
    CatalogCurationMetadata {
        profile_slug: PANEL_KNOB_SLUG,
        state: starter_template_curation_state_from_quality(quality_evidence()),
        has_visual_direction_evidence: true,
        has_readable_control_evidence: true,
        has_human_showcase_review: false,
        note: "Panel with Knob is the first safe-anchor composition proof: one Flat Panel plus one knob-like Sphere form, with no Door, motion, material, rigging, or animation claim.",
    }
}

/// Return the validated composition document represented by this fixture.
#[must_use]
pub fn composition_document() -> PrimitiveCompositionDocument {
    PrimitiveCompositionDocument {
        schema_version: shape_foundry::PRIMITIVE_COMPOSITION_SCHEMA_VERSION,
        document_id: "panel_with_knob".to_owned(),
        nodes: vec![
            PrimitiveNode {
                node_id: "panel".to_owned(),
                primitive_kind: PrimitiveKind::FlatPanelPrimitive,
                property_values: BTreeMap::from([
                    (
                        "width".to_owned(),
                        PrimitivePropertyValue::Length(DEFAULT_PANEL_WIDTH),
                    ),
                    (
                        "height".to_owned(),
                        PrimitivePropertyValue::Length(DEFAULT_PANEL_HEIGHT),
                    ),
                    (
                        "thickness".to_owned(),
                        PrimitivePropertyValue::Length(DEFAULT_PANEL_THICKNESS),
                    ),
                    (
                        "edge_softness".to_owned(),
                        PrimitivePropertyValue::Ratio(DEFAULT_PANEL_EDGE_SOFTNESS),
                    ),
                ]),
                local_label: "Panel".to_owned(),
                visibility: PrimitiveNodeVisibility::Visible,
            },
            PrimitiveNode {
                node_id: "knob".to_owned(),
                primitive_kind: PrimitiveKind::SpherePrimitive,
                property_values: BTreeMap::from([
                    (
                        "width".to_owned(),
                        PrimitivePropertyValue::Length(DEFAULT_KNOB_WIDTH),
                    ),
                    (
                        "height".to_owned(),
                        PrimitivePropertyValue::Length(DEFAULT_KNOB_HEIGHT),
                    ),
                    (
                        "depth".to_owned(),
                        PrimitivePropertyValue::Length(DEFAULT_KNOB_DEPTH),
                    ),
                    (
                        "front_flatten".to_owned(),
                        PrimitivePropertyValue::Ratio(DEFAULT_KNOB_FRONT_FLATTEN),
                    ),
                    (
                        "back_flatten".to_owned(),
                        PrimitivePropertyValue::Ratio(DEFAULT_KNOB_BACK_FLATTEN),
                    ),
                ]),
                local_label: "Knob-like form".to_owned(),
                visibility: PrimitiveNodeVisibility::Visible,
            },
        ],
        attachments: vec![PrimitiveAttachment {
            attachment_id: "panel_knob_attachment".to_owned(),
            parent_node_id: "panel".to_owned(),
            parent_anchor_id: "front_handle_zone".to_owned(),
            child_node_id: "knob".to_owned(),
            child_anchor_id: "back_mount_point".to_owned(),
            offset_policy: PrimitiveAttachmentOffsetPolicy::BoundedNormalized {
                x: DEFAULT_KNOB_X_OFFSET,
                y: DEFAULT_KNOB_Y_OFFSET,
                minimum_x: KNOB_X_OFFSET_MIN,
                maximum_x: KNOB_X_OFFSET_MAX,
                minimum_y: KNOB_Y_OFFSET_MIN,
                maximum_y: KNOB_Y_OFFSET_MAX,
            },
            orientation_policy: PrimitiveAttachmentOrientationPolicy::AlignChildToParentNormal,
            scale_policy: PrimitiveAttachmentScalePolicy::KeepChildScale,
        }],
        root_node_id: "panel".to_owned(),
    }
}

/// Resolve a relationship placement policy to a horizontal panel-space position.
///
/// This is a policy proof helper for the Panel with Knob migration. It does
/// not introduce raw transforms or a product-facing free-placement lane.
#[must_use]
pub fn relationship_horizontal_position(
    position_rule: &PositionRule,
    panel_width: f32,
) -> Option<f32> {
    if !panel_width.is_finite() || panel_width <= 0.0 {
        return None;
    }
    match position_rule {
        PositionRule::FixedOffsetFromEdge { edge, offset } if edge == "right" => {
            Some(panel_width * 0.5 - offset[0])
        }
        PositionRule::FixedOffsetFromEdge { edge, offset } if edge == "left" => {
            Some(-panel_width * 0.5 + offset[0])
        }
        PositionRule::ProportionalUv { u, .. } => Some((*u - 0.5) * panel_width),
        PositionRule::CenteredInZone { .. } | PositionRule::PreserveCurrentOnDetach => Some(0.0),
        PositionRule::FixedOffsetFromEdge { .. } => None,
    }
}

/// Build the Panel with Knob fixture catalog.
#[must_use]
pub fn fixture_catalog() -> FoundryFixtureCatalog {
    let family = family_schema(FamilySchemaSpec {
        id: PANEL_KNOB_FAMILY_ID,
        display_name: "Panel with Knob",
        summary: "One upright clay panel with a bounded knob-like sphere form attached through a safe anchor.",
        roles: vec![
            role(PANEL_BODY_ROLE, RoleMultiplicity::Single, true),
            role(KNOB_FORM_ROLE, RoleMultiplicity::Single, true),
        ],
        allowed_operations: vec![
            AllowedOperationKind::Primitive,
            AllowedOperationKind::Array,
            AllowedOperationKind::Lathe,
            AllowedOperationKind::Transform,
            AllowedOperationKind::Bevel,
        ],
        parameter_slots: parameter_slots(),
        compatible_style_kits: vec![PANEL_KNOB_STYLE_ID.to_owned()],
        tags: vec![
            "panel-with-knob".to_owned(),
            "primitive-composition".to_owned(),
            "clay".to_owned(),
        ],
    });

    let style = style_kit(
        PANEL_KNOB_STYLE_ID,
        "Panel with Knob Clay",
        PANEL_KNOB_FAMILY_ID,
        &style_prototypes(),
        vec![
            "panel-with-knob".to_owned(),
            "primitive-composition".to_owned(),
            "clay".to_owned(),
        ],
    );

    let family_impl = family_implementation(
        PANEL_KNOB_FAMILY_ID,
        "Panel with Knob family",
        parameter_bindings(),
    );

    let style_impl = style_implementation(
        PANEL_KNOB_STYLE_ID,
        PANEL_KNOB_FAMILY_ID,
        default_provider_map(),
        recipe_fragments(),
    );

    let mut profile =
        crate::customizer_profile(PANEL_KNOB_FAMILY_ID, PANEL_KNOB_STYLE_ID, controls());
    profile.candidate_strategies = Vec::new();

    build_fixture_catalog(FixtureCatalogSpec {
        slug: PANEL_KNOB_SLUG,
        document_id: "panel-with-knob-doc",
        family,
        style,
        family_implementation: family_impl,
        style_implementation: style_impl,
        customizer_profile: profile,
        control_state: default_control_state(),
    })
}

const DEFAULT_PANEL_WIDTH: f32 = 1.8;
const DEFAULT_PANEL_HEIGHT: f32 = 2.6;
const DEFAULT_PANEL_THICKNESS: f32 = 0.18;
const DEFAULT_PANEL_EDGE_SOFTNESS: f32 = 0.05;
const DEFAULT_KNOB_WIDTH: f32 = 0.32;
const DEFAULT_KNOB_HEIGHT: f32 = 0.32;
const DEFAULT_KNOB_DEPTH: f32 = 0.24;
const DEFAULT_KNOB_FRONT_FLATTEN: f32 = 0.16;
const DEFAULT_KNOB_BACK_FLATTEN: f32 = 0.48;
const DEFAULT_KNOB_X_OFFSET: f32 = 0.42;
const DEFAULT_KNOB_Y_OFFSET: f32 = 0.0;
const KNOB_X_OFFSET_MIN: f32 = -0.65;
const KNOB_X_OFFSET_MAX: f32 = 0.65;
const KNOB_Y_OFFSET_MIN: f32 = -0.55;
const KNOB_Y_OFFSET_MAX: f32 = 0.55;
const DEFAULT_KNOB_X_POSITION: f32 =
    (DEFAULT_KNOB_X_OFFSET - KNOB_X_OFFSET_MIN) / (KNOB_X_OFFSET_MAX - KNOB_X_OFFSET_MIN);
const DEFAULT_KNOB_Y_POSITION: f32 =
    (DEFAULT_KNOB_Y_OFFSET - KNOB_Y_OFFSET_MIN) / (KNOB_Y_OFFSET_MAX - KNOB_Y_OFFSET_MIN);

fn parameter_slots() -> Vec<shape_family::FamilyParameterSlot> {
    vec![
        length_slot(
            "panel_width",
            "Panel Width",
            PANEL_BODY_ROLE,
            0.6,
            4.0,
            0.05,
            DEFAULT_PANEL_WIDTH,
        ),
        length_slot(
            "panel_height",
            "Panel Height",
            PANEL_BODY_ROLE,
            0.8,
            4.0,
            0.05,
            DEFAULT_PANEL_HEIGHT,
        ),
        length_slot(
            "panel_thickness",
            "Panel Thickness",
            PANEL_BODY_ROLE,
            0.05,
            0.40,
            0.01,
            DEFAULT_PANEL_THICKNESS,
        ),
        crate::ratio_slot(
            "panel_edge_softness",
            "Panel Edge Softness",
            PANEL_BODY_ROLE,
            0.0,
            0.30,
            0.01,
            DEFAULT_PANEL_EDGE_SOFTNESS,
        ),
        length_slot(
            "knob_width",
            "Knob Width",
            KNOB_FORM_ROLE,
            0.12,
            0.8,
            0.01,
            DEFAULT_KNOB_WIDTH,
        ),
        length_slot(
            "knob_height",
            "Knob Height",
            KNOB_FORM_ROLE,
            0.12,
            0.8,
            0.01,
            DEFAULT_KNOB_HEIGHT,
        ),
        length_slot(
            "knob_depth",
            "Knob Depth",
            KNOB_FORM_ROLE,
            0.08,
            0.6,
            0.01,
            DEFAULT_KNOB_DEPTH,
        ),
        crate::ratio_slot(
            "knob_front_flatten",
            "Knob Front Flatten",
            KNOB_FORM_ROLE,
            0.0,
            0.8,
            0.02,
            DEFAULT_KNOB_FRONT_FLATTEN,
        ),
        crate::ratio_slot(
            "knob_back_flatten",
            "Knob Back Flatten",
            KNOB_FORM_ROLE,
            0.0,
            0.8,
            0.02,
            DEFAULT_KNOB_BACK_FLATTEN,
        ),
        crate::ratio_slot(
            "knob_x_offset",
            "Knob Horizontal Position",
            KNOB_FORM_ROLE,
            0.0,
            1.0,
            0.01,
            DEFAULT_KNOB_X_POSITION,
        ),
        crate::ratio_slot(
            "knob_y_offset",
            "Knob Vertical Position",
            KNOB_FORM_ROLE,
            0.0,
            1.0,
            0.01,
            DEFAULT_KNOB_Y_POSITION,
        ),
    ]
}

fn controls() -> Vec<shape_foundry::CustomizerControl> {
    vec![
        primary_control(
            "panel_width",
            "Panel Width",
            "panel_width",
            DEFAULT_PANEL_WIDTH,
            0.6,
            4.0,
        ),
        primary_control(
            "panel_height",
            "Panel Height",
            "panel_height",
            DEFAULT_PANEL_HEIGHT,
            0.8,
            4.0,
        ),
        primary_control(
            "panel_thickness",
            "Panel Thickness",
            "panel_thickness",
            DEFAULT_PANEL_THICKNESS,
            0.05,
            0.40,
        ),
        secondary_control(
            "panel_edge_softness",
            "Panel Edge Softness",
            "panel_edge_softness",
            DEFAULT_PANEL_EDGE_SOFTNESS,
            0.0,
            0.30,
        ),
        primary_control(
            "knob_width",
            "Knob Width",
            "knob_width",
            DEFAULT_KNOB_WIDTH,
            0.12,
            0.8,
        ),
        primary_control(
            "knob_height",
            "Knob Height",
            "knob_height",
            DEFAULT_KNOB_HEIGHT,
            0.12,
            0.8,
        ),
        primary_control(
            "knob_depth",
            "Knob Depth",
            "knob_depth",
            DEFAULT_KNOB_DEPTH,
            0.08,
            0.6,
        ),
        secondary_control(
            "knob_front_flatten",
            "Knob Front Flatten",
            "knob_front_flatten",
            DEFAULT_KNOB_FRONT_FLATTEN,
            0.0,
            0.8,
        ),
        primary_control(
            "knob_back_flatten",
            "Knob Back Flatten",
            "knob_back_flatten",
            DEFAULT_KNOB_BACK_FLATTEN,
            0.0,
            0.8,
        ),
        secondary_control(
            "knob_x_offset",
            "Knob Horizontal Position",
            "knob_x_offset",
            DEFAULT_KNOB_X_POSITION,
            0.0,
            1.0,
        ),
        secondary_control(
            "knob_y_offset",
            "Knob Vertical Position",
            "knob_y_offset",
            DEFAULT_KNOB_Y_POSITION,
            0.0,
            1.0,
        ),
    ]
}

fn primary_control(
    id: &str,
    label: &str,
    slot: &str,
    default: f32,
    minimum: f32,
    maximum: f32,
) -> shape_foundry::CustomizerControl {
    continuous_control(id, label, slot, default, minimum, maximum)
}

fn secondary_control(
    id: &str,
    label: &str,
    slot: &str,
    default: f32,
    minimum: f32,
    maximum: f32,
) -> shape_foundry::CustomizerControl {
    let mut control = continuous_control(id, label, slot, default, minimum, maximum);
    control.primary = false;
    control
}

fn default_control_state() -> BTreeMap<String, ControlValue> {
    BTreeMap::from([
        (
            "panel_width".to_owned(),
            ControlValue::Scalar(DEFAULT_PANEL_WIDTH),
        ),
        (
            "panel_height".to_owned(),
            ControlValue::Scalar(DEFAULT_PANEL_HEIGHT),
        ),
        (
            "panel_thickness".to_owned(),
            ControlValue::Scalar(DEFAULT_PANEL_THICKNESS),
        ),
        (
            "panel_edge_softness".to_owned(),
            ControlValue::Scalar(DEFAULT_PANEL_EDGE_SOFTNESS),
        ),
        (
            "knob_width".to_owned(),
            ControlValue::Scalar(DEFAULT_KNOB_WIDTH),
        ),
        (
            "knob_height".to_owned(),
            ControlValue::Scalar(DEFAULT_KNOB_HEIGHT),
        ),
        (
            "knob_depth".to_owned(),
            ControlValue::Scalar(DEFAULT_KNOB_DEPTH),
        ),
        (
            "knob_front_flatten".to_owned(),
            ControlValue::Scalar(DEFAULT_KNOB_FRONT_FLATTEN),
        ),
        (
            "knob_back_flatten".to_owned(),
            ControlValue::Scalar(DEFAULT_KNOB_BACK_FLATTEN),
        ),
        (
            "knob_x_offset".to_owned(),
            ControlValue::Scalar(DEFAULT_KNOB_X_POSITION),
        ),
        (
            "knob_y_offset".to_owned(),
            ControlValue::Scalar(DEFAULT_KNOB_Y_POSITION),
        ),
    ])
}

fn style_prototypes() -> Vec<(&'static str, &'static str, &'static str)> {
    vec![
        (PANEL_BODY_PROVIDER, "Upright panel body", PANEL_BODY_ROLE),
        (KNOB_FORM_PROVIDER, "Knob-like round form", KNOB_FORM_ROLE),
    ]
}

fn default_provider_map() -> BTreeMap<String, String> {
    BTreeMap::from([
        (PANEL_BODY_ROLE.to_owned(), PANEL_BODY_PROVIDER.to_owned()),
        (KNOB_FORM_ROLE.to_owned(), KNOB_FORM_PROVIDER.to_owned()),
    ])
}

fn parameter_bindings() -> Vec<ParameterBinding> {
    vec![
        scaled_definition_binding(
            "panel_width",
            PANEL_BODY_ROLE,
            crate::LOCAL_DEFINITION,
            "geometry.rounded_box.half_extents.x",
            0.5,
            0.0,
        ),
        scaled_definition_binding(
            "panel_height",
            PANEL_BODY_ROLE,
            crate::LOCAL_DEFINITION,
            "geometry.rounded_box.half_extents.y",
            0.5,
            0.0,
        ),
        scaled_definition_binding(
            "panel_thickness",
            PANEL_BODY_ROLE,
            crate::LOCAL_DEFINITION,
            "geometry.rounded_box.half_extents.z",
            0.5,
            0.0,
        ),
        scaled_definition_binding(
            "panel_edge_softness",
            PANEL_BODY_ROLE,
            crate::LOCAL_DEFINITION,
            "geometry.rounded_box.radius",
            0.25,
            0.0,
        ),
        instance_binding(
            "knob_width",
            KNOB_FORM_ROLE,
            crate::LOCAL_INSTANCE,
            "transform.scale.x",
            1.0,
            0.0,
        ),
        instance_binding(
            "knob_height",
            KNOB_FORM_ROLE,
            crate::LOCAL_INSTANCE,
            "transform.scale.y",
            1.0,
            0.0,
        ),
        instance_binding(
            "knob_depth",
            KNOB_FORM_ROLE,
            crate::LOCAL_INSTANCE,
            "transform.scale.z",
            1.0,
            0.0,
        ),
        instance_binding(
            "knob_depth",
            KNOB_FORM_ROLE,
            crate::LOCAL_INSTANCE,
            "transform.translation.z",
            0.5,
            DEFAULT_PANEL_THICKNESS * 0.5,
        ),
        scaled_definition_binding(
            "knob_front_flatten",
            KNOB_FORM_ROLE,
            crate::LOCAL_DEFINITION,
            "geometry.lathe.profile.12.y",
            -0.35,
            0.5,
        ),
        scaled_definition_binding(
            "knob_back_flatten",
            KNOB_FORM_ROLE,
            crate::LOCAL_DEFINITION,
            "geometry.lathe.profile.0.y",
            0.35,
            -0.5,
        ),
        instance_binding(
            "knob_x_offset",
            KNOB_FORM_ROLE,
            crate::LOCAL_INSTANCE,
            "transform.translation.x",
            KNOB_X_OFFSET_MAX - KNOB_X_OFFSET_MIN,
            KNOB_X_OFFSET_MIN,
        ),
        instance_binding(
            "knob_y_offset",
            KNOB_FORM_ROLE,
            crate::LOCAL_INSTANCE,
            "transform.translation.y",
            KNOB_Y_OFFSET_MAX - KNOB_Y_OFFSET_MIN,
            KNOB_Y_OFFSET_MIN,
        ),
    ]
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

fn instance_binding(
    slot: &str,
    role_name: &str,
    instance: PartInstanceId,
    local_key: &str,
    scale: f32,
    offset: f32,
) -> ParameterBinding {
    ParameterBinding::Scalar {
        slot: slot.to_owned(),
        role: role_name.to_owned(),
        local_path: instance_scalar_path(instance, local_key),
        transform: ScalarTransform::ScaleOffset { scale, offset },
    }
}

fn recipe_fragments() -> Vec<RecipeFragment> {
    vec![
        rounded_box_fragment(
            PANEL_BODY_PROVIDER,
            PANEL_BODY_ROLE,
            [
                DEFAULT_PANEL_WIDTH * 0.5,
                DEFAULT_PANEL_HEIGHT * 0.5,
                DEFAULT_PANEL_THICKNESS * 0.5,
            ],
            0.018,
            [0.0, 0.0, 0.0],
            Vec::new(),
        ),
        lathe_fragment(
            KNOB_FORM_PROVIDER,
            KNOB_FORM_ROLE,
            sphere_profile(),
            48,
            [
                DEFAULT_KNOB_X_OFFSET,
                DEFAULT_KNOB_Y_OFFSET,
                DEFAULT_PANEL_THICKNESS * 0.5 + DEFAULT_KNOB_DEPTH * 0.5,
            ],
            Vec::new(),
            &[
                ("geometry.lathe.profile.0.y", -0.5, -0.15, 0.01),
                ("geometry.lathe.profile.12.y", 0.15, 0.5, 0.01),
                ("instance.91.transform.scale.x", 0.12, 1.2, 0.01),
                ("instance.91.transform.scale.y", 0.12, 1.2, 0.01),
                ("instance.91.transform.scale.z", 0.08, 1.2, 0.01),
                (
                    "instance.91.transform.translation.x",
                    KNOB_X_OFFSET_MIN,
                    KNOB_X_OFFSET_MAX,
                    0.01,
                ),
                (
                    "instance.91.transform.translation.y",
                    KNOB_Y_OFFSET_MIN,
                    KNOB_Y_OFFSET_MAX,
                    0.01,
                ),
                ("instance.91.transform.translation.z", 0.13, 0.39, 0.01),
            ],
        ),
    ]
}

fn sphere_profile() -> Vec<[f32; 2]> {
    vec![
        [0.0, -0.5],
        [0.13, -0.48],
        [0.25, -0.43],
        [0.35, -0.35],
        [0.43, -0.25],
        [0.48, -0.13],
        [0.5, 0.0],
        [0.48, 0.13],
        [0.43, 0.25],
        [0.35, 0.35],
        [0.25, 0.43],
        [0.13, 0.48],
        [0.0, 0.5],
    ]
}
