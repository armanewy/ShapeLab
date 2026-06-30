#![forbid(unsafe_code)]

use std::{collections::BTreeSet, fs};

use serde::de::DeserializeOwned;
use shape_asset::PartInstanceId;
use shape_compile::{
    CompiledPart,
    export::{verify_model_package, write_model_package},
    validation::{ValidationLimits, validate_model, validation_config_from_recipe_with_limits},
};
use shape_family::AssetFamilySchema;
use shape_foundry::{
    CandidateLegibilityClass, ControlKind, ControlValue, CustomizerProfile,
    FoundryCompilationOutput, compile_foundry_document,
};
use shape_foundry_catalog::{FoundryFixtureCatalog, flat_panel};
use shape_search::foundry::generate_foundry_control_endpoint_visibility_report;

fn payload<T: DeserializeOwned>(fixture: &FoundryFixtureCatalog, id: &str) -> T {
    serde_json::from_str(&fixture.entries[id].canonical_json).expect("catalog payload decodes")
}

fn family(fixture: &FoundryFixtureCatalog) -> AssetFamilySchema {
    payload(fixture, &format!("{}-family", fixture.slug))
}

fn profile(fixture: &FoundryFixtureCatalog) -> CustomizerProfile {
    payload(fixture, &format!("{}-profile", fixture.slug))
}

fn compile_fixture_with(
    fixture: &FoundryFixtureCatalog,
    overrides: &[(&str, ControlValue)],
) -> FoundryCompilationOutput {
    let mut document = fixture.document.clone();
    for (control, value) in overrides {
        document
            .control_state
            .insert((*control).to_owned(), value.clone());
    }
    compile_foundry_document(&document, fixture).expect("fixture variant compiles")
}

fn compile_with(overrides: &[(&str, ControlValue)]) -> FoundryCompilationOutput {
    compile_fixture_with(&flat_panel::fixture_catalog(), overrides)
}

fn compile_hinged_with(overrides: &[(&str, ControlValue)]) -> FoundryCompilationOutput {
    compile_fixture_with(&flat_panel::hinged_panel_fixture_catalog(), overrides)
}

fn compile_handled_with(overrides: &[(&str, ControlValue)]) -> FoundryCompilationOutput {
    compile_fixture_with(&flat_panel::handled_panel_fixture_catalog(), overrides)
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
        "Flat Panel validation should pass: {:#?}",
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

fn ranges_overlap(a_min: f32, a_max: f32, b_min: f32, b_max: f32) -> bool {
    a_min <= b_max && b_min <= a_max
}

#[test]
fn flat_panel_validates_exports_and_has_only_panel_body_role() {
    let output = compile_with(&[]);
    assert_valid_model(&output);
    assert_eq!(role_instances(&output, "panel_body").len(), 1);
    for forbidden_role in [
        "door",
        "hinge_edge",
        "handle",
        "knob",
        "handle_knob",
        "inset_panel",
        "motion",
        "surface_detail",
    ] {
        assert!(
            role_instances(&output, forbidden_role).is_empty(),
            "Flat Panel Primitive must not expose {forbidden_role}"
        );
    }

    let package_dir = std::env::temp_dir().join(format!(
        "shape-lab-flat-panel-export-{}",
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
fn hinged_panel_validates_exports_and_adds_only_hinge_edge() {
    let output = compile_hinged_with(&[]);
    assert_valid_model(&output);
    assert_eq!(role_instances(&output, "panel_body").len(), 1);
    assert_eq!(role_instances(&output, "hinge_edge").len(), 1);
    for forbidden_role in [
        "door",
        "handle",
        "knob",
        "handle_knob",
        "inset_panel",
        "motion",
        "surface_detail",
    ] {
        assert!(
            role_instances(&output, forbidden_role).is_empty(),
            "Hinged Panel must not expose {forbidden_role}"
        );
    }

    let body = compiled_part_for_role(&output, "panel_body");
    let hinge = compiled_part_for_role(&output, "hinge_edge");
    let body_bounds = body.world_mesh.bounds;
    let hinge_bounds = hinge.world_mesh.bounds;
    assert!(
        hinge_bounds.min[0] < body_bounds.min[0],
        "hinge edge should sit on the side of the panel"
    );
    assert!(
        hinge_bounds.max[0] >= body_bounds.min[0] - 0.001,
        "hinge edge should overlap the panel edge instead of floating"
    );
    assert!(
        ranges_overlap(
            hinge_bounds.min[1],
            hinge_bounds.max[1],
            body_bounds.min[1],
            body_bounds.max[1]
        ) && ranges_overlap(
            hinge_bounds.min[2],
            hinge_bounds.max[2],
            body_bounds.min[2],
            body_bounds.max[2]
        ),
        "hinge edge should share the panel height and thickness ranges"
    );

    let package_dir = std::env::temp_dir().join(format!(
        "shape-lab-hinged-panel-export-{}",
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
fn handled_panel_validates_exports_and_adds_only_handle_knob_to_hinged_panel() {
    let output = compile_handled_with(&[]);
    assert_valid_model(&output);
    assert_eq!(role_instances(&output, "panel_body").len(), 1);
    assert_eq!(role_instances(&output, "hinge_edge").len(), 1);
    assert_eq!(role_instances(&output, "handle_knob").len(), 1);
    for forbidden_role in [
        "door",
        "handle",
        "knob",
        "inset_panel",
        "motion",
        "surface_detail",
        "latch",
        "frame",
    ] {
        assert!(
            role_instances(&output, forbidden_role).is_empty(),
            "Handled Panel must not expose {forbidden_role}"
        );
    }

    let body = compiled_part_for_role(&output, "panel_body");
    let hinge = compiled_part_for_role(&output, "hinge_edge");
    let handle = compiled_part_for_role(&output, "handle_knob");
    let body_bounds = body.world_mesh.bounds;
    let hinge_bounds = hinge.world_mesh.bounds;
    let handle_bounds = handle.world_mesh.bounds;

    assert!(
        hinge_bounds.max[0] <= body_bounds.min[0] + 0.001,
        "hinge edge should remain on the negative-x side"
    );
    assert!(
        handle_bounds.min[0] > body_bounds.min[0] && handle_bounds.max[0] < body_bounds.max[0],
        "handle/knob should stay inside panel width bounds"
    );
    assert!(
        handle_bounds.min[0] > 0.0,
        "handle/knob should sit opposite the hinge-side edge"
    );
    assert!(
        ranges_overlap(
            handle_bounds.min[1],
            handle_bounds.max[1],
            body_bounds.min[1],
            body_bounds.max[1]
        ) && ranges_overlap(
            handle_bounds.min[0],
            handle_bounds.max[0],
            body_bounds.min[0],
            body_bounds.max[0]
        ),
        "handle/knob should overlap the panel face area"
    );
    assert!(
        (handle_bounds.min[2] - body_bounds.max[2]).abs() <= 0.001,
        "handle/knob should attach to the front face without floating"
    );
    assert!(
        handle_bounds.max[2] > body_bounds.max[2],
        "handle/knob should visibly protrude from the panel face"
    );

    let package_dir = std::env::temp_dir().join(format!(
        "shape-lab-handled-panel-export-{}",
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
fn flat_panel_controls_and_copy_are_honest() {
    let fixture = flat_panel::fixture_catalog();
    let family = family(&fixture);
    let profile = profile(&fixture);

    assert_eq!(fixture.slug, flat_panel::FLAT_PANEL_PRIMITIVE_SLUG);
    assert_eq!(family.id, flat_panel::FLAT_PANEL_PRIMITIVE_FAMILY_ID);
    assert_eq!(family.display_name, "Flat Panel Primitive");
    assert_eq!(
        profile.family_id,
        flat_panel::FLAT_PANEL_PRIMITIVE_FAMILY_ID
    );
    assert_eq!(
        profile.style_id.as_deref(),
        Some(flat_panel::FLAT_PANEL_PRIMITIVE_STYLE_ID)
    );
    assert_eq!(
        family
            .part_roles
            .iter()
            .map(|role| role.id.as_str())
            .collect::<Vec<_>>(),
        vec!["panel_body"]
    );

    let primary = profile
        .controls
        .iter()
        .filter(|control| control.primary && control.visible)
        .collect::<Vec<_>>();
    assert_eq!(primary.len(), 2);
    assert_eq!(
        primary
            .iter()
            .map(|control| control.label.as_str())
            .collect::<Vec<_>>(),
        vec!["Proportions", "Edge Softness"]
    );
    assert!(primary.iter().any(|control| {
        control.id == "proportions" && matches!(control.kind, ControlKind::ChoiceGallery { .. })
    }));
    assert!(primary.iter().any(|control| {
        control.id == "edge_softness" && matches!(control.kind, ControlKind::ContinuousAxis { .. })
    }));

    let strategy_copy = profile
        .candidate_strategies
        .iter()
        .map(|strategy| strategy.label.as_str())
        .collect::<Vec<_>>()
        .join(" ");
    assert_eq!(
        profile
            .candidate_strategies
            .iter()
            .map(|strategy| strategy.label.as_str())
            .collect::<Vec<_>>(),
        vec![
            "Narrow Panel",
            "Wide Panel",
            "Tall Panel",
            "Short Panel",
            "Soft-Edged Panel",
            "Sharp Panel",
        ]
    );
    let visible_copy = [
        family.display_name.as_str(),
        family.summary.as_str(),
        strategy_copy.as_str(),
    ]
    .join(" ")
    .to_ascii_lowercase();
    for forbidden in [
        "door",
        "hinge",
        "handle",
        "knob",
        "open",
        "close",
        "uv",
        "texture",
        "material",
        "rigging",
        "rigged",
        "animation",
        "game-ready",
    ] {
        assert!(
            !visible_copy.contains(forbidden),
            "Flat Panel copy must not claim {forbidden}: {visible_copy}"
        );
    }
}

#[test]
fn hinged_panel_controls_and_copy_are_honest() {
    let fixture = flat_panel::hinged_panel_fixture_catalog();
    let family = family(&fixture);
    let profile = profile(&fixture);

    assert_eq!(fixture.slug, flat_panel::HINGED_PANEL_SLUG);
    assert_eq!(family.id, flat_panel::HINGED_PANEL_FAMILY_ID);
    assert_eq!(family.display_name, "Hinged Panel");
    assert_eq!(profile.family_id, flat_panel::HINGED_PANEL_FAMILY_ID);
    assert_eq!(
        profile.style_id.as_deref(),
        Some(flat_panel::HINGED_PANEL_STYLE_ID)
    );
    assert_eq!(
        family
            .part_roles
            .iter()
            .map(|role| role.id.as_str())
            .collect::<Vec<_>>(),
        vec!["panel_body", "hinge_edge"]
    );

    let primary = profile
        .controls
        .iter()
        .filter(|control| control.primary && control.visible)
        .collect::<Vec<_>>();
    assert_eq!(primary.len(), 3);
    assert_eq!(
        primary
            .iter()
            .map(|control| control.label.as_str())
            .collect::<Vec<_>>(),
        vec!["Proportions", "Edge Softness", "Hinge Edge"]
    );
    assert!(primary.iter().any(|control| {
        control.id == "hinge_edge_style"
            && matches!(control.kind, ControlKind::ContinuousAxis { .. })
    }));

    let strategy_copy = profile
        .candidate_strategies
        .iter()
        .map(|strategy| strategy.label.as_str())
        .collect::<Vec<_>>()
        .join(" ");
    assert_eq!(
        profile
            .candidate_strategies
            .iter()
            .map(|strategy| strategy.label.as_str())
            .collect::<Vec<_>>(),
        vec![
            "Clean Hinged Panel",
            "Wide Hinged Panel",
            "Tall Hinged Panel",
            "Heavy Edge Panel",
            "Minimal Hinged Panel",
        ]
    );
    let visible_copy = [
        family.display_name.as_str(),
        family.summary.as_str(),
        strategy_copy.as_str(),
    ]
    .join(" ")
    .to_ascii_lowercase();
    for forbidden in [
        "door",
        "handle",
        "knob",
        "open",
        "close",
        "uv",
        "texture",
        "material",
        "rigging",
        "rigged",
        "animation",
        "game-ready",
    ] {
        assert!(
            !visible_copy.contains(forbidden),
            "Hinged Panel copy must not claim {forbidden}: {visible_copy}"
        );
    }
}

#[test]
fn handled_panel_controls_and_copy_are_honest() {
    let fixture = flat_panel::handled_panel_fixture_catalog();
    let family = family(&fixture);
    let profile = profile(&fixture);

    assert_eq!(fixture.slug, flat_panel::HANDLED_PANEL_SLUG);
    assert_eq!(family.id, flat_panel::HANDLED_PANEL_FAMILY_ID);
    assert_eq!(family.display_name, "Handled Panel");
    assert_eq!(
        family.summary,
        "A simple upright clay panel with a visible hinge edge and handle."
    );
    assert_eq!(profile.family_id, flat_panel::HANDLED_PANEL_FAMILY_ID);
    assert_eq!(
        profile.style_id.as_deref(),
        Some(flat_panel::HANDLED_PANEL_STYLE_ID)
    );
    assert_eq!(
        family
            .part_roles
            .iter()
            .map(|role| role.id.as_str())
            .collect::<Vec<_>>(),
        vec!["panel_body", "hinge_edge", "handle_knob"]
    );

    let primary = profile
        .controls
        .iter()
        .filter(|control| control.primary && control.visible)
        .collect::<Vec<_>>();
    assert_eq!(primary.len(), 4);
    assert_eq!(
        primary
            .iter()
            .map(|control| control.label.as_str())
            .collect::<Vec<_>>(),
        vec![
            "Proportions",
            "Edge Softness",
            "Hinge Edge",
            "Handle / Knob Style"
        ]
    );
    assert!(primary.iter().any(|control| {
        control.id == "handle_knob_style"
            && matches!(control.kind, ControlKind::ContinuousAxis { .. })
    }));

    let strategy_labels = profile
        .candidate_strategies
        .iter()
        .map(|strategy| strategy.label.as_str())
        .collect::<Vec<_>>();
    assert_eq!(
        strategy_labels,
        vec![
            "Knob Panel",
            "Pull Handle Panel",
            "Wide Handled Panel",
            "Tall Handled Panel",
            "Clean Handled Panel",
            "Heavy Edge Handled Panel",
        ]
    );
    let control_copy = primary
        .iter()
        .map(|control| control.label.as_str())
        .collect::<Vec<_>>()
        .join(" ");
    let strategy_copy = strategy_labels.join(" ");
    let visible_copy = [
        family.display_name.as_str(),
        family.summary.as_str(),
        control_copy.as_str(),
        strategy_copy.as_str(),
    ]
    .join(" ")
    .to_ascii_lowercase();
    for forbidden in [
        "door",
        "open",
        "close",
        "uv",
        "texture",
        "material",
        "rigging",
        "rigged",
        "animation",
        "game-ready",
        "latch",
        "frame",
        "inset",
    ] {
        assert!(
            !visible_copy.contains(forbidden),
            "Handled Panel copy must not claim {forbidden}: {visible_copy}"
        );
    }
    for raw_term in [
        "handle_knob",
        "placement",
        "zone",
        "module",
        "provider",
        "slot",
    ] {
        assert!(
            !visible_copy.contains(raw_term),
            "Handled Panel copy must not expose raw authoring term {raw_term}: {visible_copy}"
        );
    }
}

#[test]
fn flat_panel_control_endpoints_are_visible() {
    let fixture = flat_panel::fixture_catalog();
    let report = generate_foundry_control_endpoint_visibility_report(&fixture.document, &fixture)
        .expect("endpoint report should generate");

    assert_eq!(report.controls.len(), 2);
    for row in &report.controls {
        assert!(
            matches!(
                row.legibility_class,
                CandidateLegibilityClass::Strong
                    | CandidateLegibilityClass::Clear
                    | CandidateLegibilityClass::SubtleButExplainable
            ),
            "{} endpoint should produce visible geometry: {row:#?}",
            row.control_id
        );
    }
}

#[test]
fn hinged_panel_hinge_edge_endpoint_is_visible() {
    let fixture = flat_panel::hinged_panel_fixture_catalog();
    let report = generate_foundry_control_endpoint_visibility_report(&fixture.document, &fixture)
        .expect("endpoint report should generate");

    assert_eq!(report.controls.len(), 3);
    let hinge_row = report
        .controls
        .iter()
        .find(|row| row.control_id == "hinge_edge_style")
        .expect("hinge edge control should be reported");
    assert!(
        matches!(
            hinge_row.legibility_class,
            CandidateLegibilityClass::Strong
                | CandidateLegibilityClass::Clear
                | CandidateLegibilityClass::SubtleButExplainable
        ),
        "hinge edge endpoint should produce visible clay geometry: {hinge_row:#?}"
    );
}

#[test]
fn handled_panel_handle_knob_endpoint_is_visible_in_pure_clay() {
    let fixture = flat_panel::handled_panel_fixture_catalog();
    let report = generate_foundry_control_endpoint_visibility_report(&fixture.document, &fixture)
        .expect("endpoint report should generate");

    assert_eq!(report.controls.len(), 4);
    let handle_row = report
        .controls
        .iter()
        .find(|row| row.control_id == "handle_knob_style")
        .expect("handle/knob control should be reported");
    assert!(
        matches!(
            handle_row.legibility_class,
            CandidateLegibilityClass::Strong
                | CandidateLegibilityClass::Clear
                | CandidateLegibilityClass::SubtleButExplainable
        ),
        "handle/knob endpoint should produce visible clay geometry: {handle_row:#?}"
    );

    let low = compile_handled_with(&[("handle_knob_style", ControlValue::Scalar(0.0))]);
    let high = compile_handled_with(&[("handle_knob_style", ControlValue::Scalar(1.0))]);
    assert_valid_model(&low);
    assert_valid_model(&high);
    assert_ne!(
        low.build_stamp.artifact_fingerprint, high.build_stamp.artifact_fingerprint,
        "handle/knob style endpoints should compile to visibly different geometry"
    );
    let low_handle = compiled_part_for_role(&low, "handle_knob");
    let high_handle = compiled_part_for_role(&high, "handle_knob");
    let low_height = low_handle.world_mesh.bounds.max[1] - low_handle.world_mesh.bounds.min[1];
    let high_height = high_handle.world_mesh.bounds.max[1] - high_handle.world_mesh.bounds.min[1];
    assert!(
        high_height > low_height * 2.5,
        "handle/knob endpoint should visibly change the handle length"
    );
}

#[test]
fn flat_panel_candidate_ideas_compile_to_distinct_shapes() {
    let variants = [
        (
            "Narrow Panel",
            vec![(
                "proportions",
                ControlValue::Choice("narrow_panel".to_owned()),
            )],
        ),
        (
            "Wide Panel",
            vec![("proportions", ControlValue::Choice("wide_panel".to_owned()))],
        ),
        (
            "Tall Panel",
            vec![("proportions", ControlValue::Choice("tall_panel".to_owned()))],
        ),
        (
            "Short Panel",
            vec![(
                "proportions",
                ControlValue::Choice("short_panel".to_owned()),
            )],
        ),
        (
            "Soft-Edged Panel",
            vec![("edge_softness", ControlValue::Scalar(1.0))],
        ),
        (
            "Sharp Panel",
            vec![("edge_softness", ControlValue::Scalar(0.0))],
        ),
    ];

    let mut fingerprints = BTreeSet::new();
    for (label, overrides) in variants {
        let output = compile_with(&overrides);
        assert_valid_model(&output);
        assert!(
            fingerprints.insert(output.build_stamp.artifact_fingerprint.0.to_hex()),
            "{label} should produce a distinct compiled flat panel"
        );
    }
}

#[test]
fn hinged_panel_candidate_ideas_compile_to_distinct_shapes() {
    let variants = [
        (
            "Clean Hinged Panel",
            vec![("hinge_edge_style", ControlValue::Scalar(0.40))],
        ),
        (
            "Wide Hinged Panel",
            vec![("proportions", ControlValue::Choice("wide_panel".to_owned()))],
        ),
        (
            "Tall Hinged Panel",
            vec![("proportions", ControlValue::Choice("tall_panel".to_owned()))],
        ),
        (
            "Heavy Edge Panel",
            vec![("hinge_edge_style", ControlValue::Scalar(1.0))],
        ),
        (
            "Minimal Hinged Panel",
            vec![
                ("edge_softness", ControlValue::Scalar(0.0)),
                ("hinge_edge_style", ControlValue::Scalar(0.0)),
            ],
        ),
    ];

    let mut fingerprints = BTreeSet::new();
    for (label, overrides) in variants {
        let output = compile_hinged_with(&overrides);
        assert_valid_model(&output);
        assert!(
            fingerprints.insert(output.build_stamp.artifact_fingerprint.0.to_hex()),
            "{label} should produce a distinct compiled hinged panel"
        );
    }
}

#[test]
fn handled_panel_candidate_ideas_compile_to_distinct_shapes() {
    let variants = [
        (
            "Knob Panel",
            vec![("handle_knob_style", ControlValue::Scalar(0.0))],
        ),
        (
            "Pull Handle Panel",
            vec![("handle_knob_style", ControlValue::Scalar(1.0))],
        ),
        (
            "Wide Handled Panel",
            vec![("proportions", ControlValue::Choice("wide_panel".to_owned()))],
        ),
        (
            "Tall Handled Panel",
            vec![("proportions", ControlValue::Choice("tall_panel".to_owned()))],
        ),
        (
            "Clean Handled Panel",
            vec![
                ("edge_softness", ControlValue::Scalar(0.0)),
                ("handle_knob_style", ControlValue::Scalar(0.45)),
            ],
        ),
        (
            "Heavy Edge Handled Panel",
            vec![("hinge_edge_style", ControlValue::Scalar(1.0))],
        ),
    ];

    let mut fingerprints = BTreeSet::new();
    for (label, overrides) in variants {
        let output = compile_handled_with(&overrides);
        assert_valid_model(&output);
        assert!(
            fingerprints.insert(output.build_stamp.artifact_fingerprint.0.to_hex()),
            "{label} should produce a distinct compiled handled panel"
        );
    }
}
