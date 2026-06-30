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
use shape_foundry_catalog::{
    CatalogCurationState, FoundryFixtureCatalog, box_primitive, built_in_catalog_curation_metadata,
    built_in_fixture_catalogs_with_labels, catalog_curation_metadata_for_slug,
    curated_fixture_catalogs_with_labels,
};
use shape_search::foundry::generate_foundry_control_endpoint_visibility_report;

fn payload<T: DeserializeOwned>(fixture: &FoundryFixtureCatalog, id: &str) -> T {
    serde_json::from_str(&fixture.entries[id].canonical_json).expect("catalog payload decodes")
}

fn family(fixture: &FoundryFixtureCatalog) -> AssetFamilySchema {
    payload(fixture, "box-primitive-family")
}

fn profile(fixture: &FoundryFixtureCatalog) -> CustomizerProfile {
    payload(fixture, "box-primitive-profile")
}

fn compile_with(overrides: &[(&str, ControlValue)]) -> FoundryCompilationOutput {
    let fixture = box_primitive::fixture_catalog();
    let mut document = fixture.document.clone();
    for (control, value) in overrides {
        document
            .control_state
            .insert((*control).to_owned(), value.clone());
    }
    compile_foundry_document(&document, &fixture).expect("box primitive variant compiles")
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
        "Box Primitive validation should pass: {:#?}",
        report.issues
    );
}

#[test]
fn box_primitive_validates_exports_and_has_only_body_role() {
    let output = compile_with(&[]);
    assert_valid_model(&output);
    assert_eq!(role_instances(&output, "body").len(), 1);
    for forbidden_role in ["lid", "trim_band", "feet_or_skids", "surface_detail"] {
        assert!(
            role_instances(&output, forbidden_role).is_empty(),
            "Box Primitive must not expose {forbidden_role}"
        );
    }

    let package_dir = std::env::temp_dir().join(format!(
        "shape-lab-box-primitive-export-{}",
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
fn box_primitive_controls_and_labels_are_honest() {
    let fixture = box_primitive::fixture_catalog();
    let family = family(&fixture);
    let profile = profile(&fixture);

    assert_eq!(fixture.slug, box_primitive::BOX_PRIMITIVE_SLUG);
    assert_eq!(family.id, box_primitive::BOX_PRIMITIVE_FAMILY_ID);
    assert_eq!(family.display_name, "Box Primitive");
    assert_eq!(profile.family_id, box_primitive::BOX_PRIMITIVE_FAMILY_ID);
    assert_eq!(
        profile.style_id.as_deref(),
        Some(box_primitive::BOX_PRIMITIVE_STYLE_ID)
    );
    assert_eq!(
        family
            .part_roles
            .iter()
            .map(|role| role.id.as_str())
            .collect::<Vec<_>>(),
        vec!["body"]
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
    let visible_copy = [
        family.display_name.as_str(),
        family.summary.as_str(),
        strategy_copy.as_str(),
    ]
    .join(" ")
    .to_ascii_lowercase();
    for forbidden in [
        "crate",
        "panel",
        "handle",
        "vent",
        "latch",
        "uv",
        "texture",
        "material",
        "rig",
        "animation",
    ] {
        assert!(
            !visible_copy.contains(forbidden),
            "Box Primitive copy must not claim {forbidden}: {visible_copy}"
        );
    }
}

#[test]
fn box_primitive_candidate_strategies_match_baseline_ideas() {
    let fixture = box_primitive::fixture_catalog();
    let profile = profile(&fixture);
    assert_eq!(
        profile
            .candidate_strategies
            .iter()
            .map(|strategy| strategy.label.as_str())
            .collect::<Vec<_>>(),
        vec![
            "Compact Box",
            "Wide Box",
            "Tall Box",
            "Flat Box",
            "Soft-Edged Box",
            "Sharp Box",
        ]
    );
    assert!(
        profile
            .candidate_strategies
            .iter()
            .all(|strategy| { strategy.label.contains("Box") && !strategy.control_ids.is_empty() })
    );
}

#[test]
fn box_primitive_every_control_endpoint_is_visible() {
    let fixture = box_primitive::fixture_catalog();
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
fn box_primitive_baseline_ideas_compile_to_distinct_boxes() {
    let variants = [
        (
            "Compact Box",
            vec![(
                "proportions",
                ControlValue::Choice("compact_box".to_owned()),
            )],
        ),
        (
            "Wide Box",
            vec![("proportions", ControlValue::Choice("wide_box".to_owned()))],
        ),
        (
            "Tall Box",
            vec![("proportions", ControlValue::Choice("tall_box".to_owned()))],
        ),
        (
            "Flat Box",
            vec![("proportions", ControlValue::Choice("flat_box".to_owned()))],
        ),
        (
            "Soft-Edged Box",
            vec![("edge_softness", ControlValue::Scalar(1.0))],
        ),
        (
            "Sharp Box",
            vec![("edge_softness", ControlValue::Scalar(0.0))],
        ),
    ];

    let mut fingerprints = BTreeSet::new();
    for (label, overrides) in variants {
        let output = compile_with(&overrides);
        assert_valid_model(&output);
        assert!(
            fingerprints.insert(output.build_stamp.artifact_fingerprint.0.to_hex()),
            "{label} should produce a distinct compiled box"
        );
    }
}

#[test]
fn box_primitive_is_the_only_builtin_catalog_profile() {
    let metadata = catalog_curation_metadata_for_slug(box_primitive::BOX_PRIMITIVE_SLUG)
        .expect("box metadata");
    assert_eq!(metadata.state, CatalogCurationState::Usable);
    assert!(metadata.default_novice_visible());

    let all_slugs = built_in_fixture_catalogs_with_labels()
        .into_iter()
        .map(|(_, fixture)| fixture.slug)
        .collect::<Vec<_>>();
    assert_eq!(all_slugs, vec![box_primitive::BOX_PRIMITIVE_SLUG]);

    let curation_slugs = built_in_catalog_curation_metadata()
        .into_iter()
        .map(|metadata| metadata.profile_slug.to_owned())
        .collect::<Vec<_>>();
    assert_eq!(curation_slugs, vec![box_primitive::BOX_PRIMITIVE_SLUG]);

    let novice_slugs = curated_fixture_catalogs_with_labels(false)
        .into_iter()
        .map(|(_, fixture)| fixture.slug)
        .collect::<Vec<_>>();
    assert_eq!(novice_slugs, vec![box_primitive::BOX_PRIMITIVE_SLUG]);

    let preview_slugs = curated_fixture_catalogs_with_labels(true)
        .into_iter()
        .map(|(_, fixture)| fixture.slug)
        .collect::<Vec<_>>();
    assert_eq!(preview_slugs, vec![box_primitive::BOX_PRIMITIVE_SLUG]);
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
