use std::collections::{BTreeMap, BTreeSet};

use shape_asset::{AssetRecipe, GeometrySource, PartInstanceId};
use shape_family::AllowedOperationKind;
use shape_family_compile::conformance::ConformanceStatus;
use shape_foundry::{
    ControlKind, ControlValue, FoundryCompilationOutput, compile_foundry_document,
};
use shape_foundry_catalog::{FoundryFixtureCatalog, stylized_lamp};

#[test]
fn shade_style_provider_alternatives_preserve_attachment() {
    let fixture = stylized_lamp::fixture_catalog();
    let baseline = compile_with(&fixture, &[]);
    let choices = shade_style_choices(&baseline);
    assert_eq!(choices, vec!["cone", "drum", "task", "minimal"]);

    let mut fingerprints = BTreeSet::new();
    let mut part_counts = BTreeSet::new();
    let mut shade_endpoints = BTreeSet::new();

    for choice in choices {
        let output = compile_with(
            &fixture,
            &[("shade_style", ControlValue::Choice(choice.clone()))],
        );
        assert!(
            output.final_conformance.is_accepted(),
            "{choice} shade should pass conformance"
        );
        assert_model_valid(&output, &format!("{choice} shade"));
        assert!(
            output
                .recipe
                .instances
                .values()
                .any(|instance| instance.name.contains(provider_for_choice(&choice))),
            "{choice} should instantiate its mapped shade provider"
        );

        let shade_attachment = output
            .final_conformance
            .attachments
            .iter()
            .find(|row| row.rule_id == "shade_to_stem")
            .expect("shade attachment row");
        assert_eq!(shade_attachment.status, ConformanceStatus::Passed);
        assert_eq!(shade_attachment.pairs.len(), 1);
        assert!(shade_attachment.pairs[0].connected);
        shade_endpoints.insert((
            shade_attachment.pairs[0].child.socket,
            shade_attachment.pairs[0].parent.socket,
        ));

        fingerprints.insert(format!(
            "{:?}",
            output.build_stamp.geometry_input_fingerprint
        ));
        part_counts.insert(output.artifact.statistics.part_count);
    }

    assert_eq!(
        fingerprints.len(),
        4,
        "shade options should change geometry"
    );
    assert!(
        part_counts.len() > 1,
        "shade options should visibly change whole-model part structure"
    );
    assert_eq!(
        shade_endpoints.len(),
        1,
        "shade provider swaps should preserve attachment endpoint sockets"
    );
}

#[test]
fn default_lamp_is_connected_and_conformant() {
    let fixture = stylized_lamp::fixture_catalog();
    let output = compile_with(&fixture, &[]);

    assert!(output.final_conformance.is_accepted());
    assert!(output.artifact.validation_report.is_valid());

    assert_model_valid(&output, "default lamp");

    let role_counts = output
        .final_conformance
        .roles
        .iter()
        .map(|row| (row.role.as_str(), row.actual_occurrences))
        .collect::<BTreeMap<_, _>>();
    assert_eq!(role_counts.get("base"), Some(&1));
    assert_eq!(role_counts.get("stem"), Some(&1));
    assert_eq!(role_counts.get("joint"), Some(&1));
    assert_eq!(role_counts.get("shade"), Some(&1));

    assert!(
        output
            .final_conformance
            .attachments
            .iter()
            .all(|row| row.status == ConformanceStatus::Passed
                && row.pairs.iter().all(|pair| pair.connected))
    );
    assert_eq!(
        output
            .recipe
            .instances
            .values()
            .filter(|instance| instance.attachment.is_some())
            .count(),
        3,
        "stem, joint assembly, and shade should be attached"
    );
    assert_eq!(
        output
            .recipe
            .instances
            .values()
            .filter(|instance| instance.name.contains("pivot disc joint"))
            .count(),
        2,
        "joint assembly should contain two explicit pivot discs"
    );
    assert_connected_recipe(&output.recipe);
}

#[test]
fn lamp_uses_lathe_and_sweep_not_capsule_chain_fallback() {
    let fixture = stylized_lamp::fixture_catalog();
    let output = compile_with(&fixture, &[]);

    let sources = output
        .recipe
        .definitions
        .values()
        .map(|definition| &definition.geometry.source)
        .collect::<Vec<_>>();
    assert!(
        sources
            .iter()
            .any(|source| matches!(source, GeometrySource::Lathe { .. })),
        "base should be authored as a lathe"
    );
    assert!(
        sources
            .iter()
            .any(|source| matches!(source, GeometrySource::Sweep { .. })),
        "stem should be authored as a sweep"
    );
    let cylinder_count = sources
        .iter()
        .filter(|source| matches!(source, GeometrySource::Cylinder { .. }))
        .count();
    assert!(
        cylinder_count < sources.len(),
        "lamp should not collapse to a capsule/cylinder chain"
    );

    assert!(operation_count(&output, AllowedOperationKind::Lathe) >= 1);
    assert!(operation_count(&output, AllowedOperationKind::Sweep) >= 1);
    assert!(operation_count(&output, AllowedOperationKind::Bevel) >= 1);
    assert!(operation_count(&output, AllowedOperationKind::Primitive) >= 1);
}

#[test]
fn six_authored_candidate_states_are_valid_and_distinct() {
    let fixture = stylized_lamp::fixture_catalog();
    let baseline = compile_with(&fixture, &[]);
    let strategy_labels = baseline
        .catalog
        .customizer_profile
        .candidate_strategies
        .iter()
        .map(|strategy| strategy.label.as_str())
        .collect::<Vec<_>>();
    assert_eq!(
        strategy_labels,
        vec![
            "Compact Task Lamp",
            "Tall Reading Lamp",
            "Playful Curved Lamp",
            "Heavy Base",
            "Minimal"
        ]
    );

    let candidates: Vec<(&str, Vec<(&str, ControlValue)>)> = vec![
        ("balanced", vec![]),
        (
            "compact_task",
            vec![
                ("overall_height", ControlValue::Scalar(1.15)),
                ("base_weight", ControlValue::Scalar(0.5)),
                ("stem_curvature", ControlValue::Scalar(0.2)),
                ("joint_size", ControlValue::Scalar(0.35)),
                ("shade_style", ControlValue::Choice("task".to_owned())),
                ("shade_scale", ControlValue::Scalar(0.68)),
                ("edge_softness", ControlValue::Scalar(0.4)),
            ],
        ),
        (
            "tall_reading",
            vec![
                ("overall_height", ControlValue::Scalar(2.1)),
                ("base_weight", ControlValue::Scalar(0.85)),
                ("stem_curvature", ControlValue::Scalar(0.18)),
                ("joint_size", ControlValue::Scalar(0.55)),
                ("shade_style", ControlValue::Choice("drum".to_owned())),
                ("shade_scale", ControlValue::Scalar(0.88)),
                ("edge_softness", ControlValue::Scalar(0.5)),
            ],
        ),
        (
            "playful_curved",
            vec![
                ("overall_height", ControlValue::Scalar(1.55)),
                ("base_weight", ControlValue::Scalar(0.58)),
                ("stem_curvature", ControlValue::Scalar(1.0)),
                ("joint_size", ControlValue::Scalar(0.68)),
                ("shade_style", ControlValue::Choice("task".to_owned())),
                ("shade_scale", ControlValue::Scalar(0.62)),
                ("edge_softness", ControlValue::Scalar(0.9)),
            ],
        ),
        (
            "heavy_base",
            vec![
                ("overall_height", ControlValue::Scalar(1.35)),
                ("base_weight", ControlValue::Scalar(1.0)),
                ("stem_curvature", ControlValue::Scalar(0.36)),
                ("joint_size", ControlValue::Scalar(0.75)),
                ("shade_style", ControlValue::Choice("cone".to_owned())),
                ("shade_scale", ControlValue::Scalar(0.48)),
                ("edge_softness", ControlValue::Scalar(0.62)),
            ],
        ),
        (
            "minimal",
            vec![
                ("overall_height", ControlValue::Scalar(1.42)),
                ("base_weight", ControlValue::Scalar(0.28)),
                ("stem_curvature", ControlValue::Scalar(0.1)),
                ("joint_size", ControlValue::Scalar(0.25)),
                ("shade_style", ControlValue::Choice("minimal".to_owned())),
                ("shade_scale", ControlValue::Scalar(0.38)),
                ("edge_softness", ControlValue::Scalar(0.18)),
            ],
        ),
    ];

    let mut fingerprints = BTreeSet::new();
    for (name, overrides) in candidates {
        let output = compile_with(&fixture, &overrides);
        assert!(
            output.final_conformance.is_accepted(),
            "{name} should pass conformance"
        );
        assert!(
            output.artifact.validation_report.is_valid(),
            "{name} compile validation should pass"
        );
        assert_model_valid(&output, name);
        fingerprints.insert(format!(
            "{:?}",
            output.build_stamp.geometry_input_fingerprint
        ));
    }
    assert_eq!(fingerprints.len(), 6);
}

#[test]
fn primary_control_endpoints_change_geometry() {
    let fixture = stylized_lamp::fixture_catalog();
    let endpoints = [
        (
            "overall_height",
            ControlValue::Scalar(1.1),
            ControlValue::Scalar(2.2),
        ),
        (
            "base_weight",
            ControlValue::Scalar(0.0),
            ControlValue::Scalar(1.0),
        ),
        (
            "stem_curvature",
            ControlValue::Scalar(0.0),
            ControlValue::Scalar(1.0),
        ),
        (
            "joint_size",
            ControlValue::Scalar(0.0),
            ControlValue::Scalar(1.0),
        ),
        (
            "shade_style",
            ControlValue::Choice("cone".to_owned()),
            ControlValue::Choice("minimal".to_owned()),
        ),
        (
            "shade_scale",
            ControlValue::Scalar(0.0),
            ControlValue::Scalar(1.0),
        ),
        (
            "edge_softness",
            ControlValue::Scalar(0.0),
            ControlValue::Scalar(1.0),
        ),
    ];

    for (control, low, high) in endpoints {
        let low_output = compile_with(&fixture, &[(control, low)]);
        let high_output = compile_with(&fixture, &[(control, high)]);
        assert_model_valid(&low_output, &format!("{control} low endpoint"));
        assert_model_valid(&high_output, &format!("{control} high endpoint"));
        assert_ne!(
            low_output.build_stamp.geometry_input_fingerprint,
            high_output.build_stamp.geometry_input_fingerprint,
            "{control} endpoints should produce different geometry"
        );
    }
}

fn compile_with(
    fixture: &FoundryFixtureCatalog,
    overrides: &[(&str, ControlValue)],
) -> FoundryCompilationOutput {
    let mut document = fixture.document.clone();
    for (control, value) in overrides {
        document
            .control_state
            .insert((*control).to_owned(), value.clone());
    }
    compile_foundry_document(&document, fixture)
        .unwrap_or_else(|error| panic!("stylized lamp should compile: {error:#?}"))
}

fn assert_model_valid(output: &FoundryCompilationOutput, context: &str) {
    let model_config = shape_compile::validation::validation_config_from_recipe_with_limits(
        &output.recipe,
        &output.artifact,
        shape_compile::validation::ValidationLimits::default(),
    );
    let model_report = shape_compile::validation::validate_model(&output.artifact, &model_config);
    assert!(
        model_report.is_valid(),
        "{context} model validation should pass: {:#?}",
        model_report.issues
    );
}

fn shade_style_choices(output: &FoundryCompilationOutput) -> Vec<String> {
    let control = output
        .catalog
        .customizer_profile
        .controls
        .iter()
        .find(|control| control.id == "shade_style")
        .expect("shade style control");
    let ControlKind::ChoiceGallery { options } = &control.kind else {
        panic!("shade style should be a choice gallery");
    };
    options.iter().map(|option| option.value.clone()).collect()
}

fn provider_for_choice(choice: &str) -> &'static str {
    match choice {
        "cone" => "ribbed_cone_shade",
        "drum" => "banded_drum_shade",
        "task" => "angled_task_shade",
        "minimal" => "minimal_shade",
        _ => panic!("unexpected shade choice {choice}"),
    }
}

fn operation_count(output: &FoundryCompilationOutput, operation: AllowedOperationKind) -> u32 {
    output
        .final_conformance
        .operations
        .iter()
        .find(|row| row.operation == operation)
        .map(|row| row.actual_count)
        .unwrap_or_default()
}

fn assert_connected_recipe(recipe: &AssetRecipe) {
    let roots = recipe
        .root_instances
        .iter()
        .copied()
        .collect::<BTreeSet<_>>();
    assert_eq!(roots.len(), 1, "assembled lamp should have one root");

    for instance in recipe
        .instances
        .values()
        .filter(|instance| instance.enabled)
    {
        assert!(
            reaches_root(recipe, instance.id, &roots),
            "{} should be connected to the root assembly",
            instance.name
        );
    }
}

fn reaches_root(
    recipe: &AssetRecipe,
    start: PartInstanceId,
    roots: &BTreeSet<PartInstanceId>,
) -> bool {
    let mut current = Some(start);
    let mut seen = BTreeSet::new();
    while let Some(instance) = current {
        if !seen.insert(instance) {
            return false;
        }
        if roots.contains(&instance) {
            return true;
        }
        current = recipe
            .instances
            .get(&instance)
            .and_then(|instance| instance.parent);
    }
    false
}
