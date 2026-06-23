use std::collections::BTreeSet;

use serde::de::DeserializeOwned;
use shape_asset::{CutGroupRole, ModelingOperationSpec, OperationId};
use shape_compile::validation::{
    ValidationLimits, validate_model, validation_config_from_recipe_with_limits,
};
use shape_family::AssetFamilySchema;
use shape_foundry::{
    ControlEvaluationContext, ControlKind, ControlValue, CustomizerProfile,
    FoundryCompilationOutput, compile_foundry_document,
    whole_model_preview_sample_requests_with_count,
};
use shape_foundry_catalog::{FoundryFixtureCatalog, scifi_crate};

fn payload<T: DeserializeOwned>(fixture: &FoundryFixtureCatalog, id: &str) -> T {
    serde_json::from_str(&fixture.entries[id].canonical_json).expect("catalog payload decodes")
}

fn family(fixture: &FoundryFixtureCatalog) -> AssetFamilySchema {
    payload(fixture, "sci-fi-crate-family")
}

fn profile(fixture: &FoundryFixtureCatalog) -> CustomizerProfile {
    payload(fixture, "sci-fi-crate-profile")
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
fn six_strategy_style_candidates_compile_with_current_foundry_apis() {
    let candidates = [
        (
            "baseline",
            vec![("handle_style", ControlValue::Choice("side_rail".to_owned()))],
        ),
        (
            "compact",
            vec![
                ("body_proportions", ControlValue::Scalar(0.0)),
                ("structural_heft", ControlValue::Scalar(0.15)),
                ("panel_depth", ControlValue::Scalar(0.25)),
                ("vent_density", ControlValue::Choice("sparse".to_owned())),
                ("handle_style", ControlValue::Choice("flush".to_owned())),
                ("edge_softness", ControlValue::Scalar(0.2)),
                ("detail_density", ControlValue::Integer(4)),
                ("has_trim", ControlValue::Toggle(false)),
            ],
        ),
        (
            "reinforced",
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
            "vented",
            vec![
                ("body_proportions", ControlValue::Scalar(0.45)),
                ("structural_heft", ControlValue::Scalar(0.45)),
                ("panel_depth", ControlValue::Scalar(0.55)),
                ("vent_density", ControlValue::Choice("dense".to_owned())),
                ("handle_style", ControlValue::Choice("side_rail".to_owned())),
                ("edge_softness", ControlValue::Scalar(0.35)),
                ("detail_density", ControlValue::Integer(8)),
            ],
        ),
        (
            "minimal",
            vec![
                ("body_proportions", ControlValue::Scalar(0.1)),
                ("structural_heft", ControlValue::Scalar(0.35)),
                ("panel_depth", ControlValue::Scalar(0.2)),
                ("vent_density", ControlValue::Choice("sparse".to_owned())),
                ("handle_style", ControlValue::Choice("flush".to_owned())),
                ("edge_softness", ControlValue::Scalar(0.1)),
                ("detail_density", ControlValue::Integer(4)),
                ("has_trim", ControlValue::Toggle(false)),
            ],
        ),
        (
            "hero",
            vec![
                ("body_proportions", ControlValue::Scalar(1.0)),
                ("structural_heft", ControlValue::Scalar(0.8)),
                ("panel_depth", ControlValue::Scalar(1.0)),
                ("vent_density", ControlValue::Choice("dense".to_owned())),
                ("handle_style", ControlValue::Choice("cargo_bar".to_owned())),
                ("edge_softness", ControlValue::Scalar(1.0)),
                ("detail_density", ControlValue::Integer(12)),
            ],
        ),
    ];

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
    assert_eq!(fingerprints.len(), 6);
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
