#![forbid(unsafe_code)]

use std::{collections::BTreeSet, fs};

use serde::de::DeserializeOwned;
use shape_compile::{
    export::{verify_model_package, write_model_package},
    validation::{ValidationLimits, validate_model, validation_config_from_recipe_with_limits},
};
use shape_family::AssetFamilySchema;
use shape_family_compile::StyleImplementation;
use shape_foundry::{
    CandidateLegibilityClass, ControlKind, ControlValue, CustomizerProfile,
    FoundryCompilationOutput, VariationIntent, compile_foundry_document,
};
use shape_foundry_catalog::{FoundryFixtureCatalog, cargo_case, scifi_crate};
use shape_search::foundry::{
    FoundryCandidateMode, FoundryCandidateRequest, generate_foundry_candidate_plans,
};

fn payload<T: DeserializeOwned>(fixture: &FoundryFixtureCatalog, id: &str) -> T {
    serde_json::from_str(&fixture.entries[id].canonical_json).expect("catalog payload decodes")
}

fn family_for(fixture: &FoundryFixtureCatalog, slug: &str) -> AssetFamilySchema {
    payload(fixture, &format!("{slug}-family"))
}

fn profile_for(fixture: &FoundryFixtureCatalog, slug: &str) -> CustomizerProfile {
    payload(fixture, &format!("{slug}-profile"))
}

fn style_impl_for(fixture: &FoundryFixtureCatalog, slug: &str) -> StyleImplementation {
    payload(fixture, &format!("{slug}-style-impl"))
}

fn compile_with(overrides: &[(&str, ControlValue)]) -> FoundryCompilationOutput {
    let fixture = scifi_crate::fixture_catalog();
    let mut document = fixture.document.clone();
    for (control, value) in overrides {
        document
            .control_state
            .insert((*control).to_owned(), value.clone());
    }
    compile_foundry_document(&document, &fixture).expect("sci-fi cargo case variant compiles")
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
        "Sci-Fi Cargo Case validation should pass: {:#?}",
        report.issues
    );
    assert_eq!(report.metrics.accidental_intersection_count, 0);
}

#[test]
fn scifi_crate_resolves_to_cargo_case_family_with_stable_profile_slug() {
    let fixture = scifi_crate::fixture_catalog();
    let family = family_for(&fixture, cargo_case::SCI_FI_CRATE_SLUG);
    let profile = profile_for(&fixture, cargo_case::SCI_FI_CRATE_SLUG);
    let style_impl = style_impl_for(&fixture, cargo_case::SCI_FI_CRATE_SLUG);

    assert_eq!(fixture.slug, "sci-fi-crate");
    assert_eq!(family.id, cargo_case::CARGO_CASE_FAMILY_ID);
    assert_eq!(profile.family_id, cargo_case::CARGO_CASE_FAMILY_ID);
    assert_eq!(
        profile.style_id.as_deref(),
        Some(cargo_case::SCI_FI_INDUSTRIAL_CASE_STYLE_ID)
    );
    assert_eq!(style_impl.family_id, cargo_case::CARGO_CASE_FAMILY_ID);
    assert_eq!(
        style_impl.style_kit_id,
        cargo_case::SCI_FI_INDUSTRIAL_CASE_STYLE_ID
    );
    assert_eq!(
        style_impl.default_role_providers["corner_guards"],
        "chamfered_armor_block"
    );
    assert_eq!(
        style_impl.default_role_providers["vents"],
        "dense_grille_vents"
    );
}

#[test]
fn scifi_and_clean_utility_share_family_grammar_but_have_distinct_defaults() {
    let clean = cargo_case::clean_utility_fixture_catalog();
    let scifi = scifi_crate::fixture_catalog();
    let clean_family = family_for(&clean, cargo_case::CLEAN_UTILITY_CASE_SLUG);
    let scifi_family = family_for(&scifi, cargo_case::SCI_FI_CRATE_SLUG);
    let clean_style = style_impl_for(&clean, cargo_case::CLEAN_UTILITY_CASE_SLUG);
    let scifi_style = style_impl_for(&scifi, cargo_case::SCI_FI_CRATE_SLUG);

    assert_eq!(clean_family.part_roles, scifi_family.part_roles);
    assert_eq!(clean_family.parameter_slots, scifi_family.parameter_slots);
    assert_ne!(
        clean.document.control_state["panel_complexity"],
        scifi.document.control_state["panel_complexity"]
    );
    assert_ne!(
        clean.document.control_state["vent_density"],
        scifi.document.control_state["vent_density"]
    );
    assert_ne!(
        clean_style.default_role_providers["corner_guards"],
        scifi_style.default_role_providers["corner_guards"]
    );
    assert_ne!(
        clean_style.default_role_providers["vents"],
        scifi_style.default_role_providers["vents"]
    );
    assert_eq!(
        cargo_case::semantic_clay_assignments()
            .iter()
            .map(|assignment| assignment.display_label)
            .collect::<Vec<_>>(),
        vec![
            "Primary Mass",
            "Secondary Panels",
            "Structural Trim",
            "Recesses / Vents",
            "Fasteners / Detail",
        ]
    );
}

#[test]
fn scifi_cargo_case_controls_and_strategies_are_product_safe() {
    let fixture = scifi_crate::fixture_catalog();
    let profile = profile_for(&fixture, cargo_case::SCI_FI_CRATE_SLUG);
    let primary = profile
        .controls
        .iter()
        .filter(|control| control.primary && control.visible)
        .collect::<Vec<_>>();

    assert_eq!(
        primary
            .iter()
            .map(|control| control.label.as_str())
            .collect::<Vec<_>>(),
        vec![
            "Overall Proportions",
            "Structural Heft",
            "Panel Complexity",
            "Handle Style",
            "Vent Density",
            "Trim Style",
            "Detail Density",
        ]
    );
    assert_eq!(primary.len(), 7);
    assert!(primary.iter().all(|control| {
        matches!(
            control.kind,
            ControlKind::ContinuousAxis { .. } | ControlKind::ChoiceGallery { .. }
        )
    }));
    assert_eq!(
        profile
            .candidate_strategies
            .iter()
            .map(|strategy| strategy.label.as_str())
            .collect::<Vec<_>>(),
        vec![
            "Light Industrial",
            "Reinforced Cargo",
            "Compact Vented",
            "Wide Equipment Case",
            "Minimal Industrial",
            "Detailed Utility Case",
        ]
    );
}

#[test]
fn scifi_cargo_case_exports_clean_and_has_no_bespoke_role_vocabulary() {
    let output = compile_with(&[]);
    assert_valid_model(&output);

    let role_set = output
        .recipe
        .instances
        .values()
        .flat_map(|instance| instance.tags.iter())
        .filter_map(|tag| tag.strip_prefix("role:"))
        .collect::<BTreeSet<_>>();
    for role in [
        "body",
        "lid",
        "panel_fields",
        "edge_trim",
        "corner_guards",
        "base_feet_or_skids",
        "handles",
        "vents",
        "fasteners",
    ] {
        assert!(role_set.contains(role), "missing Cargo Case role {role}");
    }
    for old_role in ["panel", "handle", "trim", "fastener"] {
        assert!(
            !role_set.contains(old_role),
            "old bespoke role {old_role} must not remain"
        );
    }

    let package_dir = std::env::temp_dir().join(format!(
        "shape-lab-scifi-cargo-case-export-{}",
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
fn scifi_cargo_case_candidates_are_visibly_distinct() {
    let fixture = scifi_crate::fixture_catalog();
    let output =
        generate_foundry_candidate_plans(&fixture.document, &fixture, &candidate_request(141))
            .expect("Sci-Fi Cargo Case candidates should generate");

    assert!(output.candidates.len() >= 4);
    let unique_signatures = output
        .candidates
        .iter()
        .map(|candidate| candidate.changed_controls.join("|"))
        .collect::<BTreeSet<_>>();
    assert!(unique_signatures.len() >= 4);
    assert!(output.candidates.iter().all(|candidate| {
        candidate.conformance.accepted
            && candidate.variation_metadata.visible_delta.shape_delta_score > 0.0
            && candidate.variation_metadata.visible_delta.legibility_class
                != CandidateLegibilityClass::TooSubtle
    }));
}

#[test]
fn scifi_part_groups_use_cargo_case_roles_and_controls() {
    let groups = scifi_crate::part_group_descriptors();
    assert_eq!(
        groups
            .iter()
            .map(|group| group.group_id.as_str())
            .collect::<Vec<_>>(),
        vec![
            "body",
            "panels",
            "vents",
            "handles",
            "edge-trim",
            "fasteners",
        ]
    );
    assert_eq!(
        group_controls(&groups, "body"),
        vec!["overall_proportions", "structural_heft"]
    );
    assert_eq!(group_controls(&groups, "panels"), vec!["panel_complexity"]);
    assert_eq!(group_controls(&groups, "vents"), vec!["vent_density"]);
    assert_eq!(group_roles(&groups, "panels"), vec!["panel_fields"]);
    assert_eq!(group_roles(&groups, "handles"), vec!["handles"]);
    assert_eq!(group_roles(&groups, "fasteners"), vec!["fasteners"]);
}

#[test]
fn scifi_surface_evidence_boundary_is_documented() {
    let docs = [
        include_str!("../../../docs/foundry-catalog/scifi_crate.md"),
        include_str!("../../../docs/foundry-catalog/cargo_case.md"),
        include_str!("../../../docs/SCIFI_INDUSTRIAL_CARGO_CASE_PROFILE_REPORT.md"),
        include_str!("../../../docs/SCIFI_CRATE_REGRESSION_ROLE.md"),
    ]
    .join("\n");
    let lower = docs
        .to_ascii_lowercase()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ");
    assert!(lower.contains("cargo case family"));
    assert!(lower.contains("sci-fi industrial style/profile"));
    assert!(lower.contains("advanced regression profile"));
    assert!(lower.contains("not the flagship"));
    assert!(lower.contains("simple crate is the new novice baseline proof"));
    assert!(lower.contains("cargo case is the advanced equipment-case proof"));
    assert!(lower.contains("material-look preview"));
    assert!(lower.contains("preview-only"));
    assert!(lower.contains("stale"));
    assert!(lower.contains("game-ready-static-prop --profile sci-fi-crate"));
    assert!(lower.contains("game_ready"));
    assert!(lower.contains("false"));
    for forbidden in [
        "uv/texturing support is approved",
        "material editor is supported",
        "full game-ready status passes",
        "rigging is supported",
        "animation is supported",
    ] {
        assert!(
            !lower.contains(forbidden),
            "Sci-Fi Cargo Case docs must not overclaim {forbidden}"
        );
    }
}

#[test]
fn scifi_control_endpoints_change_geometry() {
    let fixture = scifi_crate::fixture_catalog();
    let profile = profile_for(&fixture, cargo_case::SCI_FI_CRATE_SLUG);
    for control in profile
        .controls
        .iter()
        .filter(|control| control.primary && control.visible)
    {
        match &control.kind {
            ControlKind::ContinuousAxis { .. } => {
                let interval = &control.domain.continuous_intervals[0];
                assert_endpoint_difference(
                    &control.id,
                    ControlValue::Scalar(interval.minimum),
                    ControlValue::Scalar(interval.maximum),
                );
            }
            ControlKind::ChoiceGallery { options } => {
                assert_endpoint_difference(
                    &control.id,
                    ControlValue::Choice(options.first().expect("first option").value.clone()),
                    ControlValue::Choice(options.last().expect("last option").value.clone()),
                );
            }
            ControlKind::IntegerStepper { .. }
            | ControlKind::Toggle { .. }
            | ControlKind::ProviderGallery { .. } => {
                panic!("unexpected primary control kind")
            }
        }
    }
}

fn group_controls(groups: &[shape_foundry::FoundryPartGroupDescriptor], id: &str) -> Vec<String> {
    groups
        .iter()
        .find(|group| group.group_id == id)
        .unwrap_or_else(|| panic!("missing group {id}"))
        .bound_control_ids
        .clone()
}

fn group_roles(groups: &[shape_foundry::FoundryPartGroupDescriptor], id: &str) -> Vec<String> {
    groups
        .iter()
        .find(|group| group.group_id == id)
        .unwrap_or_else(|| panic!("missing group {id}"))
        .bound_provider_roles
        .clone()
}

fn candidate_request(seed: u64) -> FoundryCandidateRequest {
    FoundryCandidateRequest {
        seed,
        proposal_count: 72,
        result_count: 6,
        mode: FoundryCandidateMode::Explore,
        strategy_id: None,
        preference_profile: None,
        variation_intent: VariationIntent::whole_asset_shape(),
    }
}

fn assert_endpoint_difference(control_id: &str, first: ControlValue, second: ControlValue) {
    let first = compile_with(&[(control_id, first)]);
    let second = compile_with(&[(control_id, second)]);
    assert_ne!(
        first.build_stamp.geometry_input_fingerprint, second.build_stamp.geometry_input_fingerprint,
        "{control_id} endpoints should produce different geometry input"
    );
}
