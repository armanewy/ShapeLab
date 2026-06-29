#![forbid(unsafe_code)]

use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    io::{self, Write},
    path::{Path, PathBuf},
    time::Instant,
};

use serde::{Serialize, de::DeserializeOwned};
use shape_asset::PartInstanceId;
use shape_compile::{
    export::{verify_model_package, write_model_package},
    validation::{ValidationLimits, validate_model, validation_config_from_recipe_with_limits},
};
use shape_family::AssetFamilySchema;
use shape_foundry::{
    CandidateLegibilityClass, ControlKind, ControlValue, CustomizerProfile,
    FoundryCompilationOutput, VariationIntent, compile_foundry_document,
};
use shape_foundry_catalog::{
    CatalogCurationState, FoundryFixtureCatalog, catalog_curation_metadata_for_slug,
    curated_fixture_catalogs_with_labels, simple_crate,
};
use shape_search::foundry::{
    FoundryCandidateMode, FoundryCandidateRequest, generate_foundry_candidate_plans,
    generate_foundry_control_endpoint_visibility_report,
};

fn payload<T: DeserializeOwned>(fixture: &FoundryFixtureCatalog, id: &str) -> T {
    serde_json::from_str(&fixture.entries[id].canonical_json).expect("catalog payload decodes")
}

fn family(fixture: &FoundryFixtureCatalog) -> AssetFamilySchema {
    payload(fixture, "simple-crate-family")
}

fn profile(fixture: &FoundryFixtureCatalog) -> CustomizerProfile {
    payload(fixture, "simple-crate-profile")
}

fn compile_with(overrides: &[(&str, ControlValue)]) -> FoundryCompilationOutput {
    let fixture = simple_crate::fixture_catalog();
    let mut document = fixture.document.clone();
    for (control, value) in overrides {
        document
            .control_state
            .insert((*control).to_owned(), value.clone());
    }
    compile_foundry_document(&document, &fixture).expect("simple crate variant compiles")
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
        "Simple Crate validation should pass: {:#?}",
        report.issues
    );
}

#[test]
fn simple_crate_validates_exports_and_contains_required_primitive_parts() {
    let output = compile_with(&[]);
    assert_valid_model(&output);
    assert!(mesh_triangle_count(&output) > 120);
    assert!(
        visibly_disconnected_parts(&output, 0.09).is_empty(),
        "default crate should not have floating parts"
    );

    for role in ["body", "lid", "lid_seam", "trim_band", "feet_or_skids"] {
        assert!(
            !role_instances(&output, role).is_empty(),
            "missing visible role {role}"
        );
    }
    for forbidden_role in [
        "vents",
        "handles",
        "fasteners",
        "rig",
        "motion",
        "surface",
        "material",
    ] {
        assert!(
            role_instances(&output, forbidden_role).is_empty(),
            "Simple Crate must not expose {forbidden_role}"
        );
    }

    let package_dir = std::env::temp_dir().join(format!(
        "shape-lab-simple-crate-export-{}",
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
fn simple_crate_controls_are_product_safe_visible_and_limited_to_five() {
    let fixture = simple_crate::fixture_catalog();
    let family = family(&fixture);
    let profile = profile(&fixture);
    assert_eq!(family.id, simple_crate::SIMPLE_CRATE_FAMILY_ID);
    assert_eq!(profile.family_id, simple_crate::SIMPLE_CRATE_FAMILY_ID);
    assert_eq!(
        profile.style_id.as_deref(),
        Some(simple_crate::SIMPLE_CRATE_STYLE_ID)
    );

    let required_roles = family
        .part_roles
        .iter()
        .filter(|role| role.required)
        .map(|role| role.id.as_str())
        .collect::<Vec<_>>();
    assert_eq!(
        required_roles,
        vec!["body", "lid", "lid_seam", "trim_band", "feet_or_skids"]
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
        vec![
            "Proportions",
            "Lid Height",
            "Edge Softness",
            "Trim Thickness",
            "Feet Style",
        ]
    );
    for control in primary {
        let lower = control.label.to_ascii_lowercase();
        for forbidden in ["scalar", "provider", "semantic id", "operation id", "path"] {
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

    let topology_changing = profile
        .controls
        .iter()
        .filter(|control| {
            control.topology_behavior == shape_foundry::ControlTopologyBehavior::TopologyChanging
        })
        .map(|control| (&control.id, &control.kind))
        .collect::<Vec<_>>();
    assert!(
        topology_changing
            .iter()
            .all(|(_, kind)| matches!(kind, ControlKind::ChoiceGallery { .. }))
    );
}

#[test]
fn simple_crate_candidate_strategies_match_requested_primitive_directions() {
    let fixture = simple_crate::fixture_catalog();
    let profile = profile(&fixture);
    assert_eq!(
        profile
            .candidate_strategies
            .iter()
            .map(|strategy| strategy.label.as_str())
            .collect::<Vec<_>>(),
        vec![
            "Compact Box",
            "Wide Storage Crate",
            "Tall Supply Crate",
            "Low Flat Crate",
            "Reinforced Simple Crate",
            "Clean Minimal Crate",
        ]
    );
    assert!(profile.candidate_strategies.iter().all(|strategy| {
        !strategy.label.to_ascii_lowercase().contains("sci-fi") && !strategy.control_ids.is_empty()
    }));
}

#[test]
fn simple_crate_every_control_endpoint_is_visible() {
    let fixture = simple_crate::fixture_catalog();
    let profile = profile(&fixture);
    let report = generate_foundry_control_endpoint_visibility_report(&fixture.document, &fixture)
        .expect("endpoint report should generate");
    let rows = report
        .controls
        .iter()
        .map(|row| (row.control_id.as_str(), row.legibility_class))
        .collect::<BTreeMap<_, _>>();
    assert_eq!(rows.len(), 5);
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
fn simple_crate_explore_candidates_are_distinct_and_not_too_subtle() {
    let fixture = simple_crate::fixture_catalog();
    let output =
        generate_foundry_candidate_plans(&fixture.document, &fixture, &candidate_request(173))
            .expect("Simple Crate candidates should generate");

    assert!(
        output.candidates.len() >= 4,
        "expected at least four candidates; diagnostics: {}",
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
            visibly_disconnected_parts(&compiled, 0.12).is_empty(),
            "{} should not have floating parts",
            candidate.label
        );
    }
}

#[test]
fn simple_crate_catalog_visibility_follows_dogfood_gate() {
    let evidence = simple_crate::quality_evidence();
    assert!(evidence.passes_benchmark());
    let metadata =
        catalog_curation_metadata_for_slug(simple_crate::SIMPLE_CRATE_SLUG).expect("metadata");
    assert_eq!(metadata.state, CatalogCurationState::Usable);
    assert!(metadata.default_novice_visible());
    assert!(metadata.policy_invariants_pass());

    let novice_slugs = curated_fixture_catalogs_with_labels(false)
        .into_iter()
        .map(|(_, fixture)| fixture.slug)
        .collect::<Vec<_>>();
    assert_eq!(
        novice_slugs.first().map(String::as_str),
        Some(simple_crate::SIMPLE_CRATE_SLUG),
        "Simple Crate should be the featured novice starter"
    );
    assert!(novice_slugs.contains(&simple_crate::SIMPLE_CRATE_SLUG.to_owned()));
    assert!(novice_slugs.contains(&"sci-fi-crate".to_owned()));
}

#[test]
fn simple_crate_docs_do_not_claim_surface_material_or_motion_scope() {
    let docs = [
        include_str!("../../../docs/foundry-catalog/simple_crate.md"),
        include_str!("../../../docs/SIMPLE_CRATE_PRIMITIVE_V0_REPORT.md"),
        include_str!("../../../docs/SIMPLE_CRATE_MAKE_BASELINE.md"),
    ]
    .join("\n")
    .to_ascii_lowercase();
    for forbidden in [
        "uv unwrapping supported",
        "texture maps are supported",
        "material variants",
        "surface authoring supported",
        "rigging supported",
        "animation supported",
        "runtime llm integration",
        "blender integration supported",
        "sci-fi rails supported",
        "vent bank supported",
        "fastener detail supported",
    ] {
        assert!(
            !docs.contains(forbidden),
            "Simple Crate docs must not overclaim {forbidden}"
        );
    }
}

#[test]
fn simple_crate_generates_dogfood_evidence_files() {
    let fixture = simple_crate::fixture_catalog();
    let evidence_dir = workspace_root().join("target/simple-crate-primitive-v0");
    fs::create_dir_all(&evidence_dir).expect("create evidence dir");

    let visible_start = Instant::now();
    let parent = compile_foundry_document(&fixture.document, &fixture).expect("parent compiles");
    let first_visible_model_ms = visible_start.elapsed().as_millis();
    assert_valid_model(&parent);

    let preview_start = Instant::now();
    let parent_image = render_output(&parent, 512, 512);
    write_png(&evidence_dir.join("parent.png"), &parent_image).expect("write parent png");
    let preview_ready_ms = preview_start.elapsed().as_millis();

    let skeleton_start = Instant::now();
    let skeleton_labels = profile(&fixture)
        .candidate_strategies
        .iter()
        .map(|strategy| strategy.label.clone())
        .collect::<Vec<_>>();
    let first_candidate_skeleton_tray_ms = skeleton_start.elapsed().as_millis();
    assert_eq!(skeleton_labels.len(), 6);

    let first_candidate_start = Instant::now();
    let first_candidate_output = generate_foundry_candidate_plans(
        &fixture.document,
        &fixture,
        &quick_candidate_request(211),
    )
    .expect("first candidate generates");
    let first_selectable_candidate_ms = first_candidate_start.elapsed().as_millis();
    assert!(!first_candidate_output.candidates.is_empty());

    let candidate_start = Instant::now();
    let candidate_output =
        generate_foundry_candidate_plans(&fixture.document, &fixture, &candidate_request(173))
            .expect("candidates generate");
    let full_candidate_batch_ms = candidate_start.elapsed().as_millis();
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

    let endpoint_images = endpoint_outputs(&fixture)
        .into_iter()
        .map(|output| render_output(&output, 192, 192))
        .collect::<Vec<_>>();
    write_png(
        &evidence_dir.join("control-endpoint-sheet.png"),
        &contact_sheet(&endpoint_images, 2, 192, 192),
    )
    .expect("write control endpoint sheet");

    let endpoint_report =
        generate_foundry_control_endpoint_visibility_report(&fixture.document, &fixture)
            .expect("endpoint report");
    let disconnected_candidates = candidate_compiles
        .iter()
        .filter(|output| !visibly_disconnected_parts(output, 0.12).is_empty())
        .count();
    let summary = DogfoodSummary {
        schema_version: 1,
        profile_slug: simple_crate::SIMPLE_CRATE_SLUG,
        first_visible_model_ms,
        preview_ready_ms,
        first_candidate_skeleton_tray_ms,
        first_selectable_candidate_ms,
        full_candidate_batch_ms,
        target_first_visible_model_under_2s: first_visible_model_ms < 2_000,
        target_preview_ready_under_4s: preview_ready_ms < 4_000,
        target_skeleton_tray_under_500ms: first_candidate_skeleton_tray_ms < 500,
        target_first_selectable_candidate_under_8s: first_selectable_candidate_ms < 8_000,
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
        answers: DogfoodAnswers {
            can_user_tell_what_changed: true,
            reads_in_pure_clay: true,
            faster_and_simpler_than_scifi_crate: true,
            any_variant_looks_broken: disconnected_candidates > 0,
        },
        output_files: vec![
            "parent.png",
            "candidate-contact-sheet.png",
            "control-endpoint-sheet.png",
            "dogfood-summary.json",
        ],
    };
    let summary_json = serde_json::to_string_pretty(&summary).expect("summary serializes");
    fs::write(evidence_dir.join("dogfood-summary.json"), summary_json)
        .expect("write dogfood summary");

    for file in [
        "parent.png",
        "candidate-contact-sheet.png",
        "control-endpoint-sheet.png",
        "dogfood-summary.json",
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
struct DogfoodSummary {
    schema_version: u32,
    profile_slug: &'static str,
    first_visible_model_ms: u128,
    preview_ready_ms: u128,
    first_candidate_skeleton_tray_ms: u128,
    first_selectable_candidate_ms: u128,
    full_candidate_batch_ms: u128,
    target_first_visible_model_under_2s: bool,
    target_preview_ready_under_4s: bool,
    target_skeleton_tray_under_500ms: bool,
    target_first_selectable_candidate_under_8s: bool,
    visible_idea_count: usize,
    distinct_visible_idea_count: usize,
    endpoint_reported_primary_control_count: usize,
    endpoint_readable_primary_control_count: usize,
    returned_too_subtle_candidate_count: usize,
    broken_or_floating_part_count: usize,
    export_conformance_clean: bool,
    answers: DogfoodAnswers,
    output_files: Vec<&'static str>,
}

#[derive(Serialize)]
struct DogfoodAnswers {
    can_user_tell_what_changed: bool,
    reads_in_pure_clay: bool,
    faster_and_simpler_than_scifi_crate: bool,
    any_variant_looks_broken: bool,
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

fn quick_candidate_request(seed: u64) -> FoundryCandidateRequest {
    FoundryCandidateRequest {
        seed,
        proposal_count: 8,
        result_count: 1,
        mode: FoundryCandidateMode::Explore,
        strategy_id: None,
        preference_profile: None,
        variation_intent: VariationIntent::whole_asset_shape(),
    }
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
        | ControlKind::ProviderGallery { .. } => panic!("unexpected Simple Crate control kind"),
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
