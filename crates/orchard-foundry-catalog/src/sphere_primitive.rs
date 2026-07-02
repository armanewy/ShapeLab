//! Sphere Primitive fixture.

use std::collections::BTreeMap;

use orchard_asset::{
    PartDefinitionId, PartInstanceId, definition_scalar_path, instance_scalar_path,
};
use orchard_family::{AllowedOperationKind, RoleMultiplicity};
use orchard_family_compile::{ParameterBinding, RecipeFragment, ScalarTransform};
use orchard_foundry::ControlValue;

use crate::{
    CatalogCurationMetadata, FamilySchemaSpec, FixtureCatalogSpec, FoundryFixtureCatalog,
    StarterTemplateQualityEvidence, build_fixture_catalog, continuous_control,
    family_implementation, family_schema, lathe_fragment, length_slot, role,
    starter_template_curation_state_from_quality, style_implementation, style_kit,
};

/// Sphere Primitive profile slug.
pub const SPHERE_PRIMITIVE_SLUG: &str = "sphere-primitive";
/// Sphere Primitive family ID.
pub const SPHERE_PRIMITIVE_FAMILY_ID: &str = "sphere_primitive";
/// Neutral clay style ID for Sphere Primitive.
pub const SPHERE_PRIMITIVE_STYLE_ID: &str = "sphere_primitive_clay";
/// Product-safe preset label for a flattened rounded form.
pub const KNOB_LIKE_FORM_PRESET_LABEL: &str = "Knob-like form";

const SPHERE_BODY_ROLE: &str = "sphere_body";
const SPHERE_BODY_PROVIDER: &str = "sphere_body_lathe";

/// Quality evidence used to gate novice catalog exposure for Sphere Primitive.
#[must_use]
pub const fn quality_evidence() -> StarterTemplateQualityEvidence {
    StarterTemplateQualityEvidence {
        profile_slug: SPHERE_PRIMITIVE_SLUG,
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

/// Curation metadata for Sphere Primitive.
#[must_use]
pub fn curation_metadata() -> CatalogCurationMetadata {
    CatalogCurationMetadata {
        profile_slug: SPHERE_PRIMITIVE_SLUG,
        state: starter_template_curation_state_from_quality(quality_evidence()),
        has_visual_direction_evidence: true,
        has_readable_control_evidence: true,
        has_human_showcase_review: false,
        note: "Sphere Primitive is a direct round-clay baseline with bounded dimensions and flattening controls.",
    }
}

/// Return the deterministic Knob-like form preset as legal property values.
#[must_use]
pub fn knob_like_form_preset_values() -> BTreeMap<String, ControlValue> {
    BTreeMap::from([
        ("width".to_owned(), ControlValue::Scalar(0.72)),
        ("height".to_owned(), ControlValue::Scalar(0.72)),
        ("depth".to_owned(), ControlValue::Scalar(0.38)),
        ("front_flatten".to_owned(), ControlValue::Scalar(0.42)),
        ("back_flatten".to_owned(), ControlValue::Scalar(0.42)),
    ])
}

/// Build the Sphere Primitive fixture catalog.
#[must_use]
pub fn fixture_catalog() -> FoundryFixtureCatalog {
    let family = family_schema(FamilySchemaSpec {
        id: SPHERE_PRIMITIVE_FAMILY_ID,
        display_name: "Sphere Primitive",
        summary: "Pure clay closed round volume with readable dimensions and bounded flattening.",
        roles: vec![role(SPHERE_BODY_ROLE, RoleMultiplicity::Single, true)],
        allowed_operations: vec![
            AllowedOperationKind::Primitive,
            AllowedOperationKind::Array,
            AllowedOperationKind::Lathe,
            AllowedOperationKind::Transform,
            AllowedOperationKind::Bevel,
        ],
        parameter_slots: vec![
            length_slot("width", "Width", SPHERE_BODY_ROLE, 0.3, 2.4, 0.05, 1.0),
            length_slot("height", "Height", SPHERE_BODY_ROLE, 0.3, 2.4, 0.05, 1.0),
            length_slot("depth", "Depth", SPHERE_BODY_ROLE, 0.18, 2.0, 0.05, 1.0),
            crate::ratio_slot(
                "front_flatten",
                "Front Flatten",
                SPHERE_BODY_ROLE,
                0.0,
                0.8,
                0.02,
                0.0,
            ),
            crate::ratio_slot(
                "back_flatten",
                "Back Flatten",
                SPHERE_BODY_ROLE,
                0.0,
                0.8,
                0.02,
                0.0,
            ),
        ],
        compatible_style_kits: vec![SPHERE_PRIMITIVE_STYLE_ID.to_owned()],
        tags: vec![
            "sphere-primitive".to_owned(),
            "primitive-family".to_owned(),
            "clay".to_owned(),
        ],
    });

    let style = style_kit(
        SPHERE_PRIMITIVE_STYLE_ID,
        "Sphere Primitive Clay",
        SPHERE_PRIMITIVE_FAMILY_ID,
        &style_prototypes(),
        vec![
            "sphere-primitive".to_owned(),
            "round".to_owned(),
            "clay".to_owned(),
        ],
    );

    let family_impl = family_implementation(
        SPHERE_PRIMITIVE_FAMILY_ID,
        "Sphere Primitive family",
        parameter_bindings(),
    );

    let style_impl = style_implementation(
        SPHERE_PRIMITIVE_STYLE_ID,
        SPHERE_PRIMITIVE_FAMILY_ID,
        default_provider_map(),
        recipe_fragments(),
    );

    let mut profile = crate::customizer_profile(
        SPHERE_PRIMITIVE_FAMILY_ID,
        SPHERE_PRIMITIVE_STYLE_ID,
        vec![
            continuous_control("width", "Width", "width", 1.0, 0.3, 2.4),
            continuous_control("height", "Height", "height", 1.0, 0.3, 2.4),
            continuous_control("depth", "Depth", "depth", 1.0, 0.18, 2.0),
            continuous_control(
                "front_flatten",
                "Front Flatten",
                "front_flatten",
                0.0,
                0.0,
                0.8,
            ),
            continuous_control(
                "back_flatten",
                "Back Flatten",
                "back_flatten",
                0.0,
                0.0,
                0.8,
            ),
        ],
    );
    profile.candidate_strategies = Vec::new();

    build_fixture_catalog(FixtureCatalogSpec {
        slug: SPHERE_PRIMITIVE_SLUG,
        document_id: "sphere-primitive-doc",
        family,
        style,
        family_implementation: family_impl,
        style_implementation: style_impl,
        customizer_profile: profile,
        control_state: BTreeMap::from([
            ("width".to_owned(), ControlValue::Scalar(1.0)),
            ("height".to_owned(), ControlValue::Scalar(1.0)),
            ("depth".to_owned(), ControlValue::Scalar(1.0)),
            ("front_flatten".to_owned(), ControlValue::Scalar(0.0)),
            ("back_flatten".to_owned(), ControlValue::Scalar(0.0)),
        ]),
    })
}

fn style_prototypes() -> Vec<(&'static str, &'static str, &'static str)> {
    vec![(SPHERE_BODY_PROVIDER, "Round clay body", SPHERE_BODY_ROLE)]
}

fn default_provider_map() -> BTreeMap<String, String> {
    BTreeMap::from([(SPHERE_BODY_ROLE.to_owned(), SPHERE_BODY_PROVIDER.to_owned())])
}

fn parameter_bindings() -> Vec<ParameterBinding> {
    vec![
        instance_scale_binding(
            "width",
            SPHERE_BODY_ROLE,
            crate::LOCAL_INSTANCE,
            "transform.scale.x",
        ),
        instance_scale_binding(
            "height",
            SPHERE_BODY_ROLE,
            crate::LOCAL_INSTANCE,
            "transform.scale.y",
        ),
        instance_scale_binding(
            "depth",
            SPHERE_BODY_ROLE,
            crate::LOCAL_INSTANCE,
            "transform.scale.z",
        ),
        scaled_definition_binding(
            "front_flatten",
            SPHERE_BODY_ROLE,
            crate::LOCAL_DEFINITION,
            "geometry.lathe.profile.12.y",
            -0.35,
            0.5,
        ),
        scaled_definition_binding(
            "back_flatten",
            SPHERE_BODY_ROLE,
            crate::LOCAL_DEFINITION,
            "geometry.lathe.profile.0.y",
            0.35,
            -0.5,
        ),
    ]
}

fn instance_scale_binding(
    slot: &str,
    role_name: &str,
    instance: PartInstanceId,
    local_key: &str,
) -> ParameterBinding {
    ParameterBinding::Scalar {
        slot: slot.to_owned(),
        role: role_name.to_owned(),
        local_path: instance_scalar_path(instance, local_key),
        transform: ScalarTransform::ScaleOffset {
            scale: 1.0,
            offset: 0.0,
        },
    }
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

fn recipe_fragments() -> Vec<RecipeFragment> {
    vec![lathe_fragment(
        SPHERE_BODY_PROVIDER,
        SPHERE_BODY_ROLE,
        sphere_profile(),
        48,
        [0.0, 0.0, 0.0],
        Vec::new(),
        &[
            ("geometry.lathe.profile.0.y", -0.5, -0.15, 0.01),
            ("geometry.lathe.profile.12.y", 0.15, 0.5, 0.01),
            ("instance.91.transform.scale.x", 0.3, 2.4, 0.05),
            ("instance.91.transform.scale.y", 0.3, 2.4, 0.05),
            ("instance.91.transform.scale.z", 0.18, 2.0, 0.05),
        ],
    )]
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
