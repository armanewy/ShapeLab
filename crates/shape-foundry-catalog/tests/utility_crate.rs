#![forbid(unsafe_code)]

use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    io::{self, Write},
    path::{Path, PathBuf},
};

use serde::{Serialize, de::DeserializeOwned};
use shape_asset::PartInstanceId;
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
use shape_foundry_catalog::{
    CatalogCurationState, FoundryFixtureCatalog, cargo_case, catalog_curation_metadata_for_slug,
    curated_fixture_catalogs_with_labels, simple_crate, utility_crate,
};
use shape_search::foundry::{
    FoundryCandidateMode, FoundryCandidateRequest, generate_foundry_candidate_plans,
    generate_foundry_control_endpoint_visibility_report,
};

fn payload<T: DeserializeOwned>(fixture: &FoundryFixtureCatalog, id: &str) -> T {
    serde_json::from_str(&fixture.entries[id].canonical_json).expect("catalog payload decodes")
}

fn utility_family(fixture: &FoundryFixtureCatalog) -> AssetFamilySchema {
    payload(fixture, "utility-crate-family")
}

fn simple_family(fixture: &FoundryFixtureCatalog) -> AssetFamilySchema {
    payload(fixture, "simple-crate-family")
}

fn cargo_family(fixture: &FoundryFixtureCatalog) -> AssetFamilySchema {
    payload(fixture, "cargo-case-base-family")
}

fn profile(fixture: &FoundryFixtureCatalog) -> CustomizerProfile {
    payload(fixture, "utility-crate-profile")
}

fn style_impl(fixture: &FoundryFixtureCatalog) -> StyleImplementation {
    payload(fixture, "utility-crate-style-impl")
}

fn compile_with(overrides: &[(&str, ControlValue)]) -> FoundryCompilationOutput {
    let fixture = utility_crate::fixture_catalog();
    let mut document = fixture.document.clone();
    for (control, value) in overrides {
        document
            .control_state
            .insert((*control).to_owned(), value.clone());
    }
    compile_foundry_document(&document, &fixture).expect("utility crate variant compiles")
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
        "Utility Crate validation should pass: {:#?}",
        report.issues
    );
}

#[test]
fn utility_crate_validates_exports_and_contains_required_family_parts() {
    let output = compile_with(&[]);
    assert_valid_model(&output);
    assert!(mesh_triangle_count(&output) > 260);
    assert!(
        visibly_disconnected_parts(&output, 0.36).is_empty(),
        "default utility crate should not have floating parts"
    );

    for role in [
        "body",
        "lid",
        "panel_fields",
        "trim_bands",
        "handles",
        "latches",
        "feet_or_skids",
        "detail_marks",
    ] {
        assert!(
            !enabled_role_instances(&output, role).is_empty(),
            "missing visible role {role}"
        );
    }
    for forbidden_role in [
        "vents",
        "corner_guards",
        "fasteners",
        "label_plate_geometry",
        "hinge_or_closure_detail",
        "surface",
        "material",
        "rig",
        "motion",
    ] {
        assert!(
            enabled_role_instances(&output, forbidden_role).is_empty(),
            "Utility Crate must not expose {forbidden_role}"
        );
    }

    let package_dir = std::env::temp_dir().join(format!(
        "shape-lab-utility-crate-export-{}",
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
fn utility_crate_controls_are_novice_safe_visible_and_limited_to_seven() {
    let fixture = utility_crate::fixture_catalog();
    let family = utility_family(&fixture);
    let profile = profile(&fixture);
    assert_eq!(family.id, utility_crate::UTILITY_CRATE_FAMILY_ID);
    assert_eq!(profile.family_id, utility_crate::UTILITY_CRATE_FAMILY_ID);
    assert_eq!(
        profile.style_id.as_deref(),
        Some(utility_crate::UTILITY_CRATE_STYLE_ID)
    );

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
            "Proportions",
            "Lid Style",
            "Panel Style",
            "Trim Style",
            "Handle Style",
            "Latch Detail",
            "Detail Density",
        ]
    );
    assert!(
        primary
            .iter()
            .all(|control| control.label.as_str() != "Feet Style"),
        "feet are an internal provider option, not an eighth novice control"
    );
    for control in primary {
        let lower = control.label.to_ascii_lowercase();
        for forbidden in [
            "scalar",
            "provider id",
            "semantic id",
            "operation id",
            "path",
        ] {
            assert!(
                !lower.contains(forbidden),
                "{} must stay product safe",
                control.label
            );
        }
    }
    assert!(
        profile
            .controls
            .iter()
            .all(|control| !matches!(control.kind, ControlKind::ProviderGallery { .. }))
    );

    assert_control_options(
        &profile,
        "lid_style",
        &["flat_lid", "raised_lid", "rimmed_lid"],
    );
    assert_control_options(
        &profile,
        "panel_style",
        &["clean", "shallow_panels", "framed_panels"],
    );
    assert_control_options(
        &profile,
        "trim_style",
        &["none", "simple_band", "reinforced_band"],
    );
    assert_control_options(
        &profile,
        "handle_style",
        &["none", "cutout_grip", "simple_side_handle"],
    );
    assert_control_options(
        &profile,
        "latch_detail",
        &["none", "simple_latch", "double_latch"],
    );
}

#[test]
fn utility_crate_provider_inventory_covers_requested_options() {
    let fixture = utility_crate::fixture_catalog();
    let style_impl = style_impl(&fixture);
    let provider_ids = style_impl
        .prototypes
        .keys()
        .map(String::as_str)
        .collect::<BTreeSet<_>>();

    for provider in [
        "flat_lid",
        "raised_lid",
        "rimmed_lid",
        "clean_panel_field",
        "shallow_panel_fields",
        "framed_panel_fields",
        "no_trim",
        "simple_trim_band",
        "reinforced_trim_band",
        "no_handles",
        "cutout_grip_handle",
        "simple_side_handle",
        "no_latches",
        "simple_latch",
        "double_latch",
        "no_feet",
        "small_feet",
        "utility_skids",
        "low_detail_marks",
        "medium_detail_marks",
        "high_detail_marks",
    ] {
        assert!(
            provider_ids.contains(provider),
            "missing provider {provider}"
        );
    }
}

#[test]
fn utility_crate_candidate_strategies_match_family_target() {
    let fixture = utility_crate::fixture_catalog();
    let profile = profile(&fixture);
    assert_eq!(
        profile
            .candidate_strategies
            .iter()
            .map(|strategy| strategy.label.as_str())
            .collect::<Vec<_>>(),
        vec![
            "Clean Storage Crate",
            "Reinforced Utility Crate",
            "Compact Carry Crate",
            "Wide Supply Crate",
            "Lidded Field Crate",
            "Minimal Workshop Crate",
        ]
    );
    assert!(profile.candidate_strategies.iter().all(|strategy| {
        !strategy.label.to_ascii_lowercase().contains("sci-fi") && !strategy.control_ids.is_empty()
    }));
}

#[test]
fn utility_crate_sits_between_simple_crate_and_cargo_case() {
    let simple = simple_crate::fixture_catalog();
    let utility = utility_crate::fixture_catalog();
    let cargo = cargo_case::fixture_catalog();
    let simple_family = simple_family(&simple);
    let utility_family = utility_family(&utility);
    let cargo_family = cargo_family(&cargo);
    let simple_profile: CustomizerProfile = payload(&simple, "simple-crate-profile");
    let utility_profile = profile(&utility);
    let cargo_profile: CustomizerProfile = payload(&cargo, "cargo-case-base-profile");
    let utility_style = style_impl(&utility);
    let simple_style: StyleImplementation = payload(&simple, "simple-crate-style-impl");
    let cargo_style: StyleImplementation = payload(&cargo, "cargo-case-base-style-impl");

    assert!(utility_family.part_roles.len() > simple_family.part_roles.len());
    assert!(utility_family.part_roles.len() < cargo_family.part_roles.len());
    assert!(utility_style.prototypes.len() > simple_style.prototypes.len());
    assert!(utility_style.prototypes.len() < cargo_style.prototypes.len());
    assert!(
        utility_profile.controls.len() > simple_profile.controls.len(),
        "Utility Crate should enrich Simple Crate controls"
    );
    assert!(
        utility_profile.controls.len() <= cargo_profile.controls.len(),
        "Utility Crate must not exceed Cargo Case's novice control count"
    );

    let utility_roles = utility_family
        .part_roles
        .iter()
        .map(|role| role.id.as_str())
        .collect::<BTreeSet<_>>();
    for required in [
        "body",
        "lid",
        "panel_fields",
        "trim_bands",
        "handles",
        "latches",
        "feet_or_skids",
        "detail_marks",
    ] {
        assert!(utility_roles.contains(required), "missing role {required}");
    }
    for cargo_only in [
        "corner_guards",
        "vents",
        "fasteners",
        "reinforcement_bands",
        "utility_rails",
        "side_grilles",
        "label_plate_geometry",
        "hinge_or_closure_detail",
    ] {
        assert!(
            !utility_roles.contains(cargo_only),
            "Utility Crate should remain simpler than Cargo Case by omitting {cargo_only}"
        );
    }

    let provider_blob = utility_style
        .prototypes
        .keys()
        .chain(utility_style.default_role_providers.values())
        .cloned()
        .collect::<Vec<_>>()
        .join("\n")
        .to_ascii_lowercase();
    for forbidden in ["sci", "vent", "grille", "armor", "industrial", "cargo_bar"] {
        assert!(
            !provider_blob.contains(forbidden),
            "Utility Crate should not default to sci-fi/Cargo Case provider language: {forbidden}"
        );
    }
}

#[test]
fn utility_crate_every_control_endpoint_is_visible() {
    let fixture = utility_crate::fixture_catalog();
    let profile = profile(&fixture);
    let report = generate_foundry_control_endpoint_visibility_report(&fixture.document, &fixture)
        .expect("endpoint report should generate");
    let rows = report
        .controls
        .iter()
        .map(|row| (row.control_id.as_str(), row.legibility_class))
        .collect::<BTreeMap<_, _>>();
    assert_eq!(rows.len(), 7);
    for control in profile
        .controls
        .iter()
        .filter(|control| control.primary && control.visible)
    {
        assert!(
            matches!(
                rows.get(control.id.as_str()),
                Some(
                    CandidateLegibilityClass::Strong
                        | CandidateLegibilityClass::Clear
                        | CandidateLegibilityClass::SubtleButExplainable
                )
            ),
            "{} endpoint should produce visible geometry: {:?}",
            control.id,
            report
                .controls
                .iter()
                .find(|row| row.control_id == control.id)
        );
        assert_endpoint_difference(control);
    }
}

#[test]
fn utility_crate_explore_candidates_are_distinct_not_too_subtle_and_connected() {
    let fixture = utility_crate::fixture_catalog();
    let output =
        generate_foundry_candidate_plans(&fixture.document, &fixture, &candidate_request(441))
            .expect("Utility Crate candidates should generate");

    assert!(
        output.candidates.len() >= 4,
        "expected at least four Utility Crate candidates; diagnostics: {}",
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

    for candidate in &output.candidates {
        let compiled = compile_foundry_document(&candidate.document, &fixture)
            .expect("candidate document compiles");
        assert_valid_model(&compiled);
        assert!(
            visibly_disconnected_parts(&compiled, 0.38).is_empty(),
            "{} should not have floating parts",
            candidate.label
        );
    }
}

#[test]
fn utility_crate_optional_handles_latches_and_feet_do_not_float() {
    for handle in ["none", "cutout_grip", "simple_side_handle"] {
        let output = compile_with(&[("handle_style", ControlValue::Choice(handle.to_owned()))]);
        assert_valid_model(&output);
        if handle == "none" {
            assert!(enabled_role_instances(&output, "handles").is_empty());
        } else {
            assert!(
                !enabled_role_instances(&output, "handles").is_empty(),
                "{handle} should include visible handle geometry"
            );
        }
        assert!(
            visibly_disconnected_parts(&output, 0.38).is_empty(),
            "{handle} handles must stay attached"
        );
    }

    for latch in ["none", "simple_latch", "double_latch"] {
        let output = compile_with(&[("latch_detail", ControlValue::Choice(latch.to_owned()))]);
        assert_valid_model(&output);
        if latch == "none" {
            assert!(enabled_role_instances(&output, "latches").is_empty());
        } else {
            assert!(
                !enabled_role_instances(&output, "latches").is_empty(),
                "{latch} should include visible latch geometry"
            );
        }
        assert!(
            visibly_disconnected_parts(&output, 0.38).is_empty(),
            "{latch} latches must stay attached"
        );
    }

    for (density, expected_feet) in [
        ("low_detail", false),
        ("medium_detail", true),
        ("high_detail", true),
    ] {
        let output = compile_with(&[("detail_density", ControlValue::Choice(density.to_owned()))]);
        assert_valid_model(&output);
        assert_eq!(
            !enabled_role_instances(&output, "feet_or_skids").is_empty(),
            expected_feet,
            "{density} should map to the expected feet/skids provider"
        );
        assert!(
            visibly_disconnected_parts(&output, 0.38).is_empty(),
            "{density} feet/skids must stay attached"
        );
    }
}

#[test]
fn utility_crate_catalog_visibility_follows_family_quality_gate() {
    let evidence = utility_crate::quality_evidence();
    assert!(evidence.passes_benchmark());
    let metadata =
        catalog_curation_metadata_for_slug(utility_crate::UTILITY_CRATE_SLUG).expect("metadata");
    assert_eq!(metadata.state, CatalogCurationState::Usable);
    assert!(metadata.default_novice_visible());
    assert!(metadata.policy_invariants_pass());

    let novice_slugs = curated_fixture_catalogs_with_labels(false)
        .into_iter()
        .map(|(_, fixture)| fixture.slug)
        .collect::<BTreeSet<_>>();
    assert!(novice_slugs.contains(utility_crate::UTILITY_CRATE_SLUG));
}

#[test]
fn utility_crate_docs_do_not_claim_surface_texturing_or_external_runtime_scope() {
    let docs = [
        include_str!("../../../docs/foundry-catalog/utility_crate.md"),
        include_str!("../../../docs/UTILITY_CRATE_FAMILY_V1_REPORT.md"),
        include_str!("../../../docs/FAMILY_MATURITY_LADDER.md"),
    ]
    .join("\n")
    .to_ascii_lowercase();
    assert!(docs.contains("utility crate"));
    assert!(docs.contains("simple crate"));
    assert!(docs.contains("cargo case"));
    for forbidden in [
        "uv unwrapping supported",
        "texture maps are supported",
        "material variants are supported",
        "surface authoring supported",
        "runtime llm integration",
        "blender integration supported",
        "browser implementation",
        "server implementation",
        "rigging supported",
        "animation supported",
        "imported mesh editing supported",
    ] {
        assert!(
            !docs.contains(forbidden),
            "Utility Crate docs must not overclaim {forbidden}"
        );
    }
}

#[test]
fn utility_crate_generates_family_v1_evidence_files() {
    let fixture = utility_crate::fixture_catalog();
    let evidence_dir = workspace_root().join("target/utility-crate-family-v1");
    fs::create_dir_all(&evidence_dir).expect("create evidence dir");

    let parent = compile_foundry_document(&fixture.document, &fixture).expect("parent compiles");
    assert_valid_model(&parent);
    let parent_image = render_output(&parent, 512, 512);
    write_png(&evidence_dir.join("parent.png"), &parent_image).expect("write parent png");

    let candidate_output =
        generate_foundry_candidate_plans(&fixture.document, &fixture, &candidate_request(441))
            .expect("candidates generate");
    assert!(candidate_output.candidates.len() >= 4);
    let candidate_compiles = candidate_output
        .candidates
        .iter()
        .map(|candidate| {
            compile_foundry_document(&candidate.document, &fixture).expect("candidate compiles")
        })
        .collect::<Vec<_>>();
    let candidate_images = candidate_compiles
        .iter()
        .map(|output| render_output(output, 256, 256))
        .collect::<Vec<_>>();
    write_png(
        &evidence_dir.join("candidate-contact-sheet.png"),
        &contact_sheet(&candidate_images, 3, 256, 256),
    )
    .expect("write candidate contact sheet");

    let endpoint_report =
        generate_foundry_control_endpoint_visibility_report(&fixture.document, &fixture)
            .expect("endpoint report");
    let endpoint_outputs = endpoint_outputs(&fixture);
    let endpoint_images = endpoint_outputs
        .iter()
        .map(|output| render_output(output, 192, 192))
        .collect::<Vec<_>>();
    write_png(
        &evidence_dir.join("control-endpoint-sheet.png"),
        &contact_sheet(&endpoint_images, 2, 192, 192),
    )
    .expect("write control endpoint sheet");

    let simple_fixture = simple_crate::fixture_catalog();
    let simple_parent =
        compile_foundry_document(&simple_fixture.document, &simple_fixture).expect("simple parent");
    write_png(
        &evidence_dir.join("comparison-simple-vs-utility.png"),
        &contact_sheet(
            &[
                render_output(&simple_parent, 320, 320),
                render_output(&parent, 320, 320),
            ],
            2,
            320,
            320,
        ),
    )
    .expect("write comparison sheet");

    let disconnected_candidates = candidate_compiles
        .iter()
        .filter(|output| !visibly_disconnected_parts(output, 0.38).is_empty())
        .count();
    let report = QualityReport {
        schema_version: 1,
        profile_slug: utility_crate::UTILITY_CRATE_SLUG,
        family_id: utility_crate::UTILITY_CRATE_FAMILY_ID,
        primary_control_count: profile(&fixture)
            .controls
            .iter()
            .filter(|control| control.primary && control.visible)
            .count(),
        visible_idea_count: candidate_output.candidates.len(),
        distinct_visible_idea_count: candidate_output
            .candidates
            .iter()
            .map(|candidate| candidate.changed_controls.join("|"))
            .collect::<BTreeSet<_>>()
            .len(),
        endpoint_reported_primary_control_count: endpoint_report.controls.len(),
        endpoint_readable_primary_control_count: endpoint_report
            .controls
            .iter()
            .filter(|row| {
                matches!(
                    row.legibility_class,
                    CandidateLegibilityClass::Strong
                        | CandidateLegibilityClass::Clear
                        | CandidateLegibilityClass::SubtleButExplainable
                )
            })
            .count(),
        returned_too_subtle_candidate_count: candidate_output
            .candidates
            .iter()
            .filter(|candidate| {
                candidate.variation_metadata.visible_delta.legibility_class
                    == CandidateLegibilityClass::TooSubtle
            })
            .count(),
        broken_or_floating_part_count: disconnected_candidates,
        export_conformance_clean: parent.final_conformance.is_accepted(),
        family_ladder: FamilyLadderEvidence {
            richer_than_simple_crate: true,
            simpler_than_cargo_case: true,
            novice_friendly_control_count: true,
            no_sci_fi_specific_providers_by_default: true,
            no_uv_or_texturing_claims: true,
        },
        output_files: vec![
            "parent.png",
            "candidate-contact-sheet.png",
            "control-endpoint-sheet.png",
            "comparison-simple-vs-utility.png",
            "quality-report.json",
        ],
    };
    fs::write(
        evidence_dir.join("quality-report.json"),
        serde_json::to_string_pretty(&report).expect("report serializes"),
    )
    .expect("write quality report");

    for file in [
        "parent.png",
        "candidate-contact-sheet.png",
        "control-endpoint-sheet.png",
        "comparison-simple-vs-utility.png",
        "quality-report.json",
    ] {
        let path = evidence_dir.join(file);
        assert!(path.exists(), "{} should exist", path.display());
        assert!(
            fs::metadata(path).expect("evidence metadata").len() > 0,
            "{file} should not be empty"
        );
    }
}

#[derive(Serialize)]
struct QualityReport {
    schema_version: u32,
    profile_slug: &'static str,
    family_id: &'static str,
    primary_control_count: usize,
    visible_idea_count: usize,
    distinct_visible_idea_count: usize,
    endpoint_reported_primary_control_count: usize,
    endpoint_readable_primary_control_count: usize,
    returned_too_subtle_candidate_count: usize,
    broken_or_floating_part_count: usize,
    export_conformance_clean: bool,
    family_ladder: FamilyLadderEvidence,
    output_files: Vec<&'static str>,
}

#[derive(Serialize)]
struct FamilyLadderEvidence {
    richer_than_simple_crate: bool,
    simpler_than_cargo_case: bool,
    novice_friendly_control_count: bool,
    no_sci_fi_specific_providers_by_default: bool,
    no_uv_or_texturing_claims: bool,
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

fn assert_control_options(profile: &CustomizerProfile, control_id: &str, expected: &[&str]) {
    let control = profile
        .controls
        .iter()
        .find(|control| control.id == control_id)
        .expect("control exists");
    let ControlKind::ChoiceGallery { options } = &control.kind else {
        panic!("{control_id} should be a choice control");
    };
    assert_eq!(
        options
            .iter()
            .map(|option| option.value.as_str())
            .collect::<Vec<_>>(),
        expected
    );
}

fn assert_endpoint_difference(control: &shape_foundry::CustomizerControl) {
    match &control.kind {
        ControlKind::ContinuousAxis { .. } => {
            let interval = &control.domain.continuous_intervals[0];
            assert_ne!(
                compile_with(&[(control.id.as_str(), ControlValue::Scalar(interval.minimum))])
                    .build_stamp
                    .geometry_input_fingerprint,
                compile_with(&[(control.id.as_str(), ControlValue::Scalar(interval.maximum))])
                    .build_stamp
                    .geometry_input_fingerprint,
                "{} endpoints must change compiled geometry",
                control.id
            );
        }
        ControlKind::ChoiceGallery { options } => {
            assert_ne!(
                compile_with(&[(
                    control.id.as_str(),
                    ControlValue::Choice(options.first().expect("first option").value.clone()),
                )])
                .build_stamp
                .geometry_input_fingerprint,
                compile_with(&[(
                    control.id.as_str(),
                    ControlValue::Choice(options.last().expect("last option").value.clone()),
                )])
                .build_stamp
                .geometry_input_fingerprint,
                "{} endpoints must change compiled geometry",
                control.id
            );
        }
        ControlKind::IntegerStepper { .. }
        | ControlKind::Toggle { .. }
        | ControlKind::ProviderGallery { .. } => panic!("unexpected Utility Crate control kind"),
    }
}

fn endpoint_outputs(fixture: &FoundryFixtureCatalog) -> Vec<FoundryCompilationOutput> {
    let mut outputs = Vec::new();
    for control in profile(fixture)
        .controls
        .into_iter()
        .filter(|control| control.primary && control.visible)
    {
        match control.kind {
            ControlKind::ContinuousAxis { .. } => {
                let interval = &control.domain.continuous_intervals[0];
                outputs.push(compile_with(&[(
                    control.id.as_str(),
                    ControlValue::Scalar(interval.minimum),
                )]));
                outputs.push(compile_with(&[(
                    control.id.as_str(),
                    ControlValue::Scalar(interval.maximum),
                )]));
            }
            ControlKind::ChoiceGallery { options } => {
                outputs.push(compile_with(&[(
                    control.id.as_str(),
                    ControlValue::Choice(options.first().expect("first option").value.clone()),
                )]));
                outputs.push(compile_with(&[(
                    control.id.as_str(),
                    ControlValue::Choice(options.last().expect("last option").value.clone()),
                )]));
            }
            ControlKind::IntegerStepper { .. }
            | ControlKind::Toggle { .. }
            | ControlKind::ProviderGallery { .. } => {}
        }
    }
    outputs
}

fn enabled_role_instances(output: &FoundryCompilationOutput, role: &str) -> Vec<PartInstanceId> {
    let tag = format!("role:{role}");
    output
        .recipe
        .instances
        .iter()
        .filter(|(_, instance)| instance.enabled && instance.tags.contains(&tag))
        .map(|(id, _)| *id)
        .collect()
}

fn mesh_triangle_count(output: &FoundryCompilationOutput) -> usize {
    output.artifact.combined_preview.mesh.indices.len() / 3
}

fn visibly_disconnected_parts(
    output: &FoundryCompilationOutput,
    max_nearest_part_gap: f32,
) -> Vec<(String, f32)> {
    let parts = output
        .artifact
        .compiled_parts
        .iter()
        .filter(|part| !part.world_mesh.bounds.is_empty())
        .collect::<Vec<_>>();
    if parts.len() <= 1 {
        return Vec::new();
    }

    parts
        .iter()
        .filter_map(|part| {
            let nearest_gap = parts
                .iter()
                .filter(|other| other.instance_id != part.instance_id)
                .map(|other| {
                    bounds_gap(
                        part.world_mesh.bounds.min,
                        part.world_mesh.bounds.max,
                        other.world_mesh.bounds.min,
                        other.world_mesh.bounds.max,
                    )
                })
                .fold(f32::INFINITY, f32::min);
            (nearest_gap > max_nearest_part_gap).then(|| (part.instance_name.clone(), nearest_gap))
        })
        .collect()
}

fn bounds_gap(
    left_min: [f32; 3],
    left_max: [f32; 3],
    right_min: [f32; 3],
    right_max: [f32; 3],
) -> f32 {
    let mut squared = 0.0_f32;
    for axis in 0..3 {
        let gap = if left_max[axis] < right_min[axis] {
            right_min[axis] - left_max[axis]
        } else if right_max[axis] < left_min[axis] {
            left_min[axis] - right_max[axis]
        } else {
            0.0
        };
        squared += gap * gap;
    }
    squared.sqrt()
}

fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .components()
        .collect()
}

#[derive(Clone)]
struct Image {
    width: u32,
    height: u32,
    pixels: Vec<[u8; 4]>,
}

impl Image {
    fn new(width: u32, height: u32, color: [u8; 4]) -> Self {
        Self {
            width,
            height,
            pixels: vec![color; (width * height) as usize],
        }
    }

    fn set(&mut self, x: i32, y: i32, color: [u8; 4]) {
        if x >= 0 && y >= 0 && (x as u32) < self.width && (y as u32) < self.height {
            self.pixels[(y as u32 * self.width + x as u32) as usize] = color;
        }
    }

    fn blit(&mut self, source: &Image, origin_x: u32, origin_y: u32) {
        for y in 0..source.height {
            for x in 0..source.width {
                let color = source.pixels[(y * source.width + x) as usize];
                self.set((origin_x + x) as i32, (origin_y + y) as i32, color);
            }
        }
    }
}

fn render_output(output: &FoundryCompilationOutput, width: u32, height: u32) -> Image {
    let parts = output
        .artifact
        .compiled_parts
        .iter()
        .filter(|part| !part.world_mesh.bounds.is_empty())
        .map(|part| (part.world_mesh.bounds.min, part.world_mesh.bounds.max))
        .collect::<Vec<_>>();
    let mut projected = Vec::new();
    for (min, max) in &parts {
        for corner in cuboid_corners(*min, *max) {
            projected.push(project_iso(corner));
        }
    }
    let (min_x, max_x, min_y, max_y) = projected.iter().fold(
        (
            f32::INFINITY,
            f32::NEG_INFINITY,
            f32::INFINITY,
            f32::NEG_INFINITY,
        ),
        |(min_x, max_x, min_y, max_y), point| {
            (
                min_x.min(point[0]),
                max_x.max(point[0]),
                min_y.min(point[1]),
                max_y.max(point[1]),
            )
        },
    );
    let span_x = (max_x - min_x).max(0.01);
    let span_y = (max_y - min_y).max(0.01);
    let padding = width.min(height) as f32 * 0.12;
    let scale =
        ((width as f32 - padding * 2.0) / span_x).min((height as f32 - padding * 2.0) / span_y);
    let offset = [
        (width as f32 - span_x * scale) * 0.5 - min_x * scale,
        (height as f32 - span_y * scale) * 0.55 - min_y * scale,
    ];

    let mut faces = Vec::new();
    for (min, max) in parts {
        faces.extend(cuboid_faces(min, max));
    }
    faces.sort_by(|left, right| {
        left.depth
            .partial_cmp(&right.depth)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    let mut image = Image::new(width, height, [246, 246, 244, 255]);
    for face in faces {
        let points = face
            .points
            .iter()
            .map(|point| {
                let projected = project_iso(*point);
                [
                    projected[0] * scale + offset[0],
                    projected[1] * scale + offset[1],
                ]
            })
            .collect::<Vec<_>>();
        fill_polygon(&mut image, &points, face.color);
        draw_polygon_outline(&mut image, &points, [92, 92, 90, 255]);
    }
    image
}

fn contact_sheet(images: &[Image], columns: usize, cell_width: u32, cell_height: u32) -> Image {
    let padding = 16;
    let rows = images.len().div_ceil(columns);
    let mut sheet = Image::new(
        columns as u32 * cell_width + (columns as u32 + 1) * padding,
        rows as u32 * cell_height + (rows as u32 + 1) * padding,
        [238, 238, 236, 255],
    );
    for (index, image) in images.iter().enumerate() {
        let column = index % columns;
        let row = index / columns;
        let x = padding + column as u32 * (cell_width + padding);
        let y = padding + row as u32 * (cell_height + padding);
        sheet.blit(image, x, y);
    }
    sheet
}

#[derive(Clone)]
struct Face {
    points: [[f32; 3]; 4],
    depth: f32,
    color: [u8; 4],
}

fn cuboid_corners(min: [f32; 3], max: [f32; 3]) -> [[f32; 3]; 8] {
    [
        [min[0], min[1], min[2]],
        [max[0], min[1], min[2]],
        [max[0], max[1], min[2]],
        [min[0], max[1], min[2]],
        [min[0], min[1], max[2]],
        [max[0], min[1], max[2]],
        [max[0], max[1], max[2]],
        [min[0], max[1], max[2]],
    ]
}

fn cuboid_faces(min: [f32; 3], max: [f32; 3]) -> Vec<Face> {
    let corners = cuboid_corners(min, max);
    let faces = [
        (
            [corners[3], corners[2], corners[6], corners[7]],
            [210, 210, 206, 255],
        ),
        (
            [corners[4], corners[5], corners[6], corners[7]],
            [178, 178, 174, 255],
        ),
        (
            [corners[1], corners[5], corners[6], corners[2]],
            [190, 190, 186, 255],
        ),
        (
            [corners[0], corners[4], corners[7], corners[3]],
            [168, 168, 164, 255],
        ),
        (
            [corners[0], corners[1], corners[5], corners[4]],
            [158, 158, 154, 255],
        ),
    ];
    faces
        .into_iter()
        .map(|(points, color)| Face {
            depth: points
                .iter()
                .map(|point| point[0] * 0.45 + point[1] * 0.25 + point[2] * 0.65)
                .sum::<f32>()
                / 4.0,
            points,
            color,
        })
        .collect()
}

fn project_iso(point: [f32; 3]) -> [f32; 2] {
    [
        (point[0] - point[2]) * 0.86,
        -point[1] + (point[0] + point[2]) * 0.28,
    ]
}

fn fill_polygon(image: &mut Image, points: &[[f32; 2]], color: [u8; 4]) {
    let min_y = points
        .iter()
        .map(|point| point[1].floor() as i32)
        .min()
        .unwrap_or(0)
        .max(0);
    let max_y = points
        .iter()
        .map(|point| point[1].ceil() as i32)
        .max()
        .unwrap_or(0)
        .min(image.height as i32 - 1);
    for y in min_y..=max_y {
        let scan_y = y as f32 + 0.5;
        let mut intersections = Vec::new();
        for index in 0..points.len() {
            let current = points[index];
            let next = points[(index + 1) % points.len()];
            if (current[1] <= scan_y && next[1] > scan_y)
                || (next[1] <= scan_y && current[1] > scan_y)
            {
                let t = (scan_y - current[1]) / (next[1] - current[1]);
                intersections.push(current[0] + t * (next[0] - current[0]));
            }
        }
        intersections
            .sort_by(|left, right| left.partial_cmp(right).unwrap_or(std::cmp::Ordering::Equal));
        for pair in intersections.chunks(2) {
            if let [left, right] = pair {
                for x in (*left as i32).max(0)..=(*right as i32).min(image.width as i32 - 1) {
                    image.set(x, y, color);
                }
            }
        }
    }
}

fn draw_polygon_outline(image: &mut Image, points: &[[f32; 2]], color: [u8; 4]) {
    for index in 0..points.len() {
        draw_line(
            image,
            points[index],
            points[(index + 1) % points.len()],
            color,
        );
    }
}

fn draw_line(image: &mut Image, start: [f32; 2], end: [f32; 2], color: [u8; 4]) {
    let dx = end[0] - start[0];
    let dy = end[1] - start[1];
    let steps = dx.abs().max(dy.abs()).ceil().max(1.0) as i32;
    for step in 0..=steps {
        let t = step as f32 / steps as f32;
        let x = start[0] + dx * t;
        let y = start[1] + dy * t;
        image.set(x.round() as i32, y.round() as i32, color);
    }
}

fn write_png(path: &Path, image: &Image) -> io::Result<()> {
    let mut raw = Vec::with_capacity(((image.width * 4 + 1) * image.height) as usize);
    for y in 0..image.height {
        raw.push(0);
        for x in 0..image.width {
            raw.extend_from_slice(&image.pixels[(y * image.width + x) as usize]);
        }
    }

    let mut zlib = vec![0x78, 0x01];
    let mut cursor = 0;
    while cursor < raw.len() {
        let remaining = raw.len() - cursor;
        let block_len = remaining.min(65_535);
        let final_block = cursor + block_len == raw.len();
        zlib.push(if final_block { 0x01 } else { 0x00 });
        let len = block_len as u16;
        zlib.extend_from_slice(&len.to_le_bytes());
        zlib.extend_from_slice(&(!len).to_le_bytes());
        zlib.extend_from_slice(&raw[cursor..cursor + block_len]);
        cursor += block_len;
    }
    zlib.extend_from_slice(&adler32(&raw).to_be_bytes());

    let mut file = fs::File::create(path)?;
    file.write_all(b"\x89PNG\r\n\x1a\n")?;
    write_chunk(&mut file, b"IHDR", &png_ihdr(image.width, image.height))?;
    write_chunk(&mut file, b"IDAT", &zlib)?;
    write_chunk(&mut file, b"IEND", &[])?;
    Ok(())
}

fn png_ihdr(width: u32, height: u32) -> Vec<u8> {
    let mut ihdr = Vec::with_capacity(13);
    ihdr.extend_from_slice(&width.to_be_bytes());
    ihdr.extend_from_slice(&height.to_be_bytes());
    ihdr.extend_from_slice(&[8, 6, 0, 0, 0]);
    ihdr
}

fn write_chunk(writer: &mut impl Write, kind: &[u8; 4], data: &[u8]) -> io::Result<()> {
    writer.write_all(&(data.len() as u32).to_be_bytes())?;
    writer.write_all(kind)?;
    writer.write_all(data)?;
    let mut crc_input = Vec::with_capacity(kind.len() + data.len());
    crc_input.extend_from_slice(kind);
    crc_input.extend_from_slice(data);
    writer.write_all(&crc32(&crc_input).to_be_bytes())?;
    Ok(())
}

fn adler32(bytes: &[u8]) -> u32 {
    const MOD: u32 = 65_521;
    let mut a = 1_u32;
    let mut b = 0_u32;
    for byte in bytes {
        a = (a + u32::from(*byte)) % MOD;
        b = (b + a) % MOD;
    }
    (b << 16) | a
}

fn crc32(bytes: &[u8]) -> u32 {
    let mut crc = 0xffff_ffff_u32;
    for byte in bytes {
        crc ^= u32::from(*byte);
        for _ in 0..8 {
            let mask = (crc & 1).wrapping_neg();
            crc = (crc >> 1) ^ (0xedb8_8320 & mask);
        }
    }
    !crc
}
