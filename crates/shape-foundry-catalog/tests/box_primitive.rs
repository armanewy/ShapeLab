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
    FoundryCompilationOutput, compile_foundry_document, lid_seam_feature_module_contract,
    trim_band_feature_module_contract, validate_feature_module_contract,
};
use shape_foundry_catalog::{
    CatalogCurationState, FoundryFixtureCatalog, box_primitive, built_in_catalog_curation_metadata,
    built_in_fixture_catalogs_with_labels, catalog_curation_metadata_for_slug,
    curated_fixture_catalogs_with_labels, flat_panel,
};
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
    let fixture = box_primitive::fixture_catalog();
    compile_fixture_with(&fixture, overrides)
}

fn compile_lidded_with(overrides: &[(&str, ControlValue)]) -> FoundryCompilationOutput {
    let fixture = box_primitive::lidded_box_fixture_catalog();
    compile_fixture_with(&fixture, overrides)
}

fn compile_trimmed_with(overrides: &[(&str, ControlValue)]) -> FoundryCompilationOutput {
    let fixture = box_primitive::trimmed_box_fixture_catalog();
    compile_fixture_with(&fixture, overrides)
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
fn box_primitive_without_lid_seam_remains_unchanged() {
    let output = compile_with(&[]);
    assert_valid_model(&output);
    assert_eq!(role_instances(&output, "body").len(), 1);
    assert!(role_instances(&output, "lid_seam").is_empty());
    assert_eq!(
        output
            .catalog
            .customizer_profile
            .controls
            .iter()
            .map(|control| control.label.as_str())
            .collect::<Vec<_>>(),
        vec!["Proportions", "Edge Softness"]
    );
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
fn lidded_box_includes_lid_seam_module_contract() {
    let contract = lid_seam_feature_module_contract();
    let report = validate_feature_module_contract(&contract);
    assert!(
        report.is_valid(),
        "Lid Seam module contract should validate: {report:#?}"
    );
    assert_eq!(contract.module_id, box_primitive::LID_SEAM_MODULE_ID);
    assert_eq!(contract.owns_controls, vec!["lid_height"]);
    assert!(
        contract
            .quality_gates
            .iter()
            .any(|gate| gate.id == "seam-visible-in-pure-clay")
    );
    assert!(
        contract
            .quality_gates
            .iter()
            .any(|gate| gate.id == "not-material-stripe")
    );
}

#[test]
fn lidded_box_validates_exports_and_has_only_lid_seam_feature_role() {
    let output = compile_lidded_with(&[]);
    assert_valid_model(&output);
    assert_eq!(role_instances(&output, "body").len(), 1);
    assert_eq!(role_instances(&output, "lid_seam").len(), 1);
    for forbidden_role in [
        "trim_band",
        "feet_or_skids",
        "panel",
        "handle",
        "latch",
        "vent",
    ] {
        assert!(
            role_instances(&output, forbidden_role).is_empty(),
            "Lidded Box must not expose {forbidden_role}"
        );
    }

    let package_dir = std::env::temp_dir().join(format!(
        "shape-lab-lidded-box-export-{}",
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
fn lidded_box_without_trim_band_remains_unchanged() {
    let output = compile_lidded_with(&[]);
    assert_valid_model(&output);
    assert_eq!(role_instances(&output, "body").len(), 1);
    assert_eq!(role_instances(&output, "lid_seam").len(), 1);
    assert!(role_instances(&output, "trim_band").is_empty());
    assert_eq!(
        output
            .catalog
            .customizer_profile
            .controls
            .iter()
            .map(|control| control.label.as_str())
            .collect::<Vec<_>>(),
        vec!["Proportions", "Edge Softness", "Lid Seam"]
    );
}

#[test]
fn trimmed_box_includes_trim_band_module_contract() {
    let contract = trim_band_feature_module_contract();
    let report = validate_feature_module_contract(&contract);
    assert!(
        report.is_valid(),
        "Trim Band module contract should validate: {report:#?}"
    );
    assert_eq!(contract.module_id, box_primitive::TRIM_BAND_MODULE_ID);
    assert_eq!(contract.owns_controls, vec!["trim_thickness"]);
    assert!(
        contract
            .quality_gates
            .iter()
            .any(|gate| gate.id == "trim-visible-in-pure-clay")
    );
    assert!(
        contract
            .quality_gates
            .iter()
            .any(|gate| gate.id == "trim-not-material-stripe")
    );
}

#[test]
fn trimmed_box_validates_exports_and_has_only_lid_seam_and_trim_band_roles() {
    let output = compile_trimmed_with(&[]);
    assert_valid_model(&output);
    assert_eq!(role_instances(&output, "body").len(), 1);
    assert_eq!(role_instances(&output, "lid_seam").len(), 1);
    assert_eq!(role_instances(&output, "trim_band").len(), 1);
    for forbidden_role in ["feet_or_skids", "panel", "handle", "latch", "vent"] {
        assert!(
            role_instances(&output, forbidden_role).is_empty(),
            "Trimmed Box must not expose {forbidden_role}"
        );
    }

    let package_dir = std::env::temp_dir().join(format!(
        "shape-lab-trimmed-box-export-{}",
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
fn trimmed_box_controls_and_copy_are_honest() {
    let fixture = box_primitive::trimmed_box_fixture_catalog();
    let family = family(&fixture);
    let profile = profile(&fixture);

    assert_eq!(fixture.slug, box_primitive::TRIMMED_BOX_SLUG);
    assert_eq!(family.id, box_primitive::TRIMMED_BOX_FAMILY_ID);
    assert_eq!(family.display_name, "Trimmed Box");
    assert_eq!(
        family.summary,
        "A simple lidded box with a visible trim band."
    );
    assert_eq!(profile.family_id, box_primitive::TRIMMED_BOX_FAMILY_ID);
    assert_eq!(
        profile.style_id.as_deref(),
        Some(box_primitive::TRIMMED_BOX_STYLE_ID)
    );
    assert_eq!(
        family
            .part_roles
            .iter()
            .map(|role| role.id.as_str())
            .collect::<Vec<_>>(),
        vec!["body", "lid_seam", "trim_band"]
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
        vec!["Proportions", "Edge Softness", "Lid Seam", "Trim Thickness"]
    );

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
            "Clean Trimmed Box",
            "Reinforced Trimmed Box",
            "Compact Trimmed Box",
            "Wide Trimmed Box",
            "Low Trim Box",
            "Soft Trimmed Box",
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
        "crate",
        "case",
        "feet",
        "skid",
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
            "Trimmed Box copy must not claim {forbidden}: {visible_copy}"
        );
    }
}

#[test]
fn trimmed_box_trim_thickness_endpoint_is_visible_in_pure_clay() {
    let fixture = box_primitive::trimmed_box_fixture_catalog();
    let report = generate_foundry_control_endpoint_visibility_report(&fixture.document, &fixture)
        .expect("endpoint report should generate");

    assert_eq!(report.controls.len(), 4);
    let trim_row = report
        .controls
        .iter()
        .find(|row| row.control_id == "trim_thickness")
        .expect("Trim Thickness endpoint row");
    assert!(
        matches!(
            trim_row.legibility_class,
            CandidateLegibilityClass::Strong
                | CandidateLegibilityClass::Clear
                | CandidateLegibilityClass::SubtleButExplainable
        ),
        "Trim Thickness endpoint should produce visible geometry: {trim_row:#?}"
    );
}

#[test]
fn trimmed_box_candidate_ideas_compile_to_distinct_shapes() {
    let variants = [
        (
            "Clean Trimmed Box",
            vec![("trim_thickness", ControlValue::Scalar(0.0))],
        ),
        (
            "Reinforced Trimmed Box",
            vec![("trim_thickness", ControlValue::Scalar(1.0))],
        ),
        (
            "Compact Trimmed Box",
            vec![(
                "proportions",
                ControlValue::Choice("compact_box".to_owned()),
            )],
        ),
        (
            "Wide Trimmed Box",
            vec![("proportions", ControlValue::Choice("wide_box".to_owned()))],
        ),
        (
            "Low Trim Box",
            vec![("lid_height", ControlValue::Scalar(0.0))],
        ),
        (
            "Soft Trimmed Box",
            vec![("edge_softness", ControlValue::Scalar(1.0))],
        ),
    ];

    let mut fingerprints = BTreeSet::new();
    for (label, overrides) in variants {
        let output = compile_trimmed_with(&overrides);
        assert_valid_model(&output);
        assert!(
            fingerprints.insert(output.build_stamp.artifact_fingerprint.0.to_hex()),
            "{label} should produce a distinct compiled trimmed box"
        );
    }
}

#[test]
fn lidded_box_controls_and_copy_are_honest() {
    let fixture = box_primitive::lidded_box_fixture_catalog();
    let family = family(&fixture);
    let profile = profile(&fixture);

    assert_eq!(fixture.slug, box_primitive::LIDDED_BOX_SLUG);
    assert_eq!(family.id, box_primitive::LIDDED_BOX_FAMILY_ID);
    assert_eq!(family.display_name, "Lidded Box");
    assert_eq!(family.summary, "A simple box with a visible lid seam.");
    assert_eq!(profile.family_id, box_primitive::LIDDED_BOX_FAMILY_ID);
    assert_eq!(
        profile.style_id.as_deref(),
        Some(box_primitive::LIDDED_BOX_STYLE_ID)
    );
    assert_eq!(
        family
            .part_roles
            .iter()
            .map(|role| role.id.as_str())
            .collect::<Vec<_>>(),
        vec!["body", "lid_seam"]
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
        vec!["Proportions", "Edge Softness", "Lid Seam"]
    );

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
            "Low Lid Box",
            "Raised Lid Box",
            "Compact Lidded Box",
            "Wide Lidded Box",
            "Flat Storage Box",
            "Soft-Edged Lidded Box",
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
        "crate",
        "case",
        "trim",
        "feet",
        "skid",
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
            "Lidded Box copy must not claim {forbidden}: {visible_copy}"
        );
    }
}

#[test]
fn lidded_box_lid_seam_endpoint_is_visible_in_pure_clay() {
    let fixture = box_primitive::lidded_box_fixture_catalog();
    let report = generate_foundry_control_endpoint_visibility_report(&fixture.document, &fixture)
        .expect("endpoint report should generate");

    assert_eq!(report.controls.len(), 3);
    let lid_row = report
        .controls
        .iter()
        .find(|row| row.control_id == "lid_height")
        .expect("Lid Seam endpoint row");
    assert!(
        matches!(
            lid_row.legibility_class,
            CandidateLegibilityClass::Strong
                | CandidateLegibilityClass::Clear
                | CandidateLegibilityClass::SubtleButExplainable
        ),
        "Lid Seam endpoint should be visible: {lid_row:#?}"
    );
}

#[test]
fn lidded_box_candidate_ideas_compile_to_distinct_shapes() {
    let variants = [
        (
            "Low Lid Box",
            vec![("lid_height", ControlValue::Scalar(0.0))],
        ),
        (
            "Raised Lid Box",
            vec![("lid_height", ControlValue::Scalar(1.0))],
        ),
        (
            "Compact Lidded Box",
            vec![(
                "proportions",
                ControlValue::Choice("compact_box".to_owned()),
            )],
        ),
        (
            "Wide Lidded Box",
            vec![("proportions", ControlValue::Choice("wide_box".to_owned()))],
        ),
        (
            "Flat Storage Box",
            vec![("proportions", ControlValue::Choice("flat_box".to_owned()))],
        ),
        (
            "Soft-Edged Lidded Box",
            vec![("edge_softness", ControlValue::Scalar(1.0))],
        ),
    ];

    let mut fingerprints = BTreeSet::new();
    for (label, overrides) in variants {
        let output = compile_lidded_with(&overrides);
        assert_valid_model(&output);
        assert!(
            fingerprints.insert(output.build_stamp.artifact_fingerprint.0.to_hex()),
            "{label} should produce a distinct compiled lidded box"
        );
    }
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
fn two_kernel_profiles_are_the_builtin_catalog_profiles() {
    let box_metadata = catalog_curation_metadata_for_slug(box_primitive::BOX_PRIMITIVE_SLUG)
        .expect("box metadata");
    assert_eq!(box_metadata.state, CatalogCurationState::Usable);
    assert!(box_metadata.default_novice_visible());

    let lidded_metadata = catalog_curation_metadata_for_slug(box_primitive::LIDDED_BOX_SLUG)
        .expect("lidded box metadata");
    assert_eq!(lidded_metadata.state, CatalogCurationState::Usable);
    assert!(lidded_metadata.default_novice_visible());
    assert!(
        lidded_metadata
            .note
            .contains("Box Primitive plus one visible Lid Seam")
    );

    let flat_panel_metadata =
        catalog_curation_metadata_for_slug(flat_panel::FLAT_PANEL_PRIMITIVE_SLUG)
            .expect("flat panel metadata");
    assert_eq!(flat_panel_metadata.state, CatalogCurationState::Usable);
    assert!(flat_panel_metadata.default_novice_visible());

    let hinged_panel_metadata = catalog_curation_metadata_for_slug(flat_panel::HINGED_PANEL_SLUG)
        .expect("hinged panel metadata");
    assert_eq!(hinged_panel_metadata.state, CatalogCurationState::Usable);
    assert!(hinged_panel_metadata.default_novice_visible());
    assert!(hinged_panel_metadata.note.contains("Hinge Edge"));

    let handled_panel_metadata = catalog_curation_metadata_for_slug(flat_panel::HANDLED_PANEL_SLUG)
        .expect("handled panel metadata");
    assert_eq!(handled_panel_metadata.state, CatalogCurationState::Usable);
    assert!(handled_panel_metadata.default_novice_visible());
    assert!(handled_panel_metadata.note.contains("Handle / Knob"));

    let all_slugs = built_in_fixture_catalogs_with_labels()
        .into_iter()
        .map(|(_, fixture)| fixture.slug)
        .collect::<Vec<_>>();
    assert_eq!(
        all_slugs,
        vec![
            box_primitive::BOX_PRIMITIVE_SLUG,
            box_primitive::LIDDED_BOX_SLUG,
            flat_panel::FLAT_PANEL_PRIMITIVE_SLUG,
            flat_panel::HINGED_PANEL_SLUG,
            flat_panel::HANDLED_PANEL_SLUG,
        ]
    );

    let curation_slugs = built_in_catalog_curation_metadata()
        .into_iter()
        .map(|metadata| metadata.profile_slug.to_owned())
        .collect::<Vec<_>>();
    assert_eq!(
        curation_slugs,
        vec![
            box_primitive::BOX_PRIMITIVE_SLUG,
            box_primitive::LIDDED_BOX_SLUG,
            flat_panel::FLAT_PANEL_PRIMITIVE_SLUG,
            flat_panel::HINGED_PANEL_SLUG,
            flat_panel::HANDLED_PANEL_SLUG,
        ]
    );

    let novice_slugs = curated_fixture_catalogs_with_labels(false)
        .into_iter()
        .map(|(_, fixture)| fixture.slug)
        .collect::<Vec<_>>();
    assert_eq!(
        novice_slugs,
        vec![
            box_primitive::BOX_PRIMITIVE_SLUG,
            box_primitive::LIDDED_BOX_SLUG,
            flat_panel::FLAT_PANEL_PRIMITIVE_SLUG,
            flat_panel::HINGED_PANEL_SLUG,
            flat_panel::HANDLED_PANEL_SLUG,
        ]
    );

    let preview_slugs = curated_fixture_catalogs_with_labels(true)
        .into_iter()
        .map(|(_, fixture)| fixture.slug)
        .collect::<Vec<_>>();
    assert_eq!(
        preview_slugs,
        vec![
            box_primitive::BOX_PRIMITIVE_SLUG,
            box_primitive::LIDDED_BOX_SLUG,
            flat_panel::FLAT_PANEL_PRIMITIVE_SLUG,
            flat_panel::HINGED_PANEL_SLUG,
            flat_panel::HANDLED_PANEL_SLUG,
        ]
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
