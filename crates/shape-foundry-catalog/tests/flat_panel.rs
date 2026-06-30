#![forbid(unsafe_code)]

use std::{collections::BTreeSet, fs};

use serde::de::DeserializeOwned;
use shape_asset::PartInstanceId;
use shape_compile::{
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

fn compile_with(overrides: &[(&str, ControlValue)]) -> FoundryCompilationOutput {
    let fixture = flat_panel::fixture_catalog();
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
