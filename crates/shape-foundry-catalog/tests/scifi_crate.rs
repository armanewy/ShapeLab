use std::collections::BTreeSet;

use serde::de::DeserializeOwned;
use shape_asset::{
    CutGroupRole, GeometrySource, ModelingOperationSpec, OperationId, PartInstanceId,
};
use shape_compile::validation::{
    ValidationLimits, validate_model, validation_config_from_recipe_with_limits,
};
use shape_family::AssetFamilySchema;
use shape_foundry::{
    ControlEvaluationContext, ControlKind, ControlValue, CustomizerProfile, FoundryCommand,
    FoundryCompilationOutput, FoundryLock, FoundryLockMode, FoundryLockTarget, VariationIntent,
    compile_foundry_document, whole_model_preview_sample_requests_with_count,
};
use shape_foundry_catalog::{FoundryFixtureCatalog, scifi_crate};
use shape_search::foundry::{
    FoundryCandidateMode, FoundryCandidateRequest, generate_foundry_candidate_plans,
};

fn payload<T: DeserializeOwned>(fixture: &FoundryFixtureCatalog, id: &str) -> T {
    serde_json::from_str(&fixture.entries[id].canonical_json).expect("catalog payload decodes")
}

fn family(fixture: &FoundryFixtureCatalog) -> AssetFamilySchema {
    payload(fixture, "sci-fi-crate-family")
}

fn profile(fixture: &FoundryFixtureCatalog) -> CustomizerProfile {
    payload(fixture, "sci-fi-crate-profile")
}

#[test]
fn scifi_crate_exposes_product_part_groups() {
    let groups = scifi_crate::part_group_descriptors();

    assert_eq!(
        groups
            .iter()
            .map(|group| group.display_name.as_str())
            .collect::<Vec<_>>(),
        vec![
            "Body",
            "Panels",
            "Vents",
            "Handles",
            "Edge Trim",
            "Fasteners"
        ]
    );
    let handles = groups
        .iter()
        .find(|group| group.group_id == "handles")
        .expect("handles group");
    assert_eq!(handles.bound_control_ids, vec!["handle_style"]);
    assert!(handles.focusable);
}

fn compile_with(overrides: &[(&str, ControlValue)]) -> FoundryCompilationOutput {
    let fixture = scifi_crate::fixture_catalog();
    let mut document = fixture.document.clone();
    for (control, value) in overrides {
        document
            .control_state
            .insert((*control).to_owned(), value.clone());
    }
    compile_foundry_document(&document, &fixture).expect("crate variant compiles")
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
        "model validation should pass: {:#?}",
        report.issues
    );
    assert_eq!(report.metrics.accidental_intersection_count, 0);
}

#[test]
fn semantic_body_cuts_survive_foundry_compile() {
    let output = compile_with(&[]);
    let groups = &output.recipe.variation.semantic_cut_groups;

    assert_eq!(groups["front_recesses"].role, CutGroupRole::Recesses);
    assert_eq!(groups["vent_slots"].role, CutGroupRole::Vents);
    assert_eq!(groups["mount_holes"].role, CutGroupRole::MountHoles);
    assert_eq!(groups["front_recesses"].operations.len(), 2);
    assert_eq!(groups["vent_slots"].operations.len(), 3);
    assert_eq!(groups["mount_holes"].operations.len(), 2);

    let body_definition = groups["front_recesses"].definition;
    let operations = &output.recipe.definitions[&body_definition]
        .geometry
        .operations;
    assert!(group_operations_match(
        operations,
        &groups["front_recesses"].operations,
        |operation| matches!(operation, ModelingOperationSpec::RecessedPanelCut { .. }),
    ));
    assert!(group_operations_match(
        operations,
        &groups["vent_slots"].operations,
        |operation| matches!(
            operation,
            ModelingOperationSpec::RectangularThroughCut { .. }
        ),
    ));
    assert!(group_operations_match(
        operations,
        &groups["mount_holes"].operations,
        |operation| matches!(operation, ModelingOperationSpec::CircularThroughCut { .. }),
    ));
}

#[test]
fn boundary_bevels_and_provenance_cover_cut_operations() {
    let output = compile_with(&[]);
    let groups = &output.recipe.variation.semantic_cut_groups;
    let body_definition = groups["front_recesses"].definition;
    let operations = &output.recipe.definitions[&body_definition]
        .geometry
        .operations;
    let bevel_operations = operations
        .iter()
        .filter_map(|operation| match operation {
            ModelingOperationSpec::BevelBoundaryLoop { operation, .. } => Some(*operation),
            _ => None,
        })
        .collect::<Vec<_>>();
    assert!(bevel_operations.len() >= 4);

    let mapped_operations = output
        .artifact
        .provenance_report
        .part_region_operation_mappings
        .iter()
        .filter_map(|mapping| mapping.operation)
        .collect::<BTreeSet<_>>();
    for operation in groups
        .values()
        .flat_map(|group| group.operations.iter().copied())
        .chain(bevel_operations)
    {
        assert!(
            mapped_operations.contains(&operation),
            "missing provenance for operation {operation:?}"
        );
    }

    let triangle_count = output.artifact.combined_preview.mesh.indices.len() / 3;
    assert_eq!(
        output.artifact.combined_preview.triangle_to_part.len(),
        triangle_count
    );
    assert_eq!(
        output.artifact.combined_preview.triangle_to_region.len(),
        triangle_count
    );
    assert_eq!(
        output.artifact.combined_preview.triangle_to_operation.len(),
        triangle_count
    );
}

#[test]
fn compiled_crate_has_no_accidental_intersections() {
    let output = compile_with(&[]);
    assert_valid_model(&output);
}

#[test]
fn authored_strategy_labels_match_hq_intents() {
    let fixture = scifi_crate::fixture_catalog();
    let profile = profile(&fixture);

    assert_eq!(
        profile
            .candidate_strategies
            .iter()
            .map(|strategy| strategy.label.as_str())
            .collect::<Vec<_>>(),
        vec![
            "Compact Vented",
            "Reinforced Cargo",
            "Clean Lab Crate",
            "Heavy Utility",
            "Deep Panel Equipment",
            "Minimal Industrial",
        ]
    );
    for strategy in &profile.candidate_strategies {
        assert!(
            strategy.control_ids.len() >= 3,
            "{} should combine whole-asset controls",
            strategy.label
        );
        assert!(
            !strategy.label.contains("Provider") && !strategy.label.contains("Scalar"),
            "{} should be an intent label",
            strategy.label
        );
    }
}

#[test]
fn six_strategy_style_candidates_compile_with_current_foundry_apis() {
    let candidates = [
        (
            "baseline",
            vec![("handle_style", ControlValue::Choice("side_rail".to_owned()))],
        ),
        (
            "compact-vented",
            vec![
                ("body_proportions", ControlValue::Scalar(0.0)),
                ("structural_heft", ControlValue::Scalar(0.15)),
                ("panel_depth", ControlValue::Scalar(0.25)),
                ("vent_density", ControlValue::Choice("dense".to_owned())),
                ("handle_style", ControlValue::Choice("flush".to_owned())),
                ("edge_softness", ControlValue::Scalar(0.2)),
                ("detail_density", ControlValue::Integer(8)),
            ],
        ),
        (
            "reinforced-cargo",
            vec![
                ("body_proportions", ControlValue::Scalar(0.82)),
                ("structural_heft", ControlValue::Scalar(0.95)),
                ("panel_depth", ControlValue::Scalar(0.8)),
                ("vent_density", ControlValue::Choice("standard".to_owned())),
                ("handle_style", ControlValue::Choice("cargo_bar".to_owned())),
                ("edge_softness", ControlValue::Scalar(0.75)),
                ("detail_density", ControlValue::Integer(12)),
            ],
        ),
        (
            "clean-lab-crate",
            vec![
                ("body_proportions", ControlValue::Scalar(0.35)),
                ("structural_heft", ControlValue::Scalar(0.35)),
                ("panel_depth", ControlValue::Scalar(0.15)),
                ("vent_density", ControlValue::Choice("sparse".to_owned())),
                ("handle_style", ControlValue::Choice("flush".to_owned())),
                ("edge_softness", ControlValue::Scalar(0.8)),
                ("detail_density", ControlValue::Integer(4)),
                ("has_trim", ControlValue::Toggle(false)),
            ],
        ),
        (
            "heavy-utility",
            vec![
                ("body_proportions", ControlValue::Scalar(1.0)),
                ("structural_heft", ControlValue::Scalar(1.0)),
                ("panel_depth", ControlValue::Scalar(0.65)),
                ("vent_density", ControlValue::Choice("standard".to_owned())),
                ("handle_style", ControlValue::Choice("cargo_bar".to_owned())),
                ("edge_softness", ControlValue::Scalar(0.35)),
                ("detail_density", ControlValue::Integer(12)),
            ],
        ),
        (
            "deep-panel-equipment",
            vec![
                ("body_proportions", ControlValue::Scalar(0.55)),
                ("structural_heft", ControlValue::Scalar(0.65)),
                ("panel_depth", ControlValue::Scalar(1.0)),
                ("vent_density", ControlValue::Choice("dense".to_owned())),
                ("handle_style", ControlValue::Choice("side_rail".to_owned())),
                ("edge_softness", ControlValue::Scalar(0.55)),
                ("detail_density", ControlValue::Integer(10)),
            ],
        ),
        (
            "minimal-industrial",
            vec![
                ("body_proportions", ControlValue::Scalar(0.1)),
                ("structural_heft", ControlValue::Scalar(0.25)),
                ("panel_depth", ControlValue::Scalar(0.0)),
                ("vent_density", ControlValue::Choice("sparse".to_owned())),
                ("handle_style", ControlValue::Choice("flush".to_owned())),
                ("edge_softness", ControlValue::Scalar(0.1)),
                ("detail_density", ControlValue::Integer(4)),
                ("has_trim", ControlValue::Toggle(false)),
            ],
        ),
    ];

    let expected_count = candidates.len();
    let mut fingerprints = BTreeSet::new();
    for (label, overrides) in candidates {
        let output = compile_with(&overrides);
        assert_valid_model(&output);
        assert!(
            fingerprints.insert(format!(
                "{:?}",
                output.build_stamp.geometry_input_fingerprint
            )),
            "{label} should have a unique geometry input fingerprint"
        );
    }
    assert_eq!(fingerprints.len(), expected_count);
}

#[test]
fn vent_variants_are_structurally_distinct() {
    let sparse = compile_with(&[("vent_density", ControlValue::Choice("sparse".to_owned()))]);
    let standard = compile_with(&[("vent_density", ControlValue::Choice("standard".to_owned()))]);
    let dense = compile_with(&[("vent_density", ControlValue::Choice("dense".to_owned()))]);

    let sparse_vents = vent_operations(&sparse);
    let standard_vents = vent_operations(&standard);
    let dense_vents = vent_operations(&dense);

    assert_eq!(sparse_vents.len(), 2);
    assert_eq!(standard_vents.len(), 3);
    assert_eq!(dense_vents.len(), 5);
    assert!(
        sparse_vents.iter().any(|(_, size, _)| size[0] > 0.25),
        "sparse vents should read as large paired slots"
    );
    assert!(
        dense_vents.iter().all(|(_, size, _)| size[0] < 0.16),
        "dense vents should read as a tight bank of smaller slots"
    );
}

#[test]
fn handle_options_validate_and_are_attached_assemblies() {
    for style in ["flush", "side_rail", "cargo_bar"] {
        let output = compile_with(&[("handle_style", ControlValue::Choice(style.to_owned()))]);
        assert_valid_model(&output);
        let handle_instances = role_instances(&output, "handle");
        assert!(
            handle_instances.len() >= 3,
            "{style} handle should include grip plus authored mounts"
        );
        assert!(
            handle_instances
                .iter()
                .all(|instance| instance_sits_near_body_front(&output, instance)),
            "{style} handle parts should physically overlap the front body shell"
        );
    }
}

#[test]
fn body_panel_heft_detail_and_edge_endpoints_are_visible_descriptors() {
    let compact = compile_with(&[("body_proportions", ControlValue::Scalar(0.0))]);
    let broad = compile_with(&[("body_proportions", ControlValue::Scalar(1.0))]);
    let compact_half = role_rounded_box_half_extents(&compact, "body");
    let broad_half = role_rounded_box_half_extents(&broad, "body");
    assert!(broad_half[0] - compact_half[0] > 0.65);
    assert!(broad_half[2] - compact_half[2] > 0.2);

    let light = compile_with(&[("structural_heft", ControlValue::Scalar(0.0))]);
    let heavy = compile_with(&[("structural_heft", ControlValue::Scalar(1.0))]);
    assert!(
        role_rounded_box_half_extents(&heavy, "body")[1]
            > role_rounded_box_half_extents(&light, "body")[1] + 0.2
    );

    let shallow = compile_with(&[("panel_depth", ControlValue::Scalar(0.0))]);
    let deep = compile_with(&[("panel_depth", ControlValue::Scalar(1.0))]);
    assert!(panel_depth(&deep) > panel_depth(&shallow) + 0.08);

    let low_detail = compile_with(&[("detail_density", ControlValue::Integer(4))]);
    let high_detail = compile_with(&[("detail_density", ControlValue::Integer(12))]);
    assert!(fastener_count(&high_detail) > fastener_count(&low_detail));

    let crisp = compile_with(&[("edge_softness", ControlValue::Scalar(0.0))]);
    let soft = compile_with(&[("edge_softness", ControlValue::Scalar(1.0))]);
    assert!(body_radius(&soft) > body_radius(&crisp) + 0.09);
}

#[test]
fn explore_candidates_survive_as_intent_labeled_whole_asset_ideas() {
    let fixture = scifi_crate::fixture_catalog();
    let output =
        generate_foundry_candidate_plans(&fixture.document, &fixture, &candidate_request(41))
            .expect("crate candidates should generate");

    assert!(
        output.candidates.len() >= 4,
        "expected at least four visibly distinct whole-asset candidates"
    );
    assert!(
        output
            .candidates
            .iter()
            .all(|candidate| candidate.changed_controls.len() >= 2),
        "whole-asset ideas should combine multiple visible controls"
    );
    assert!(
        output.candidates.iter().all(|candidate| {
            candidate.variation_metadata.visible_delta.shape_delta_score > 0.0
                && candidate.conformance.accepted
        }),
        "returned candidates should have visible shape evidence and conformance"
    );
    for candidate in &output.candidates {
        assert!(candidate_label_is_intent(&candidate.label));
    }
}

#[test]
fn foundry_locks_are_respected_for_crate_hq_candidates() {
    let mut fixture = scifi_crate::fixture_catalog();
    fixture.document.foundry_locks.push(FoundryLock {
        target: FoundryLockTarget::Control("handle_style".to_owned()),
        mode: FoundryLockMode::SearchProtected,
        reason: Some("handle is user locked".to_owned()),
    });
    fixture.document.foundry_locks.push(FoundryLock {
        target: FoundryLockTarget::Control("vent_density".to_owned()),
        mode: FoundryLockMode::SearchProtected,
        reason: Some("vents are user locked".to_owned()),
    });

    let output =
        generate_foundry_candidate_plans(&fixture.document, &fixture, &candidate_request(43))
            .expect("locked crate candidates should generate");

    assert!(output.diagnostics.locked_targets_skipped >= 2);
    for candidate in &output.candidates {
        for command in &candidate.edit.commands {
            if let FoundryCommand::SetControl { control_id, .. } = command {
                assert_ne!(control_id, "handle_style");
                assert_ne!(control_id, "vent_density");
            }
        }
    }
}

#[test]
fn primary_control_endpoints_have_previews_and_change_geometry() {
    let fixture = scifi_crate::fixture_catalog();
    let family = family(&fixture);
    let profile = profile(&fixture);
    let primary = profile
        .controls
        .iter()
        .filter(|control| control.primary)
        .collect::<Vec<_>>();
    assert_eq!(
        primary
            .iter()
            .map(|control| control.label.as_str())
            .collect::<Vec<_>>(),
        vec![
            "Body Proportions",
            "Structural Heft",
            "Panel Depth",
            "Vent Density",
            "Handle Style",
            "Edge Softness",
            "Detail Density",
        ]
    );

    let context = ControlEvaluationContext::new(&family.parameter_slots);
    for control in primary {
        match &control.kind {
            ControlKind::ContinuousAxis { .. } => {
                let samples = whole_model_preview_sample_requests_with_count(control, context, 3)
                    .expect("continuous preview samples");
                let values = samples
                    .iter()
                    .map(|sample| sample.value.clone())
                    .collect::<Vec<_>>();
                let interval = &control.domain.continuous_intervals[0];
                assert!(
                    contains_scalar(&values, interval.minimum),
                    "{} missing minimum {:?} in {:?}",
                    control.id,
                    interval.minimum,
                    values
                );
                assert!(
                    contains_scalar(&values, interval.maximum),
                    "{} missing maximum {:?} in {:?}",
                    control.id,
                    interval.maximum,
                    values
                );
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

fn group_operations_match(
    operations: &[ModelingOperationSpec],
    group_operations: &[OperationId],
    predicate: impl Fn(&ModelingOperationSpec) -> bool,
) -> bool {
    group_operations.iter().all(|operation_id| {
        operations
            .iter()
            .find(|operation| operation.operation_id() == *operation_id)
            .is_some_and(&predicate)
    })
}

fn vent_operations(output: &FoundryCompilationOutput) -> Vec<(OperationId, [f32; 2], [f32; 2])> {
    let groups = &output.recipe.variation.semantic_cut_groups;
    let body_definition = groups["vent_slots"].definition;
    let operations = &output.recipe.definitions[&body_definition]
        .geometry
        .operations;
    groups["vent_slots"]
        .operations
        .iter()
        .filter_map(|operation_id| {
            operations
                .iter()
                .find(|operation| operation.operation_id() == *operation_id)
                .and_then(|operation| match operation {
                    ModelingOperationSpec::RectangularThroughCut {
                        operation,
                        size,
                        center,
                        ..
                    } => Some((*operation, *size, *center)),
                    _ => None,
                })
        })
        .collect()
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
    (-0.01..=0.28).contains(&gap)
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

fn panel_depth(output: &FoundryCompilationOutput) -> f32 {
    output
        .recipe
        .definitions
        .values()
        .flat_map(|definition| &definition.geometry.operations)
        .find_map(|operation| match operation {
            ModelingOperationSpec::RecessedPanelCut { depth, .. } => Some(*depth),
            _ => None,
        })
        .expect("panel cut")
}

fn fastener_count(output: &FoundryCompilationOutput) -> u32 {
    output
        .recipe
        .definitions
        .values()
        .flat_map(|definition| &definition.geometry.operations)
        .find_map(|operation| match operation {
            ModelingOperationSpec::LinearArray { count, .. } => Some(*count),
            _ => None,
        })
        .expect("fastener linear array")
}
fn body_radius(output: &FoundryCompilationOutput) -> f32 {
    let tag = "role:body";
    let body = output
        .recipe
        .instances
        .values()
        .find(|instance| instance.tags.contains(tag))
        .expect("body instance");
    match output.recipe.definitions[&body.definition].geometry.source {
        GeometrySource::RoundedBox { radius, .. } => radius,
        _ => panic!("body should be a rounded box"),
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

fn candidate_label_is_intent(label: &str) -> bool {
    let lower = label.to_ascii_lowercase();
    ![
        "provider",
        "scalar",
        "operation",
        "compiler",
        "fragment",
        "recipe",
        "fingerprint",
        "semantic",
    ]
    .iter()
    .any(|forbidden| lower.contains(forbidden))
}

fn assert_endpoint_difference(control_id: &str, first: ControlValue, second: ControlValue) {
    let first = compile_with(&[(control_id, first)]);
    let second = compile_with(&[(control_id, second)]);
    assert_ne!(
        first.build_stamp.geometry_input_fingerprint, second.build_stamp.geometry_input_fingerprint,
        "{control_id} endpoints should produce different geometry input"
    );
}

fn contains_scalar(values: &[ControlValue], expected: f32) -> bool {
    values.iter().any(|value| {
        matches!(value, ControlValue::Scalar(actual) if (*actual - expected).abs() < 0.0001)
    })
}
