#![forbid(unsafe_code)]

use std::{collections::BTreeSet, fs};

use orchard_asset::PartInstanceId;
use orchard_compile::{
    CompiledPart,
    export::{verify_model_package, write_model_package},
    validation::{ValidationLimits, validate_model, validation_config_from_recipe_with_limits},
};
use orchard_family::AssetFamilySchema;
use orchard_foundry::{
    ControlKind, ControlValue, CustomizerProfile, FoundryCompilationOutput,
    compile_foundry_document, sphere_primitive_property_schema, validate_primitive_property_schema,
};
use orchard_foundry_catalog::{FoundryFixtureCatalog, sphere_primitive};
use serde::de::DeserializeOwned;

fn payload<T: DeserializeOwned>(fixture: &FoundryFixtureCatalog, id: &str) -> T {
    serde_json::from_str(&fixture.entries[id].canonical_json).expect("catalog payload decodes")
}

fn family(fixture: &FoundryFixtureCatalog) -> AssetFamilySchema {
    payload(fixture, &format!("{}-family", fixture.slug))
}

fn profile(fixture: &FoundryFixtureCatalog) -> CustomizerProfile {
    payload(fixture, &format!("{}-profile", fixture.slug))
}

fn compile_with(overrides: &[(&str, ControlValue)]) -> FoundryCompilationOutput {
    let fixture = sphere_primitive::fixture_catalog();
    let mut document = fixture.document.clone();
    for (control, value) in overrides {
        document
            .control_state
            .insert((*control).to_owned(), value.clone());
    }
    compile_foundry_document(&document, &fixture).expect("fixture variant compiles")
}

fn assert_valid_model(output: &FoundryCompilationOutput) {
    assert!(output.final_conformance.is_accepted());
    assert!(output.artifact.validation_report.is_valid());
    let config = validation_config_from_recipe_with_limits(
        &output.recipe,
        &output.artifact,
        ValidationLimits::default(),
    );
    let report = validate_model(&output.artifact, &config);
    assert!(
        report.is_valid(),
        "Sphere Primitive validation should pass: {:#?}",
        report.issues
    );
}

fn role_instances(output: &FoundryCompilationOutput, role: &str) -> Vec<PartInstanceId> {
    let tag = format!("role:{role}");
    output
        .recipe
        .instances
        .iter()
        .filter(|(_, instance)| instance.tags.contains(&tag))
        .map(|(id, _)| *id)
        .collect()
}

fn compiled_part_for_role<'a>(
    output: &'a FoundryCompilationOutput,
    role: &str,
) -> &'a CompiledPart {
    let instances = role_instances(output, role);
    assert_eq!(
        instances.len(),
        1,
        "{role} should have exactly one instance"
    );
    output
        .artifact
        .compiled_parts
        .iter()
        .find(|part| part.instance_id == instances[0])
        .expect("compiled role part exists")
}

fn axis_extent(part: &CompiledPart, axis: usize) -> f32 {
    part.world_mesh.bounds.max[axis] - part.world_mesh.bounds.min[axis]
}

fn axis_center(part: &CompiledPart, axis: usize) -> f32 {
    (part.world_mesh.bounds.max[axis] + part.world_mesh.bounds.min[axis]) * 0.5
}

fn assert_close(left: f32, right: f32, tolerance: f32, message: &str) {
    assert!(
        (left - right).abs() <= tolerance,
        "{message}: expected {left} ~= {right}"
    );
}

#[test]
fn sphere_primitive_validates_exports_and_has_only_round_body_role() {
    let output = compile_with(&[]);
    assert_valid_model(&output);
    assert_eq!(role_instances(&output, "sphere_body").len(), 1);
    for forbidden_role in [
        "door",
        "panel",
        "handle",
        "knob",
        "hinge_edge",
        "surface_detail",
    ] {
        assert!(
            role_instances(&output, forbidden_role).is_empty(),
            "Sphere Primitive must not expose {forbidden_role}"
        );
    }

    let package_dir = std::env::temp_dir().join(format!(
        "shape-lab-sphere-primitive-export-{}",
        std::process::id()
    ));
    let _ = fs::remove_dir_all(&package_dir);
    write_model_package(&output.recipe, &output.artifact, &package_dir).expect("write package");
    let verification = verify_model_package(&package_dir).expect("verify package");
    assert!(
        verification.checksums_match
            && verification.topology_matches_manifest
            && verification.finite_numeric_payloads,
        "package verification should pass: {verification:#?}"
    );
    fs::remove_dir_all(&package_dir).expect("clean package temp dir");
}

#[test]
fn sphere_primitive_default_body_is_origin_centered_and_round() {
    let output = compile_with(&[]);
    assert_valid_model(&output);
    let body = compiled_part_for_role(&output, "sphere_body");

    for axis in 0..3 {
        assert_close(
            axis_center(body, axis),
            0.0,
            0.001,
            "default sphere body should be centered on the origin",
        );
    }
    assert_close(
        axis_extent(body, 0),
        axis_extent(body, 1),
        0.03,
        "default sphere width and height should match",
    );
    assert_close(
        axis_extent(body, 0),
        axis_extent(body, 2),
        0.03,
        "default sphere width and depth should match",
    );
}

#[test]
fn sphere_primitive_controls_schema_and_copy_are_honest() {
    let fixture = sphere_primitive::fixture_catalog();
    let family = family(&fixture);
    let profile = profile(&fixture);
    let property_schema = sphere_primitive_property_schema();

    assert!(validate_primitive_property_schema(&property_schema).is_valid());
    assert_eq!(fixture.slug, sphere_primitive::SPHERE_PRIMITIVE_SLUG);
    assert_eq!(family.id, sphere_primitive::SPHERE_PRIMITIVE_FAMILY_ID);
    assert_eq!(family.display_name, "Sphere Primitive");
    assert_eq!(
        profile.family_id,
        sphere_primitive::SPHERE_PRIMITIVE_FAMILY_ID
    );
    assert_eq!(
        profile.style_id.as_deref(),
        Some(sphere_primitive::SPHERE_PRIMITIVE_STYLE_ID)
    );
    assert_eq!(
        family
            .part_roles
            .iter()
            .map(|role| role.id.as_str())
            .collect::<Vec<_>>(),
        vec!["sphere_body"]
    );

    let primary = profile
        .controls
        .iter()
        .filter(|control| control.primary && control.visible)
        .collect::<Vec<_>>();
    assert_eq!(primary.len(), 5);
    assert_eq!(
        primary
            .iter()
            .map(|control| control.label.as_str())
            .collect::<Vec<_>>(),
        vec!["Width", "Height", "Depth", "Front Flatten", "Back Flatten"]
    );
    assert!(primary.iter().all(|control| {
        matches!(control.kind, ControlKind::ContinuousAxis { .. })
            && control.domain.continuous_intervals.len() == 1
    }));
    assert!(
        profile.candidate_strategies.is_empty(),
        "Sphere Primitive active profile should not expose product candidate strategies"
    );

    let visible_copy = [
        family.display_name.as_str(),
        family.summary.as_str(),
        sphere_primitive::KNOB_LIKE_FORM_PRESET_LABEL,
    ]
    .join(" ")
    .to_ascii_lowercase();
    for forbidden in [
        "door",
        "panel",
        "handle",
        "sculpt",
        "vertex",
        "face",
        "boolean",
        "topology",
        "uv",
        "texture",
        "material",
        "rig",
        "animation",
    ] {
        assert!(
            !visible_copy.contains(forbidden),
            "Sphere Primitive copy must not claim {forbidden}: {visible_copy}"
        );
    }
    assert!(visible_copy.contains("knob-like form"));
}

#[test]
fn sphere_primitive_dimension_controls_change_geometry() {
    let default = compile_with(&[]);
    let wide = compile_with(&[("width", ControlValue::Scalar(1.4))]);
    let tall = compile_with(&[("height", ControlValue::Scalar(1.5))]);
    let shallow = compile_with(&[("depth", ControlValue::Scalar(0.55))]);
    assert_valid_model(&default);
    assert_valid_model(&wide);
    assert_valid_model(&tall);
    assert_valid_model(&shallow);

    let default_body = compiled_part_for_role(&default, "sphere_body");
    let wide_body = compiled_part_for_role(&wide, "sphere_body");
    let tall_body = compiled_part_for_role(&tall, "sphere_body");
    let shallow_body = compiled_part_for_role(&shallow, "sphere_body");

    assert!(axis_extent(wide_body, 0) > axis_extent(default_body, 0));
    assert!(axis_extent(tall_body, 1) > axis_extent(default_body, 1));
    assert!(axis_extent(shallow_body, 2) < axis_extent(default_body, 2));
}

#[test]
fn sphere_primitive_flatten_controls_change_geometry() {
    let default = compile_with(&[]);
    let front_flat = compile_with(&[("front_flatten", ControlValue::Scalar(0.65))]);
    let back_flat = compile_with(&[("back_flatten", ControlValue::Scalar(0.65))]);
    assert_valid_model(&default);
    assert_valid_model(&front_flat);
    assert_valid_model(&back_flat);

    let default_body = compiled_part_for_role(&default, "sphere_body");
    let front_body = compiled_part_for_role(&front_flat, "sphere_body");
    let back_body = compiled_part_for_role(&back_flat, "sphere_body");

    assert_ne!(
        default_body.world_mesh.positions, front_body.world_mesh.positions,
        "Front Flatten should alter the round body geometry"
    );
    assert_ne!(
        default_body.world_mesh.positions, back_body.world_mesh.positions,
        "Back Flatten should alter the round body geometry"
    );
}

#[test]
fn sphere_primitive_knob_like_preset_uses_only_legal_values() {
    let fixture = sphere_primitive::fixture_catalog();
    let profile = profile(&fixture);
    let preset = sphere_primitive::knob_like_form_preset_values();
    assert_eq!(
        preset.keys().map(String::as_str).collect::<BTreeSet<_>>(),
        ["back_flatten", "depth", "front_flatten", "height", "width"]
            .into_iter()
            .collect::<BTreeSet<_>>()
    );
    for control in &profile.controls {
        let value = preset
            .get(&control.id)
            .unwrap_or_else(|| panic!("preset missing {}", control.id));
        assert!(
            control.domain.contains_available_value(value),
            "preset value for {} must be legal: {value:?}",
            control.id
        );
    }

    let overrides = preset
        .iter()
        .map(|(control, value)| (control.as_str(), value.clone()))
        .collect::<Vec<_>>();
    let output = compile_with(&overrides);
    assert_valid_model(&output);
    let body = compiled_part_for_role(&output, "sphere_body");
    assert!(
        axis_extent(body, 2) < axis_extent(body, 0),
        "Knob-like form should be visibly flattened in depth"
    );
}
