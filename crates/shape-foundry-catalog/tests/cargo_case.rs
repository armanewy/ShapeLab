#![forbid(unsafe_code)]

use std::{collections::BTreeSet, fs};

use serde::de::DeserializeOwned;
use shape_asset::{GeometrySource, PartInstanceId};
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
use shape_foundry_catalog::{FoundryFixtureCatalog, cargo_case};
use shape_search::foundry::{
    FoundryCandidateMode, FoundryCandidateRequest, generate_foundry_candidate_plans,
};

fn payload<T: DeserializeOwned>(fixture: &FoundryFixtureCatalog, id: &str) -> T {
    serde_json::from_str(&fixture.entries[id].canonical_json).expect("catalog payload decodes")
}

fn family(fixture: &FoundryFixtureCatalog) -> AssetFamilySchema {
    payload(fixture, "cargo-case-base-family")
}

fn clean_family(fixture: &FoundryFixtureCatalog) -> AssetFamilySchema {
    payload(fixture, "clean-utility-case-family")
}

fn profile(fixture: &FoundryFixtureCatalog) -> CustomizerProfile {
    payload(fixture, "cargo-case-base-profile")
}

fn clean_profile(fixture: &FoundryFixtureCatalog) -> CustomizerProfile {
    payload(fixture, "clean-utility-case-profile")
}

fn style_impl(fixture: &FoundryFixtureCatalog) -> StyleImplementation {
    payload(fixture, "cargo-case-base-style-impl")
}

fn clean_style_impl(fixture: &FoundryFixtureCatalog) -> StyleImplementation {
    payload(fixture, "clean-utility-case-style-impl")
}

fn compile_with(overrides: &[(&str, ControlValue)]) -> FoundryCompilationOutput {
    let fixture = cargo_case::fixture_catalog();
    let mut document = fixture.document.clone();
    for (control, value) in overrides {
        document
            .control_state
            .insert((*control).to_owned(), value.clone());
    }
    compile_foundry_document(&document, &fixture).expect("cargo case variant compiles")
}

fn compile_clean_with(overrides: &[(&str, ControlValue)]) -> FoundryCompilationOutput {
    let fixture = cargo_case::clean_utility_fixture_catalog();
    let mut document = fixture.document.clone();
    for (control, value) in overrides {
        document
            .control_state
            .insert((*control).to_owned(), value.clone());
    }
    compile_foundry_document(&document, &fixture).expect("clean utility case variant compiles")
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
        "Cargo Case validation should pass: {:#?}",
        report.issues
    );
    assert_eq!(report.metrics.accidental_intersection_count, 0);
}

#[test]
fn cargo_case_validates_and_exports_cleanly() {
    let output = compile_with(&[]);
    assert_valid_model(&output);
    assert!(mesh_triangle_count(&output) > 0);

    let package_dir = std::env::temp_dir().join(format!(
        "shape-lab-cargo-case-export-{}",
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
fn cargo_case_required_and_optional_roles_are_declared() {
    let fixture = cargo_case::fixture_catalog();
    let family = family(&fixture);

    let required = family
        .part_roles
        .iter()
        .filter(|role| role.required)
        .map(|role| role.id.as_str())
        .collect::<Vec<_>>();
    assert_eq!(
        required,
        vec![
            "body",
            "lid",
            "panel_fields",
            "edge_trim",
            "corner_guards",
            "base_feet_or_skids",
        ]
    );

    let optional = family
        .part_roles
        .iter()
        .filter(|role| !role.required)
        .map(|role| role.id.as_str())
        .collect::<BTreeSet<_>>();
    for role in [
        "handles",
        "latches",
        "vents",
        "fasteners",
        "reinforcement_bands",
        "utility_rails",
        "side_grilles",
        "label_plate_geometry",
        "hinge_or_closure_detail",
    ] {
        assert!(optional.contains(role), "missing optional role {role}");
    }
}

#[test]
fn cargo_case_primary_controls_are_product_safe_and_visible() {
    let fixture = cargo_case::fixture_catalog();
    let profile = profile(&fixture);
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
    for control in primary {
        let lower = control.label.to_ascii_lowercase();
        for forbidden in ["scalar", "provider id", "semantic id", "operation id"] {
            assert!(
                !lower.contains(forbidden),
                "{} must stay product safe",
                control.label
            );
        }
    }
}

#[test]
fn cargo_case_provider_inventory_covers_required_options() {
    let fixture = cargo_case::fixture_catalog();
    let style_impl = style_impl(&fixture);
    let provider_ids = style_impl
        .prototypes
        .keys()
        .map(String::as_str)
        .collect::<BTreeSet<_>>();

    for provider in [
        "flush_grip_handle",
        "side_rail_handle",
        "cargo_bar_handle",
        "inset_latch_handle",
        "none_sparse_vents",
        "standard_grille_vents",
        "dense_grille_vents",
        "side_vent_bay",
        "clean_edge_trim",
        "utility_rail_trim",
        "reinforced_edge_trim",
        "industrial_band_trim",
        "clean_panel",
        "shallow_recessed_panel",
        "deep_framed_panel",
        "minimal_corner_cap",
        "block_corner_guard",
        "chamfered_armor_block",
        "low_fasteners",
        "medium_fasteners",
        "high_fasteners",
    ] {
        assert!(
            provider_ids.contains(provider),
            "missing provider {provider}"
        );
    }
}

#[test]
fn cargo_case_optional_handle_and_vent_options_attach_cleanly() {
    for handle in ["flush_grip", "side_rail", "cargo_bar", "inset_latch_handle"] {
        let output = compile_with(&[("handle_style", ControlValue::Choice(handle.to_owned()))]);
        assert_valid_model(&output);
        let handles = role_instances(&output, "handles");
        assert!(
            handles.len() >= 3,
            "{handle} should include visible handle geometry"
        );
        assert!(
            handles
                .iter()
                .all(|instance| instance_sits_near_body_front(&output, instance)),
            "{handle} handles must stay attached to the case body"
        );
    }

    for vent in [
        "none_sparse",
        "standard_grille",
        "dense_grille",
        "side_vent_bay",
    ] {
        let output = compile_with(&[("vent_density", ControlValue::Choice(vent.to_owned()))]);
        assert_valid_model(&output);
        let vents = role_instances(&output, "vents");
        assert!(
            !vents.is_empty(),
            "{vent} should produce readable vent geometry"
        );
        assert!(
            vents
                .iter()
                .all(|instance| instance_sits_near_body_front(&output, instance)),
            "{vent} vents must stay attached to the case body"
        );
    }
}

#[test]
fn cargo_case_every_control_endpoint_changes_geometry() {
    let fixture = cargo_case::fixture_catalog();
    let profile = profile(&fixture);
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
            ControlKind::IntegerStepper { .. } => {
                let values = control
                    .domain
                    .discrete_values
                    .iter()
                    .filter_map(|value| match value {
                        ControlValue::Integer(value) => Some(*value),
                        _ => None,
                    })
                    .collect::<Vec<_>>();
                assert_endpoint_difference(
                    &control.id,
                    ControlValue::Integer(*values.first().expect("integer minimum")),
                    ControlValue::Integer(*values.last().expect("integer maximum")),
                );
            }
            ControlKind::ChoiceGallery { options } => {
                assert!(
                    options
                        .iter()
                        .all(|option| !option.preview.preview_id.is_empty())
                );
                assert_endpoint_difference(
                    &control.id,
                    ControlValue::Choice(options.first().expect("first option").value.clone()),
                    ControlValue::Choice(options.last().expect("last option").value.clone()),
                );
            }
            ControlKind::Toggle { .. } | ControlKind::ProviderGallery { .. } => {
                panic!("unexpected primary control kind")
            }
        }
    }
}

#[test]
fn cargo_case_explore_candidates_are_visibly_distinct() {
    let fixture = cargo_case::fixture_catalog();
    let output =
        generate_foundry_candidate_plans(&fixture.document, &fixture, &candidate_request(74))
            .expect("Cargo Case candidates should generate");

    assert!(
        output.candidates.len() >= 4,
        "expected at least four Cargo Case candidates; diagnostics: {}",
        output.diagnostics.human_summary
    );
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
fn cargo_case_clay_preview_metadata_is_preview_only() {
    assert_eq!(cargo_case::pure_clay_gray_value(), 0.68);
    let assignments = cargo_case::semantic_clay_assignments();
    assert!(assignments.len() >= 5);
    assert!(
        assignments
            .iter()
            .all(|assignment| (0.0..=1.0).contains(&assignment.neutral_gray_value))
    );
    assert!(
        assignments
            .iter()
            .all(|assignment| assignment.applies_to_candidates)
    );
    assert!(
        assignments
            .iter()
            .any(|assignment| assignment.role_or_part_group.contains("vents"))
    );

    let docs = [
        include_str!("../../../docs/foundry-catalog/cargo_case.md"),
        include_str!("../../../docs/CARGO_CASE_BASE_FAMILY_V1_REPORT.md"),
    ]
    .join("\n");
    let lower = docs.to_ascii_lowercase();
    for forbidden in [
        "uv/texturing support is approved",
        "material editor is supported",
        "texture maps are supported",
        "decals are supported",
    ] {
        assert!(
            !lower.contains(forbidden),
            "Cargo Case docs must not overclaim {forbidden}"
        );
    }
}

#[test]
fn cargo_case_base_is_not_plain_rounded_box_with_tiny_attachments() {
    let output = compile_with(&[]);
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
        assert!(role_set.contains(role), "missing visible role {role}");
    }
    assert!(output.recipe.instances.len() >= 20);
    assert!(role_instances(&output, "corner_guards").len() >= 4);
    assert!(role_instances(&output, "base_feet_or_skids").len() >= 2);
    assert!(mesh_triangle_count(&output) > 300);
}

#[test]
fn clean_utility_case_uses_cargo_case_family_without_bespoke_fork() {
    let base = cargo_case::fixture_catalog();
    let clean = cargo_case::clean_utility_fixture_catalog();
    let base_family = family(&base);
    let clean_family = clean_family(&clean);
    let clean_profile = clean_profile(&clean);
    let clean_style_impl = clean_style_impl(&clean);

    assert_eq!(clean.slug, cargo_case::CLEAN_UTILITY_CASE_SLUG);
    assert_eq!(clean_family.id, cargo_case::CARGO_CASE_FAMILY_ID);
    assert_eq!(clean_profile.family_id, cargo_case::CARGO_CASE_FAMILY_ID);
    assert_eq!(
        clean_profile.style_id.as_deref(),
        Some(cargo_case::CLEAN_UTILITY_CASE_STYLE_ID)
    );
    assert_eq!(clean_style_impl.family_id, cargo_case::CARGO_CASE_FAMILY_ID);
    assert_eq!(
        clean_style_impl.style_kit_id,
        cargo_case::CLEAN_UTILITY_CASE_STYLE_ID
    );
    assert_eq!(base_family.part_roles, clean_family.part_roles);
    assert_eq!(base_family.parameter_slots, clean_family.parameter_slots);
}

#[test]
fn clean_utility_case_defaults_are_cleaner_than_base_cargo_case() {
    let clean = cargo_case::clean_utility_fixture_catalog();
    let base = cargo_case::fixture_catalog();
    let clean_style = clean_style_impl(&clean);

    assert_eq!(
        clean.document.control_state["panel_complexity"],
        ControlValue::Choice("clean_panel".to_owned())
    );
    assert_eq!(
        clean.document.control_state["vent_density"],
        ControlValue::Choice("none_sparse".to_owned())
    );
    assert_eq!(
        clean.document.control_state["trim_style"],
        ControlValue::Choice("clean".to_owned())
    );
    assert_eq!(
        clean.document.control_state["detail_density"],
        ControlValue::Choice("low_detail".to_owned())
    );
    assert_eq!(
        clean.document.control_state["handle_style"],
        ControlValue::Choice("flush_grip".to_owned())
    );
    assert_ne!(
        clean.document.control_state["vent_density"],
        base.document.control_state["vent_density"]
    );
    assert_eq!(
        clean_style.default_role_providers["corner_guards"],
        "minimal_corner_cap"
    );
    assert_ne!(
        clean_style.default_role_providers["handles"], "cargo_bar_handle",
        "Clean Utility must not default to cargo-bar handles"
    );
    assert_ne!(
        clean_style.default_role_providers["edge_trim"], "industrial_band_trim",
        "Clean Utility must not default to heavy industrial bands"
    );
}

#[test]
fn clean_utility_case_controls_and_candidates_remain_visible() {
    let fixture = cargo_case::clean_utility_fixture_catalog();
    let profile = clean_profile(&fixture);
    let primary = profile
        .controls
        .iter()
        .filter(|control| control.primary && control.visible)
        .collect::<Vec<_>>();
    assert_eq!(primary.len(), 7);
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
    assert_eq!(
        profile
            .candidate_strategies
            .iter()
            .map(|strategy| strategy.label.as_str())
            .collect::<Vec<_>>(),
        vec![
            "Light Utility",
            "Compact Carry Case",
            "Clean Storage Case",
            "Reinforced Utility",
            "Minimal Field Case",
        ]
    );

    let output =
        generate_foundry_candidate_plans(&fixture.document, &fixture, &candidate_request(91))
            .expect("Clean Utility candidates should generate");
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
fn clean_utility_case_exports_clean_and_reads_in_clay() {
    let output = compile_clean_with(&[]);
    assert_valid_model(&output);
    assert!(mesh_triangle_count(&output) > 250);
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
        assert!(
            !role_instances(&output, role).is_empty(),
            "Clean Utility missing readable role {role}"
        );
    }
    assert!(
        role_instances(&output, "vents").len() <= 2,
        "Clean Utility default should keep vents sparse"
    );
}

#[test]
fn clean_utility_case_keeps_semantic_clay_preview_only() {
    let assignments = cargo_case::semantic_clay_assignments();
    assert!(assignments.len() >= 5);
    assert!(
        assignments
            .iter()
            .all(|assignment| assignment.applies_to_candidates)
    );

    let docs = [
        include_str!("../../../docs/foundry-catalog/cargo_case.md"),
        include_str!("../../../docs/CLEAN_UTILITY_CASE_PROFILE_REPORT.md"),
    ]
    .join("\n");
    let lower = docs.to_ascii_lowercase();
    assert!(lower.contains("clean utility case"));
    assert!(lower.contains("same cargo case family grammar"));
    assert!(lower.contains("pure clay"));
    assert!(lower.contains("semantic clay"));
    for forbidden in [
        "uv/texturing support is approved",
        "material editor is supported",
        "texture maps are supported",
        "decals are supported",
        "logos are supported",
    ] {
        assert!(
            !lower.contains(forbidden),
            "Clean Utility docs must not overclaim {forbidden}"
        );
    }
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
        "{control_id} endpoints must change compiled geometry"
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

fn instance_sits_near_body_front(
    output: &FoundryCompilationOutput,
    instance: &PartInstanceId,
) -> bool {
    let body_half_y = role_rounded_box_half_extents(output, "body")[1];
    let Some((_, half_y)) = instance_world_y_extent(output, *instance) else {
        return false;
    };
    let center_y = instance_world_translation(output, *instance)[1];
    let gap = center_y - half_y - body_half_y;
    (-0.04..=0.55).contains(&gap)
}

fn instance_world_y_extent(
    output: &FoundryCompilationOutput,
    instance: PartInstanceId,
) -> Option<([f32; 3], f32)> {
    let instance = &output.recipe.instances[&instance];
    let half_y = match output.recipe.definitions[&instance.definition]
        .geometry
        .source
    {
        GeometrySource::RoundedBox { half_extents, .. } => half_extents[1],
        GeometrySource::Cylinder { height, .. } | GeometrySource::Frustum { height, .. } => {
            height * 0.5
        }
        GeometrySource::Plate { thickness, .. } => thickness * 0.5,
        _ => return None,
    };
    Some((instance_world_translation(output, instance.id), half_y))
}

fn instance_world_translation(
    output: &FoundryCompilationOutput,
    instance: PartInstanceId,
) -> [f32; 3] {
    let current = &output.recipe.instances[&instance];
    let mut translation = current.local_transform.translation;
    if let Some(parent) = current.parent {
        let parent_translation = instance_world_translation(output, parent);
        translation[0] += parent_translation[0];
        translation[1] += parent_translation[1];
        translation[2] += parent_translation[2];
    }
    translation
}

fn role_rounded_box_half_extents(output: &FoundryCompilationOutput, role: &str) -> [f32; 3] {
    let tag = format!("role:{role}");
    let instance = output
        .recipe
        .instances
        .values()
        .find(|instance| instance.tags.contains(&tag))
        .expect("role instance exists");
    match output.recipe.definitions[&instance.definition]
        .geometry
        .source
    {
        GeometrySource::RoundedBox { half_extents, .. } => half_extents,
        _ => panic!("{role} should be a rounded box"),
    }
}

fn mesh_triangle_count(output: &FoundryCompilationOutput) -> usize {
    output.artifact.combined_preview.mesh.indices.len() / 3
}
