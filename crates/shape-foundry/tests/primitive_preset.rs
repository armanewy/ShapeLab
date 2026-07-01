use shape_foundry::{
    ObjectPlanReviewTier, PresetSource, PrimitiveKind, PrimitivePreset,
    PrimitivePresetObjectPlanNodeError, PrimitivePropertyValue, built_in_primitive_preset,
    built_in_primitive_presets, object_plan_node_from_reviewed_preset,
    primitive_preset_public_catalog_publish_allowed, validate_primitive_preset,
};

#[test]
fn primitive_preset_built_ins_validate() {
    let presets = built_in_primitive_presets();

    assert_eq!(presets.len(), 12);
    for preset in presets {
        let report = validate_primitive_preset(&preset);
        assert!(
            report.is_valid(),
            "preset {} should validate: {:?}",
            preset.preset_id,
            report.issues
        );
        assert_eq!(preset.source, PresetSource::BuiltIn);
        assert_eq!(preset.review_tier, ObjectPlanReviewTier::Reviewed);
    }
}

#[test]
fn primitive_preset_required_names_are_present_without_door_knob() {
    let names = built_in_primitive_presets()
        .into_iter()
        .map(|preset| preset.display_name)
        .collect::<Vec<_>>();

    for expected in [
        "Compact Box",
        "Wide Box",
        "Tall Box",
        "Flat Box",
        "Narrow Panel",
        "Wide Panel",
        "Tall Panel",
        "Short Panel",
        "Round Sphere",
        "Squashed Sphere",
        "Flattened Back Sphere",
        "Knob-Like Form",
    ] {
        assert!(
            names.iter().any(|name| name == expected),
            "missing {expected}"
        );
    }
    assert!(
        names.iter().all(|name| name != "Door Knob"),
        "preset names must not use Door Knob yet"
    );
}

#[test]
fn primitive_preset_invalid_property_rejected() {
    let mut preset = built_in_primitive_preset("round_sphere").expect("preset exists");
    preset
        .property_values
        .insert("raw_radius".to_owned(), PrimitivePropertyValue::Length(0.5));

    let report = validate_primitive_preset(&preset);

    assert_issue(&report, "unknown_current_property_value");
}

#[test]
fn primitive_preset_out_of_domain_value_rejected() {
    let mut preset = built_in_primitive_preset("round_sphere").expect("preset exists");
    preset
        .property_values
        .insert("width".to_owned(), PrimitivePropertyValue::Length(99.0));

    let report = validate_primitive_preset(&preset);

    assert_issue(&report, "invalid_current_property_value");
}

#[test]
fn primitive_preset_user_copy_hides_internal_terms() {
    for preset in built_in_primitive_presets() {
        assert_product_safe(&preset);
    }

    let mut preset = built_in_primitive_preset("compact_box").expect("preset exists");
    preset.user_description = "Uses an internal provider slot.".to_owned();

    let report = validate_primitive_preset(&preset);

    assert_issue(&report, "invalid_preset_user_description");
}

#[test]
fn primitive_preset_raw_mesh_payload_rejected() {
    let preset = built_in_primitive_preset("compact_box").expect("preset exists");
    let mut value = serde_json::to_value(&preset).expect("preset serializes");
    value.as_object_mut().expect("preset object").insert(
        "raw_mesh_payload".to_owned(),
        serde_json::json!({"vertices": []}),
    );

    let decoded = serde_json::from_value::<PrimitivePreset>(value);

    assert!(
        decoded.is_err(),
        "PrimitivePreset must reject raw mesh payload fields"
    );
}

#[test]
fn primitive_preset_public_catalog_publish_never_allowed() {
    for preset in built_in_primitive_presets() {
        assert!(!primitive_preset_public_catalog_publish_allowed(&preset));
    }
}

#[test]
fn primitive_preset_object_plan_node_from_reviewed_preset() {
    let preset = built_in_primitive_preset("knob_like_form").expect("preset exists");

    let node =
        object_plan_node_from_reviewed_preset(&preset, "knob", "Rounded attached form", false)
            .expect("reviewed preset converts");

    assert_eq!(node.node_id, "knob");
    assert_eq!(node.primitive_kind, PrimitiveKind::SpherePrimitive);
    assert_eq!(node.display_name, "Knob-Like Form");
    assert_eq!(node.property_values, preset.property_values);
}

#[test]
fn primitive_preset_object_plan_node_requires_reviewed_preset() {
    let mut preset = built_in_primitive_preset("knob_like_form").expect("preset exists");
    preset.review_tier = ObjectPlanReviewTier::Draft;
    preset.source = PresetSource::ObjectPlanDraft;

    let err =
        object_plan_node_from_reviewed_preset(&preset, "knob", "Rounded attached form", false)
            .expect_err("draft preset must not convert");

    assert_eq!(
        err,
        PrimitivePresetObjectPlanNodeError::PresetRequiresReview
    );
}

#[test]
fn primitive_preset_serde_roundtrip_is_deterministic() {
    let preset = built_in_primitive_preset("wide_panel").expect("preset exists");

    let first = serde_json::to_string(&preset).expect("preset serializes");
    let decoded = serde_json::from_str::<PrimitivePreset>(&first).expect("preset decodes");
    let second = serde_json::to_string(&decoded).expect("preset serializes again");

    assert_eq!(first, second);
    assert_eq!(preset, decoded);
}

fn assert_issue(report: &shape_foundry::PrimitivePresetValidationReport, expected_code: &str) {
    assert!(
        report
            .issues
            .iter()
            .any(|issue| issue.code == expected_code),
        "expected issue {expected_code}, got {:?}",
        report.issues
    );
}

fn assert_product_safe(preset: &PrimitivePreset) {
    let all_text = std::iter::once(preset.display_name.as_str())
        .chain(std::iter::once(preset.user_description.as_str()))
        .chain(preset.intended_use_tags.iter().map(String::as_str))
        .collect::<Vec<_>>()
        .join(" ");
    for forbidden in [
        "kernel",
        "module",
        "provider",
        "slot",
        "topology",
        "fingerprint",
        "raw transform",
        "mesh payload",
        "conformance",
        "artifact",
        "door knob",
    ] {
        assert!(
            !all_text.to_ascii_lowercase().contains(forbidden),
            "preset copy must not expose {forbidden}: {all_text}"
        );
    }
}
