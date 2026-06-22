#![forbid(unsafe_code)]

#[path = "../src/asset/mod.rs"]
mod asset;
#[path = "../src/viewport.rs"]
mod viewport;

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::PathBuf;

use asset::jobs::AssetCandidate as JobAssetCandidate;
use asset::state::AssetCandidateSlot;
use asset::{
    AssetAppCommand, AssetAppEffect, AssetAppState, AssetAppStateError, AssetGenerationMode,
    AssetJobEvent, AssetJobKind, AssetJobRequest, AssetLockTarget, AssetTemplate, BoundaryLoopId,
    OperationId, run_asset_job,
};
use shape_asset::{
    AssetEdit, AssetEditProgram, AssetId, AssetRecipe, Frame3, GeometryRecipe, GeometrySource,
    ModelingOperationSpec, ParameterDescriptor, ParameterId, PartDefinition, PartDefinitionId,
    PartInstance, PartInstanceId, ReplacementGroupHint, Transform3, apply_edit_program,
    definition_scalar_path, get_scalar, instance_scalar_path,
};
use shape_modeling_assets::BenchmarkAsset;
use shape_render::{OrbitCamera, RenderSettings};
use shape_search::asset::AssetCandidateEditKind;
use viewport::{ViewportAction, ViewportRenderRequest, ViewportRenderSize};

const BODY_DEFINITION: PartDefinitionId = PartDefinitionId(1);
const ALT_BODY_DEFINITION: PartDefinitionId = PartDefinitionId(2);
const BODY_INSTANCE: PartInstanceId = PartInstanceId(1);
const OPTIONAL_INSTANCE: PartInstanceId = PartInstanceId(2);
const THICKNESS: ParameterId = ParameterId(1);
const TRANSLATE_X: ParameterId = ParameterId(2);

#[test]
fn parameter_edit_updates_recipe_and_schedules_compile() {
    let mut state = test_state();

    let effects = state
        .handle_command(AssetAppCommand::SetParameter {
            parameter: THICKNESS,
            value: 0.24,
        })
        .expect("parameter edit should apply");

    assert_eq!(scalar(&state, THICKNESS), 0.24);
    assert_eq!(state.selected_parameter, Some(THICKNESS));
    assert!(state.dirty);
    assert!(matches!(
        start_job(effects).kind,
        AssetJobKind::CompileCurrentAsset
    ));
}

#[test]
fn structural_edit_adds_and_removes_optional_part() {
    let mut state = test_state();

    state
        .handle_command(AssetAppCommand::AddOptionalPart {
            instance: optional_instance(),
        })
        .expect("optional part should add");
    assert!(state.recipe.instances.contains_key(&OPTIONAL_INSTANCE));
    assert!(
        state
            .recipe
            .variation
            .optional_instances
            .contains(&OPTIONAL_INSTANCE)
    );
    assert_eq!(state.selected_part_instance, Some(OPTIONAL_INSTANCE));

    state
        .handle_command(AssetAppCommand::RemoveOptionalPart(OPTIONAL_INSTANCE))
        .expect("optional part should disable");
    assert!(!state.recipe.instances[&OPTIONAL_INSTANCE].enabled);
    assert!(state.dirty);
}

#[test]
fn lock_blocks_parameter_edits() {
    let mut state = test_state();

    state
        .handle_command(AssetAppCommand::SetLock {
            target: AssetLockTarget::Parameter(THICKNESS),
            locked: true,
        })
        .expect("lock should apply");

    assert!(state.locks.parameters.contains(&THICKNESS));
    assert_eq!(
        state.handle_command(AssetAppCommand::SetParameter {
            parameter: THICKNESS,
            value: 0.3,
        }),
        Err(AssetAppStateError::LockedParameter(THICKNESS))
    );
}

#[test]
fn viewport_commands_update_camera_without_dirtying_recipe() {
    let mut state = test_state();
    let original_revision = state.revision_history.current;
    let original_camera = state.current_camera.clone();

    let effects = state
        .handle_command(AssetAppCommand::Viewport(ViewportAction::Orbit {
            delta_yaw: 12.0,
            delta_pitch: -4.0,
            camera: original_camera.clone(),
        }))
        .expect("orbit should update camera");

    assert!(effects.is_empty());
    assert_eq!(
        state.current_camera.yaw_degrees,
        original_camera.yaw_degrees + 12.0
    );
    assert_eq!(
        state.current_camera.pitch_degrees,
        original_camera.pitch_degrees - 4.0
    );
    assert_eq!(state.revision_history.current, original_revision);
    assert!(!state.dirty);

    let mut requested_camera = OrbitCamera::default();
    requested_camera.zoom(0.5);
    state
        .handle_command(AssetAppCommand::Viewport(
            ViewportAction::RequestFinalRender(ViewportRenderRequest {
                size: ViewportRenderSize::new(640, 480),
                camera: requested_camera.clone(),
            }),
        ))
        .expect("render request should accept camera");

    assert_eq!(state.current_camera, requested_camera.clamped());
    assert_eq!(state.revision_history.current, original_revision);
    assert!(!state.dirty);
}

#[test]
fn candidate_acceptance_replaces_current_recipe() {
    let mut state = test_state();
    let effects = state
        .handle_command(AssetAppCommand::GenerateExplore)
        .expect("generation should schedule");
    let request = start_job(effects);
    let generation_id = request.generation_id().expect("generation id");

    for event in run_asset_job(request) {
        state.handle_job_event(event);
    }

    assert_eq!(
        state
            .active_generation
            .as_ref()
            .map(|generation| generation.mode),
        Some(AssetGenerationMode::Explore)
    );
    let candidate = state.candidate_slots[0].candidate.clone();
    let effects = state
        .handle_command(AssetAppCommand::AcceptCandidate(candidate.id))
        .expect("candidate should apply");

    assert!(effects.is_empty());
    assert_eq!(state.recipe, candidate.recipe);
    assert!(state.candidate_slots.is_empty());
    assert!(state.active_generation.is_none());
    assert!(state.current_artifact.is_some());
    assert!(state.current_timeline.is_some());
    assert!(state.dirty);
    assert_eq!(generation_id, asset::AssetGenerationId(1));
}

#[test]
fn branch_switch_restores_non_linear_revision() {
    let mut state = test_state();

    state
        .handle_command(AssetAppCommand::SetParameter {
            parameter: THICKNESS,
            value: 0.2,
        })
        .expect("first branch edit");
    let first_branch = state.revision_history.current;

    state
        .handle_command(AssetAppCommand::Undo)
        .expect("undo to root");
    state
        .handle_command(AssetAppCommand::SetParameter {
            parameter: THICKNESS,
            value: 0.32,
        })
        .expect("second branch edit");
    let second_branch = state.revision_history.current;

    assert_ne!(first_branch, second_branch);
    assert_eq!(scalar(&state, THICKNESS), 0.32);

    state
        .handle_command(AssetAppCommand::SwitchBranch(first_branch))
        .expect("switch branch");

    assert_eq!(state.revision_history.current, first_branch);
    assert_eq!(scalar(&state, THICKNESS), 0.2);
}

#[test]
fn stale_job_result_is_rejected() {
    let mut state = test_state();
    let first_effects = state
        .handle_command(AssetAppCommand::SetParameter {
            parameter: THICKNESS,
            value: 0.2,
        })
        .expect("first edit");
    let first_request = start_job(first_effects);
    let first_job = first_request.job_id;

    state
        .handle_command(AssetAppCommand::SetParameter {
            parameter: THICKNESS,
            value: 0.3,
        })
        .expect("second edit");

    let stale_event = run_asset_job(first_request)
        .into_iter()
        .find(|event| matches!(event, AssetJobEvent::CompileReady { .. }))
        .expect("compile ready event");

    assert!(!state.handle_job_event(stale_event));
    assert!(state.stale_jobs.contains(&first_job));
    assert!(state.current_artifact.is_none());
}

#[test]
fn failed_compilation_clears_artifact_and_records_issue() {
    let mut state = test_state();
    let request = start_job(
        state
            .handle_command(AssetAppCommand::SetParameter {
                parameter: THICKNESS,
                value: 0.2,
            })
            .expect("edit should schedule compile"),
    );

    assert!(state.handle_job_event(AssetJobEvent::Failed {
        job_id: request.job_id,
        message: "compile failed for test".to_owned(),
    }));

    assert!(state.current_artifact.is_none());
    assert!(state.active_jobs.is_empty());
    assert_eq!(state.validation_issues[0].code, "compile_failed");
}

#[test]
fn template_reset_replaces_recipe_and_clears_dirty_state() {
    let mut state =
        AssetAppState::from_template(template("crate", "Crate", 0.12)).expect("template state");
    state
        .handle_command(AssetAppCommand::SetParameter {
            parameter: THICKNESS,
            value: 0.2,
        })
        .expect("edit template");
    assert!(state.dirty);

    state
        .handle_command(AssetAppCommand::LoadTemplate(Box::new(template(
            "lamp", "Lamp", 0.36,
        ))))
        .expect("template reset");

    assert_eq!(state.recipe.title, "Lamp");
    assert_eq!(scalar(&state, THICKNESS), 0.36);
    assert!(!state.dirty);
    assert_eq!(
        state.current_template.as_ref().map(|t| t.id.as_str()),
        Some("lamp")
    );
    assert!(state.current_file_path.is_none());
    assert_eq!(state.revision_history.current.0, 1);
}

#[test]
fn dirty_state_tracks_save_and_load_boundaries() {
    let mut state = test_state();
    assert!(!state.dirty);
    assert_eq!(
        state.handle_command(AssetAppCommand::Save),
        Err(AssetAppStateError::MissingSavePath)
    );

    state
        .handle_command(AssetAppCommand::SetParameter {
            parameter: THICKNESS,
            value: 0.2,
        })
        .expect("edit should dirty state");
    assert!(state.dirty);

    let path = PathBuf::from("asset.shape.json");
    let effects = state
        .handle_command(AssetAppCommand::SaveAs(path.clone()))
        .expect("save as effect");
    assert!(matches!(
        effects.as_slice(),
        [AssetAppEffect::SaveProject { path: saved, .. }] if saved == &path
    ));

    state.mark_saved(path.clone());
    assert!(!state.dirty);
    assert_eq!(state.current_file_path, Some(path.clone()));

    state
        .replace_loaded_recipe(recipe("Loaded", 0.4), path.clone())
        .expect("loaded recipe should replace state");
    assert!(!state.dirty);
    assert_eq!(state.current_file_path, Some(path));
}

#[test]
fn crate_mvp_workflow_generates_six_unlocked_variants() {
    let mut state =
        AssetAppState::from_template(benchmark_template(BenchmarkAsset::IndustrialCrate))
            .expect("crate template should load");

    state
        .handle_command(AssetAppCommand::SetParameter {
            parameter: ParameterId(1),
            value: 2.25,
        })
        .expect("body should widen");
    state
        .handle_command(AssetAppCommand::SetParameter {
            parameter: ParameterId(5),
            value: 0.11,
        })
        .expect("handle thickness should change");
    state
        .handle_command(AssetAppCommand::SetParameter {
            parameter: ParameterId(6),
            value: 8.0,
        })
        .expect("bolt count should change");
    state
        .handle_command(AssetAppCommand::ToggleOptionalPart {
            instance: PartInstanceId(16),
            enabled: false,
        })
        .expect("optional trim should disable");
    state
        .handle_command(AssetAppCommand::SetLock {
            target: AssetLockTarget::Instance(PartInstanceId(1)),
            locked: true,
        })
        .expect("body instance should lock");

    let request = start_job(
        state
            .handle_command(AssetAppCommand::GenerateExplore)
            .expect("generation should schedule"),
    );
    for event in run_asset_job(request) {
        state.handle_job_event(event);
    }

    assert_eq!(state.candidate_slots.len(), 6);
    assert!(
        state
            .candidate_slots
            .iter()
            .all(|slot| slot.candidate.artifact.is_some()),
        "selected semantic candidates should retain compiled artifacts for preview reuse"
    );
    assert!(
        state.candidate_slots.iter().any(|slot| {
            slot.candidate.changes.len() > 1
                || slot
                    .candidate
                    .changes
                    .iter()
                    .any(|change| change.kind != AssetCandidateEditKind::Parameter)
        }),
        "Explore should surface semantic or structural candidates, not only one scalar nudge"
    );
    let body_parameters = BTreeSet::from([
        ParameterId(1),
        ParameterId(2),
        ParameterId(3),
        ParameterId(4),
    ]);
    assert!(state.candidate_slots.iter().all(|slot| {
        slot.candidate
            .changed_parameters
            .is_disjoint(&body_parameters)
    }));

    let accepted = state.candidate_slots[0].candidate.id;
    state
        .handle_command(AssetAppCommand::AcceptCandidate(accepted))
        .expect("candidate should apply");
    let accepted_revision = state.revision_history.current;
    state
        .handle_command(AssetAppCommand::Undo)
        .expect("undo should return to parent");
    assert_ne!(state.revision_history.current, accepted_revision);
}

#[test]
fn candidate_preview_batch_keeps_successes_when_one_candidate_fails() {
    let mut state =
        AssetAppState::from_template(benchmark_template(BenchmarkAsset::IndustrialCrate))
            .expect("crate template should load");
    let request = start_job(
        state
            .handle_command(AssetAppCommand::GenerateExplore)
            .expect("generation should schedule"),
    );
    for event in run_asset_job(request) {
        state.handle_job_event(event);
    }
    assert_eq!(state.candidate_slots.len(), 6);

    let failed_candidate = state.candidate_slots[0].candidate.id;
    state.candidate_slots[0]
        .candidate
        .recipe
        .definitions
        .clear();
    let preview_request = start_job(
        state
            .request_candidate_previews(RenderSettings {
                width: 24,
                height: 24,
                ..RenderSettings::default()
            })
            .expect("preview batch should schedule"),
    );
    let events = run_asset_job(preview_request);
    let ready = events
        .iter()
        .find_map(|event| match event {
            AssetJobEvent::CandidatePreviewsReady {
                previews, failures, ..
            } => Some((previews, failures)),
            _ => None,
        })
        .expect("preview batch should complete");

    assert_eq!(ready.1.len(), 1);
    assert_eq!(ready.1[0].candidate_id, failed_candidate);
    assert!(
        !ready.0.is_empty(),
        "one failed candidate should not discard successful previews"
    );

    for event in events {
        state.handle_job_event(event);
    }

    assert!(
        state.candidate_slots[0].preview_failure.is_some(),
        "failed candidate should retain an individual error"
    );
    assert!(
        state
            .candidate_slots
            .iter()
            .skip(1)
            .any(|slot| slot.preview.is_some()),
        "successful candidate previews should still be stored"
    );
}

#[test]
fn multi_cut_panel_benchmark_generates_structural_explore_variants() {
    let mut state = AssetAppState::from_template(benchmark_template(BenchmarkAsset::MultiCutPanel))
        .expect("multi-cut panel template should load");
    let request = start_job(
        state
            .handle_command(AssetAppCommand::GenerateExplore)
            .expect("generation should schedule"),
    );
    for event in run_asset_job(request) {
        state.handle_job_event(event);
    }

    assert_eq!(state.candidate_slots.len(), 6);
    assert!(
        state
            .candidate_slots
            .iter()
            .all(|slot| slot.candidate.artifact.is_some()),
        "multi-cut panel candidates should keep compiled preview artifacts"
    );
    assert!(
        state.candidate_slots.iter().any(|slot| {
            slot.candidate
                .changes
                .iter()
                .any(|change| change.kind == AssetCandidateEditKind::ModelingOperation)
        }),
        "Explore should surface duplicated semantic cut operations for the multi-cut panel"
    );
}

#[test]
fn descriptor_free_cut_operation_edit_schedules_compile() {
    let mut state = AssetAppState::from_template(benchmark_template(BenchmarkAsset::MultiCutPanel))
        .expect("multi-cut panel template should load");
    let (definition, operation) = first_circular_cut(&state.recipe);
    let radius_path = definition_scalar_path(
        definition,
        format!("operation.{}.circular_through_cut.radius", operation.0),
    );

    let effects = state
        .handle_command(AssetAppCommand::SetCutOperationScalar {
            definition,
            operation,
            field: "circular_through_cut.radius".to_owned(),
            value: 0.095,
        })
        .expect("cut radius edit should apply");

    assert_eq!(state.selected_cut_operation, Some(operation));
    assert_eq!(
        get_scalar(&state.recipe, &radius_path).expect("radius should be readable"),
        0.095
    );
    assert!(state.dirty);
    assert!(matches!(
        start_job(effects).kind,
        AssetJobKind::CompileCurrentAsset
    ));

    state
        .handle_command(AssetAppCommand::SetLock {
            target: AssetLockTarget::Topology(definition),
            locked: true,
        })
        .expect("topology lock should apply");

    assert!(matches!(
        state.handle_command(AssetAppCommand::SetCutOperationScalar {
            definition,
            operation,
            field: "circular_through_cut.radial_segments".to_owned(),
            value: 24.0,
        }),
        Err(AssetAppStateError::EditRejected(message))
            if message.contains("topology is locked")
    ));
}

#[test]
fn edge_treatment_controls_reflect_and_edit_boundary_bevels() {
    let mut state = AssetAppState::from_template(benchmark_template(BenchmarkAsset::MultiCutPanel))
        .expect("multi-cut panel template should load");
    let (definition, operation) = first_circular_cut(&state.recipe);
    let part = state
        .recipe
        .instances
        .values()
        .find(|instance| instance.definition == definition)
        .expect("definition should have an instance")
        .id;
    state.selected_part_instance = Some(part);
    state.selected_cut_operation = Some(operation);

    let ui_state = asset::view_model::build_asset_ui_state(&state, false);
    let reflected_cut = ui_state
        .cut_operations
        .iter()
        .find(|candidate| candidate.operation == operation)
        .expect("selected cut should be reflected");
    assert!(
        !reflected_cut.edge_treatments.is_empty(),
        "multi-cut panel circular cuts should expose beveled edge treatments"
    );
    let treatment = &reflected_cut.edge_treatments[0];
    assert_eq!(treatment.definition, definition);
    assert_eq!(treatment.part, part);
    assert_eq!(treatment.source_operation, operation);
    assert!(treatment.label.contains("edge: Rounded"));
    let source_loops = state.recipe.definitions[&definition]
        .geometry
        .operations
        .iter()
        .find(|candidate| candidate.operation_id() == operation)
        .expect("source cut should exist")
        .direct_boundary_loop_outputs();
    assert!(
        reflected_cut
            .edge_treatments
            .iter()
            .all(|treatment| source_loops.contains(&treatment.target_loop)),
        "reflected treatments should target direct loops from the selected cut"
    );
    let fields = treatment
        .controls
        .iter()
        .map(|control| control.field.as_str())
        .collect::<Vec<_>>();
    assert_eq!(
        fields,
        vec![
            "bevel_boundary_loop.width",
            "bevel_boundary_loop.segments",
            "bevel_boundary_loop.profile"
        ]
    );
    let width = &treatment.controls[0];
    assert!(width.minimum > 0.0);
    assert!(width.maximum >= width.value);
    assert!(!width.topology_changing);
    assert!(treatment.controls[1].topology_changing);
    assert_eq!(treatment.controls[2].minimum, 0.05);
    assert_eq!(treatment.controls[2].maximum, 8.0);

    let effects = state
        .handle_command(AssetAppCommand::SetCutOperationScalar {
            definition,
            operation: treatment.operation,
            field: "bevel_boundary_loop.width".to_owned(),
            value: (width.value * 0.5).max(width.minimum),
        })
        .expect("edge treatment width edit should apply");

    assert_eq!(
        state.selected_cut_operation,
        Some(operation),
        "editing a nested edge treatment should preserve the selected cut"
    );
    assert_eq!(
        state.revision_history.revisions[&state.revision_history.current].label,
        "Set edge treatment"
    );
    assert!(state.dirty);
    assert!(matches!(
        start_job(effects).kind,
        AssetJobKind::CompileCurrentAsset
    ));
}

#[test]
fn multi_cut_panel_edge_treatments_reflect_authored_cut_roles() {
    let mut state = AssetAppState::from_template(benchmark_template(BenchmarkAsset::MultiCutPanel))
        .expect("multi-cut panel template should load");
    let definition = PartDefinitionId(1);
    state.selected_part_instance = state
        .recipe
        .instances
        .values()
        .find(|instance| instance.definition == definition)
        .map(|instance| instance.id);

    let ui_state = asset::view_model::build_asset_ui_state(&state, false);
    let cuts = ui_state
        .cut_operations
        .iter()
        .map(|cut| (cut.operation, cut))
        .collect::<BTreeMap<_, _>>();

    let recess = cuts
        .get(&OperationId(1))
        .expect("multi-cut panel should reflect recessed panel cut");
    let recess_labels = edge_treatment_labels(recess);
    assert_eq!(
        recess_labels,
        vec!["Entry edge: Rounded", "Floor edge: Rounded"]
    );
    for treatment in &recess.edge_treatments {
        let width = edge_control(treatment, "bevel_boundary_loop.width");
        let expected_max = match treatment.label.as_str() {
            "Entry edge: Rounded" | "Floor edge: Rounded" => 0.054,
            label => panic!("unexpected recessed edge treatment {label}"),
        };
        assert_approx_eq(width.maximum, expected_max);
        assert_eq!(
            edge_control(treatment, "bevel_boundary_loop.segments").maximum,
            8.0
        );
        assert_eq!(
            edge_control(treatment, "bevel_boundary_loop.profile").maximum,
            8.0
        );
    }
    assert!(
        recess.available_edge_treatments.is_empty(),
        "fully rounded recessed panel should not show addable edge treatments"
    );

    for operation in [2, 3, 4, 5] {
        let cut = cuts
            .get(&OperationId(operation))
            .expect("multi-cut panel should reflect mounting-hole cut");
        assert_eq!(edge_treatment_labels(cut), vec!["Entry edge: Rounded"]);
        assert_eq!(
            cut.available_edge_treatments
                .iter()
                .map(|treatment| treatment.label.as_str())
                .collect::<Vec<_>>(),
            vec!["Exit edge: Hard"]
        );
        let width = edge_control(&cut.edge_treatments[0], "bevel_boundary_loop.width");
        assert_approx_eq(width.maximum, 0.039);
    }

    for operation in [6, 7, 8] {
        let cut = cuts
            .get(&OperationId(operation))
            .expect("multi-cut panel should reflect hard-edged vent cut");
        assert!(
            cut.edge_treatments.is_empty(),
            "hard-edged vent operation {operation} should not show rounded edge controls"
        );
        assert!(
            cut.available_edge_treatments.is_empty(),
            "hard-edged vent operation {operation} should not show addable rounding"
        );
    }
}

#[test]
fn parent_cut_controls_respect_dependent_boundary_bevels() {
    let mut state = AssetAppState::from_template(benchmark_template(BenchmarkAsset::MultiCutPanel))
        .expect("multi-cut panel template should load");
    let definition = PartDefinitionId(1);
    state.selected_part_instance = state
        .recipe
        .instances
        .values()
        .find(|instance| instance.definition == definition)
        .map(|instance| instance.id);

    let ui_state = asset::view_model::build_asset_ui_state(&state, false);
    let recess = ui_state
        .cut_operations
        .iter()
        .find(|cut| cut.operation == OperationId(1))
        .expect("multi-cut panel should reflect recessed panel cut");

    assert_approx_eq(
        cut_control(recess, "recessed_panel_cut.depth").minimum,
        0.041,
    );
    assert_approx_eq(
        cut_control(recess, "recessed_panel_cut.rim_width").minimum,
        0.023,
    );
    assert_approx_eq(
        cut_control(recess, "recessed_panel_cut.corner_radius").minimum,
        0.019,
    );
    assert_approx_eq(
        cut_control(recess, "recessed_panel_cut.size.x").minimum,
        0.089,
    );

    assert!(matches!(
        state.handle_command(AssetAppCommand::SetCutOperationScalar {
            definition,
            operation: OperationId(1),
            field: "recessed_panel_cut.depth".to_owned(),
            value: 0.030,
        }),
        Err(AssetAppStateError::EditRejected(message))
            if message.contains("outside feasible range")
    ));
    assert!(matches!(
        state.handle_command(AssetAppCommand::SetCutOperationScalar {
            definition,
            operation: OperationId(1),
            field: "recessed_panel_cut.rim_width".to_owned(),
            value: 0.020,
        }),
        Err(AssetAppStateError::EditRejected(message))
            if message.contains("outside feasible range")
    ));
}

#[test]
fn direct_cut_scalar_validation_preserves_generator_safety_margin() {
    let mut state = AssetAppState::from_template(benchmark_template(BenchmarkAsset::MultiCutPanel))
        .expect("multi-cut panel template should load");

    assert!(matches!(
        state.handle_command(AssetAppCommand::SetCutOperationScalar {
            definition: PartDefinitionId(1),
            operation: OperationId(2),
            field: "circular_through_cut.radius".to_owned(),
            value: 0.030,
        }),
        Err(AssetAppStateError::EditRejected(message))
            if message.contains("outside feasible range")
    ));
}

#[test]
fn add_boundary_loop_bevel_rejects_when_sibling_consumes_cut_depth() {
    let mut state = AssetAppState::from_template(benchmark_template(BenchmarkAsset::MultiCutPanel))
        .expect("multi-cut panel template should load");
    let definition = PartDefinitionId(1);
    let operations = &mut state
        .recipe
        .definitions
        .get_mut(&definition)
        .unwrap()
        .geometry
        .operations;
    operations.retain(|operation| operation.operation_id() != OperationId(10));
    for operation in operations {
        if let ModelingOperationSpec::BevelBoundaryLoop {
            operation: OperationId(9),
            width,
            ..
        } = operation
        {
            *width = 0.079;
        }
    }

    assert!(matches!(
        state.handle_command(AssetAppCommand::AddBoundaryLoopBevel {
            definition,
            source_operation: OperationId(1),
            target_loop: BoundaryLoopId(2),
            width: 0.001,
            segments: 2,
            profile: 1.0,
        }),
        Err(AssetAppStateError::EditRejected(message))
            if message.contains("no feasible bevel width")
    ));
}

#[test]
fn add_boundary_loop_bevel_command_inserts_explicit_edge_treatment() {
    let mut state = AssetAppState::from_template(benchmark_template(BenchmarkAsset::MultiCutPanel))
        .expect("multi-cut panel template should load");
    let definition = PartDefinitionId(1);
    state.selected_part_instance = state
        .recipe
        .instances
        .values()
        .find(|instance| instance.definition == definition)
        .map(|instance| instance.id);
    state.selected_cut_operation = Some(OperationId(2));

    let ui_state = asset::view_model::build_asset_ui_state(&state, false);
    let treatment = ui_state
        .cut_operations
        .iter()
        .find(|cut| cut.operation == OperationId(2))
        .and_then(|cut| cut.available_edge_treatments.first())
        .cloned()
        .expect("mounting hole exit edge should be addable");
    let next_ids = state.recipe.next_ids.clone();

    let effects = state
        .handle_command(AssetAppCommand::AddBoundaryLoopBevel {
            definition: treatment.definition,
            source_operation: treatment.source_operation,
            target_loop: treatment.target_loop,
            width: treatment.width,
            segments: treatment.segments,
            profile: treatment.profile,
        })
        .expect("edge treatment should be inserted");

    let inserted = state.recipe.definitions[&definition]
        .geometry
        .operations
        .iter()
        .find_map(|operation| {
            let ModelingOperationSpec::BevelBoundaryLoop {
                operation,
                target_loop,
                width,
                segments,
                profile,
                bevel_region,
                outer_replacement_loop,
                inner_replacement_loop,
                ..
            } = operation
            else {
                return None;
            };
            (*target_loop == treatment.target_loop).then_some((
                *operation,
                *width,
                *segments,
                *profile,
                *bevel_region,
                *outer_replacement_loop,
                *inner_replacement_loop,
            ))
        })
        .expect("new boundary-loop bevel should exist");

    assert_eq!(inserted.0, OperationId(next_ids.operation));
    assert_eq!(inserted.1, treatment.width);
    assert_eq!(inserted.2, treatment.segments);
    assert_eq!(inserted.3, treatment.profile);
    assert_eq!(inserted.4, shape_asset::RegionId(next_ids.region));
    assert_eq!(inserted.5, BoundaryLoopId(next_ids.boundary_loop));
    assert_eq!(inserted.6, BoundaryLoopId(next_ids.boundary_loop + 1));
    assert_eq!(state.recipe.next_ids.operation, next_ids.operation + 1);
    assert_eq!(state.recipe.next_ids.region, next_ids.region + 1);
    assert_eq!(
        state.recipe.next_ids.boundary_loop,
        next_ids.boundary_loop + 2
    );
    assert_eq!(state.selected_cut_operation, Some(OperationId(2)));
    assert_eq!(
        state.revision_history.revisions[&state.revision_history.current].label,
        "Add edge treatment"
    );
    let revision = &state.revision_history.revisions[&state.revision_history.current];
    let edit = revision
        .edit
        .as_ref()
        .expect("edge treatment revision should preserve its edit program");
    assert_eq!(edit.label, "Add edge treatment");
    assert!(matches!(
        edit.operations.as_slice(),
        [AssetEdit::InsertModelingOperation {
            definition: PartDefinitionId(1),
            operation: ModelingOperationSpec::BevelBoundaryLoop {
                operation,
                target_loop,
                width,
                segments,
                profile,
                bevel_region,
                outer_replacement_loop,
                inner_replacement_loop,
            },
            ..
        }] if *operation == inserted.0
            && *target_loop == treatment.target_loop
            && *width == treatment.width
            && *segments == treatment.segments
            && *profile == treatment.profile
            && *bevel_region == inserted.4
            && *outer_replacement_loop == inserted.5
            && *inner_replacement_loop == inserted.6
    ));
    let json = serde_json::to_string(&state.project_snapshot()).expect("snapshot serializes");
    let loaded_project = serde_json::from_str(&json).expect("snapshot deserializes");
    let mut loaded =
        AssetAppState::from_template(benchmark_template(BenchmarkAsset::MultiCutPanel))
            .expect("seed state");
    loaded
        .replace_loaded_project(loaded_project, PathBuf::from("panel.shapelab-asset.json"))
        .expect("snapshot with replay edit should load");
    assert!(
        loaded.revision_history.revisions[&loaded.revision_history.current]
            .edit
            .is_some()
    );
    assert!(state.dirty);
    assert!(matches!(
        start_job(effects).kind,
        AssetJobKind::CompileCurrentAsset
    ));
}

#[test]
fn add_boundary_loop_bevel_rejects_hard_only_cut_loop() {
    let mut state = AssetAppState::from_template(benchmark_template(BenchmarkAsset::MultiCutPanel))
        .expect("multi-cut panel template should load");
    let definition = PartDefinitionId(1);
    let before = state.recipe.definitions[&definition]
        .geometry
        .operations
        .len();

    let result = state.handle_command(AssetAppCommand::AddBoundaryLoopBevel {
        definition,
        source_operation: OperationId(6),
        target_loop: BoundaryLoopId(11),
        width: 0.01,
        segments: 2,
        profile: 1.0,
    });

    assert!(matches!(
        result,
        Err(AssetAppStateError::EditRejected(message))
            if message.contains("eligible boundary loop")
    ));
    assert_eq!(
        state.recipe.definitions[&definition]
            .geometry
            .operations
            .len(),
        before
    );
}

#[test]
fn recessed_edge_treatment_limits_treat_missing_sibling_as_zero() {
    let mut entry_only =
        AssetAppState::from_template(benchmark_template(BenchmarkAsset::MultiCutPanel))
            .expect("multi-cut panel template should load");
    let definition = PartDefinitionId(1);
    entry_only
        .recipe
        .definitions
        .get_mut(&definition)
        .unwrap()
        .geometry
        .operations
        .retain(|operation| operation.operation_id() != OperationId(10));
    entry_only.selected_part_instance = entry_only
        .recipe
        .instances
        .values()
        .find(|instance| instance.definition == definition)
        .map(|instance| instance.id);

    let ui_state = asset::view_model::build_asset_ui_state(&entry_only, false);
    let recess = ui_state
        .cut_operations
        .iter()
        .find(|cut| cut.operation == OperationId(1))
        .expect("multi-cut panel should reflect recessed panel cut");
    assert_eq!(edge_treatment_labels(recess), vec!["Entry edge: Rounded"]);
    let width = edge_control(&recess.edge_treatments[0], "bevel_boundary_loop.width");
    assert_approx_eq(width.maximum, 0.054);

    let mut floor_only =
        AssetAppState::from_template(benchmark_template(BenchmarkAsset::MultiCutPanel))
            .expect("multi-cut panel template should load");
    floor_only
        .recipe
        .definitions
        .get_mut(&definition)
        .unwrap()
        .geometry
        .operations
        .retain(|operation| operation.operation_id() != OperationId(9));
    floor_only.selected_part_instance = floor_only
        .recipe
        .instances
        .values()
        .find(|instance| instance.definition == definition)
        .map(|instance| instance.id);

    let ui_state = asset::view_model::build_asset_ui_state(&floor_only, false);
    let recess = ui_state
        .cut_operations
        .iter()
        .find(|cut| cut.operation == OperationId(1))
        .expect("multi-cut panel should reflect recessed panel cut");
    assert_eq!(edge_treatment_labels(recess), vec!["Floor edge: Rounded"]);
    let width = edge_control(&recess.edge_treatments[0], "bevel_boundary_loop.width");
    assert_approx_eq(width.maximum, 0.054);
}

#[test]
fn recessed_edge_treatment_limit_preserves_over_budget_current_width() {
    let mut state = AssetAppState::from_template(benchmark_template(BenchmarkAsset::MultiCutPanel))
        .expect("multi-cut panel template should load");
    let definition = PartDefinitionId(1);
    state.selected_part_instance = state
        .recipe
        .instances
        .values()
        .find(|instance| instance.definition == definition)
        .map(|instance| instance.id);
    for operation in &mut state
        .recipe
        .definitions
        .get_mut(&definition)
        .unwrap()
        .geometry
        .operations
    {
        if let ModelingOperationSpec::BevelBoundaryLoop {
            operation: OperationId(9),
            width,
            ..
        } = operation
        {
            *width = 0.070;
        }
    }

    let ui_state = asset::view_model::build_asset_ui_state(&state, false);
    let recess = ui_state
        .cut_operations
        .iter()
        .find(|cut| cut.operation == OperationId(1))
        .expect("multi-cut panel should reflect recessed panel cut");
    let treatment = recess
        .edge_treatments
        .iter()
        .find(|treatment| treatment.label == "Entry edge: Rounded")
        .expect("entry treatment should be reflected");
    let width = edge_control(treatment, "bevel_boundary_loop.width");

    assert_approx_eq(width.value, 0.070);
    assert_approx_eq(width.maximum, 0.070);
}

#[test]
fn edge_treatment_lock_rules_follow_topology_signature() {
    let mut state = AssetAppState::from_template(benchmark_template(BenchmarkAsset::MultiCutPanel))
        .expect("multi-cut panel template should load");
    let (definition, operation) = first_circular_cut(&state.recipe);
    let treatment = first_reflected_edge_treatment(&state, definition, operation);
    let width = treatment.controls[0].clone();
    let segments = treatment.controls[1].clone();
    let profile = treatment.controls[2].clone();

    state
        .handle_command(AssetAppCommand::SetLock {
            target: AssetLockTarget::Topology(definition),
            locked: true,
        })
        .expect("topology lock should apply");

    let width_effects = state
        .handle_command(AssetAppCommand::SetCutOperationScalar {
            definition,
            operation: treatment.operation,
            field: width.field.clone(),
            value: (width.value * 0.5).max(width.minimum),
        })
        .expect("bevel width should remain editable under topology lock");
    assert!(matches!(
        start_job(width_effects).kind,
        AssetJobKind::CompileCurrentAsset
    ));

    let profile_value = if (profile.value - 1.25).abs() < 0.001 {
        1.5
    } else {
        1.25
    };
    let profile_effects = state
        .handle_command(AssetAppCommand::SetCutOperationScalar {
            definition,
            operation: treatment.operation,
            field: profile.field.clone(),
            value: profile_value,
        })
        .expect("bevel profile should remain editable under topology lock");
    assert!(matches!(
        start_job(profile_effects).kind,
        AssetJobKind::CompileCurrentAsset
    ));

    let segment_value = if (segments.value - 2.0).abs() < 0.001 {
        3.0
    } else {
        2.0
    };
    assert!(matches!(
        state.handle_command(AssetAppCommand::SetCutOperationScalar {
            definition,
            operation: treatment.operation,
            field: segments.field.clone(),
            value: segment_value,
        }),
        Err(AssetAppStateError::EditRejected(message))
            if message.contains("topology is locked")
    ));
    assert!(matches!(
        state.handle_command(AssetAppCommand::RemoveCutOperation {
            definition,
            operation: treatment.operation,
        }),
        Err(AssetAppStateError::EditRejected(message))
            if message.contains("topology is locked")
    ));
}

#[test]
fn remove_edge_treatment_preserves_selected_cut() {
    let mut state = AssetAppState::from_template(benchmark_template(BenchmarkAsset::MultiCutPanel))
        .expect("multi-cut panel template should load");
    let (definition, operation) = first_circular_cut(&state.recipe);
    let treatment = first_reflected_edge_treatment(&state, definition, operation);
    state.selected_cut_operation = Some(operation);

    let effects = state
        .handle_command(AssetAppCommand::RemoveCutOperation {
            definition,
            operation: treatment.operation,
        })
        .expect("edge treatment removal should apply");

    let remaining_operations = &state.recipe.definitions[&definition].geometry.operations;
    assert!(
        remaining_operations
            .iter()
            .any(|candidate| candidate.operation_id() == operation),
        "source cut should remain after removing its edge treatment"
    );
    assert!(
        remaining_operations
            .iter()
            .all(|candidate| candidate.operation_id() != treatment.operation),
        "edge treatment operation should be removed"
    );
    assert_eq!(state.selected_cut_operation, Some(operation));
    assert_eq!(
        state.revision_history.revisions[&state.revision_history.current].label,
        "Remove edge treatment"
    );
    assert!(state.dirty);
    assert!(matches!(
        start_job(effects).kind,
        AssetJobKind::CompileCurrentAsset
    ));
}

#[test]
fn remove_cut_operation_cascades_dependent_edge_treatments() {
    let mut state = AssetAppState::from_template(benchmark_template(BenchmarkAsset::MultiCutPanel))
        .expect("multi-cut panel template should load");
    let (definition, operation) = first_circular_cut(&state.recipe);
    let dependent_bevels = dependent_bevel_operations(&state.recipe, definition, operation);
    assert!(
        !dependent_bevels.is_empty(),
        "multi-cut panel should bevel at least one circular cut"
    );

    let effects = state
        .handle_command(AssetAppCommand::RemoveCutOperation {
            definition,
            operation,
        })
        .expect("cut removal should cascade dependent edge treatments");

    let remaining_operations = &state.recipe.definitions[&definition].geometry.operations;
    assert!(
        remaining_operations
            .iter()
            .all(|candidate| candidate.operation_id() != operation)
    );
    for dependent in dependent_bevels {
        assert!(
            remaining_operations
                .iter()
                .all(|candidate| candidate.operation_id() != dependent),
            "dependent bevel {dependent:?} should be removed"
        );
    }
    assert_ne!(state.selected_cut_operation, Some(operation));
    assert!(state.dirty);
    assert!(matches!(
        start_job(effects).kind,
        AssetJobKind::CompileCurrentAsset
    ));
}

#[test]
fn project_snapshot_round_trip_preserves_branch_history() {
    let mut state = AssetAppState::from_template(benchmark_template(BenchmarkAsset::StylizedStool))
        .expect("stool template should load");
    state
        .handle_command(AssetAppCommand::SetParameter {
            parameter: ParameterId(1),
            value: 1.35,
        })
        .expect("first branch edit");
    let first_branch = state.revision_history.current;
    state
        .handle_command(AssetAppCommand::Undo)
        .expect("undo to root");
    state
        .handle_command(AssetAppCommand::SetParameter {
            parameter: ParameterId(2),
            value: 1.08,
        })
        .expect("second branch edit");
    let second_branch = state.revision_history.current;

    let snapshot = state.project_snapshot();
    assert_eq!(snapshot.schema_version, 2);
    let json = serde_json::to_string(&snapshot).expect("snapshot serializes");
    let project = serde_json::from_str(&json).expect("snapshot deserializes");
    let mut loaded =
        AssetAppState::from_template(benchmark_template(BenchmarkAsset::StylizedStool))
            .expect("seed state");
    loaded
        .replace_loaded_project(project, PathBuf::from("stool.shapelab-asset.json"))
        .expect("snapshot should load");

    assert_eq!(loaded.revision_history.revisions.len(), 3);
    assert!(
        loaded
            .revision_history
            .revisions
            .contains_key(&first_branch)
    );
    assert!(
        loaded
            .revision_history
            .revisions
            .contains_key(&second_branch)
    );
    assert_eq!(loaded.revision_history.current, second_branch);
}

#[test]
fn legacy_project_schema_one_loads_without_revision_edit_metadata() {
    let state = AssetAppState::from_template(benchmark_template(BenchmarkAsset::StylizedStool))
        .expect("stool template should load");
    let mut project = state.project_snapshot();
    project.schema_version = 1;
    let json = serde_json::to_string(&project).expect("legacy snapshot serializes");
    let legacy_project = serde_json::from_str(&json).expect("legacy snapshot deserializes");
    let mut loaded =
        AssetAppState::from_template(benchmark_template(BenchmarkAsset::StylizedStool))
            .expect("seed state");

    loaded
        .replace_loaded_project(legacy_project, PathBuf::from("legacy.shapelab-asset.json"))
        .expect("schema 1 project should migrate at load boundary");

    assert_eq!(loaded.revision_history.revisions.len(), 1);
}

#[test]
fn accepting_candidate_verifies_replay_program_before_committing_history() {
    let mut state = test_state();
    let parent_revision = state.revision_history.current;
    let good_program = AssetEditProgram {
        label: "good".to_owned(),
        seed: 1,
        operations: vec![AssetEdit::SetScalar {
            parameter: THICKNESS,
            value: 0.30,
        }],
    };
    let bad_program = AssetEditProgram {
        label: "bad".to_owned(),
        seed: 1,
        operations: vec![AssetEdit::SetScalar {
            parameter: THICKNESS,
            value: 0.24,
        }],
    };
    let candidate_recipe =
        apply_edit_program(&state.recipe, &good_program).expect("fixture edit should apply");
    state.candidate_slots = vec![AssetCandidateSlot {
        slot: 0,
        candidate: JobAssetCandidate {
            id: asset::AssetCandidateId(1),
            slot: 0,
            label: "Mismatched direction".to_owned(),
            program: bad_program,
            recipe: candidate_recipe,
            changed_parameters: BTreeSet::new(),
            changes: Vec::new(),
            quality_penalty: 0.0,
            artifact: None,
        },
        preview: None,
        preview_failure: None,
    }];

    assert!(matches!(
        state.handle_command(AssetAppCommand::AcceptCandidate(asset::AssetCandidateId(1))),
        Err(AssetAppStateError::EditRejected(message))
            if message.contains("does not reproduce")
    ));
    assert_eq!(state.revision_history.current, parent_revision);
    assert_eq!(state.revision_history.revisions.len(), 1);
}

#[test]
fn export_jobs_write_obj_and_canonical_package() {
    let mut state =
        AssetAppState::from_template(benchmark_template(BenchmarkAsset::ExplicitDeskLamp))
            .expect("lamp template should load");
    let out_dir = unique_test_dir("shape-lab-asset-export");
    fs::create_dir_all(&out_dir).expect("test export dir");

    let obj_path = out_dir.join("lamp.obj");
    let obj_request = start_job(
        state
            .handle_command(AssetAppCommand::ExportObj(obj_path.clone()))
            .expect("obj export should schedule"),
    );
    let obj_events = run_asset_job(obj_request);
    assert!(obj_events.iter().any(|event| {
        matches!(event, AssetJobEvent::ExportObjReady { path, .. } if path == &obj_path)
    }));
    assert!(obj_path.exists());

    let package_dir = out_dir.join("package");
    let package_request = start_job(
        state
            .handle_command(AssetAppCommand::ExportPackage(package_dir.clone()))
            .expect("package export should schedule"),
    );
    let package_events = run_asset_job(package_request);
    assert!(package_events.iter().any(|event| {
        matches!(event, AssetJobEvent::ExportPackageReady { path, package_paths, .. }
            if path == &package_dir && package_paths.manifest.exists())
    }));
    assert!(package_dir.join("asset-manifest.json").exists());

    let _ = fs::remove_dir_all(out_dir);
}

fn test_state() -> AssetAppState {
    AssetAppState::new(recipe("Test Asset", 0.12)).expect("valid test state")
}

fn template(id: &str, title: &str, thickness: f32) -> AssetTemplate {
    AssetTemplate {
        id: id.to_owned(),
        title: title.to_owned(),
        recipe: recipe(title, thickness),
    }
}

fn benchmark_template(asset: BenchmarkAsset) -> AssetTemplate {
    let recipe = asset.recipe();
    AssetTemplate {
        id: asset.slug().to_owned(),
        title: recipe.title.clone(),
        recipe,
    }
}

fn first_circular_cut(recipe: &AssetRecipe) -> (PartDefinitionId, OperationId) {
    recipe
        .definitions
        .iter()
        .find_map(|(definition, spec)| {
            spec.geometry.operations.iter().find_map(|operation| {
                matches!(operation, ModelingOperationSpec::CircularThroughCut { .. })
                    .then_some((*definition, operation.operation_id()))
            })
        })
        .expect("benchmark should include a circular cut")
}

fn dependent_bevel_operations(
    recipe: &AssetRecipe,
    definition: PartDefinitionId,
    operation: OperationId,
) -> Vec<OperationId> {
    let definition = &recipe.definitions[&definition];
    let source_loops = definition
        .geometry
        .operations
        .iter()
        .find(|candidate| candidate.operation_id() == operation)
        .expect("source operation should exist")
        .direct_boundary_loop_outputs();
    definition
        .geometry
        .operations
        .iter()
        .filter_map(|candidate| {
            let ModelingOperationSpec::BevelBoundaryLoop {
                operation,
                target_loop,
                ..
            } = candidate
            else {
                return None;
            };
            source_loops.contains(target_loop).then_some(*operation)
        })
        .collect()
}

fn first_reflected_edge_treatment(
    state: &AssetAppState,
    definition: PartDefinitionId,
    operation: OperationId,
) -> asset::AssetEdgeTreatment {
    let part = state
        .recipe
        .instances
        .values()
        .find(|instance| instance.definition == definition)
        .expect("definition should have an instance")
        .id;
    let mut state = state.clone();
    state.selected_part_instance = Some(part);
    state.selected_cut_operation = Some(operation);
    let ui_state = asset::view_model::build_asset_ui_state(&state, false);
    ui_state
        .cut_operations
        .iter()
        .find(|candidate| candidate.operation == operation)
        .and_then(|cut| cut.edge_treatments.first())
        .cloned()
        .expect("selected cut should expose an edge treatment")
}

fn edge_treatment_labels(cut: &asset::AssetCutOperation) -> Vec<&str> {
    cut.edge_treatments
        .iter()
        .map(|treatment| treatment.label.as_str())
        .collect()
}

fn edge_control<'a>(
    treatment: &'a asset::AssetEdgeTreatment,
    field: &str,
) -> &'a asset::AssetCutControl {
    treatment
        .controls
        .iter()
        .find(|control| control.field == field)
        .unwrap_or_else(|| panic!("missing edge treatment control {field}"))
}

fn cut_control<'a>(cut: &'a asset::AssetCutOperation, field: &str) -> &'a asset::AssetCutControl {
    cut.controls
        .iter()
        .find(|control| control.field == field)
        .unwrap_or_else(|| panic!("missing cut control {field}"))
}

fn assert_approx_eq(actual: f32, expected: f32) {
    assert!(
        (actual - expected).abs() <= 0.000_01,
        "expected {actual} to equal {expected}"
    );
}

fn unique_test_dir(stem: &str) -> PathBuf {
    std::env::temp_dir().join(format!(
        "{}-{}",
        stem,
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("system time")
            .as_nanos()
    ))
}

fn recipe(title: &str, thickness: f32) -> AssetRecipe {
    let mut recipe = AssetRecipe::new(AssetId(1), title);
    recipe.definitions.insert(
        BODY_DEFINITION,
        definition(BODY_DEFINITION, "Body", thickness),
    );
    recipe.definitions.insert(
        ALT_BODY_DEFINITION,
        definition(ALT_BODY_DEFINITION, "Alternate Body", thickness * 1.5),
    );
    recipe.variation.replacement_groups.insert(
        "body".to_owned(),
        ReplacementGroupHint {
            definitions: BTreeSet::from([BODY_DEFINITION, ALT_BODY_DEFINITION]),
        },
    );
    recipe.instances.insert(
        BODY_INSTANCE,
        instance(BODY_INSTANCE, BODY_DEFINITION, "Body"),
    );
    recipe.root_instances.push(BODY_INSTANCE);
    recipe.parameters.insert(
        THICKNESS,
        parameter(
            THICKNESS,
            definition_scalar_path(BODY_DEFINITION, "geometry.plate.thickness"),
            "Thickness",
            0.05,
            1.0,
            0.01,
            0.08,
        ),
    );
    recipe.parameters.insert(
        TRANSLATE_X,
        parameter(
            TRANSLATE_X,
            instance_scalar_path(BODY_INSTANCE, "transform.translation.x"),
            "Translate X",
            -2.0,
            2.0,
            0.05,
            0.25,
        ),
    );
    recipe.next_ids.part_definition = 3;
    recipe.next_ids.part_instance = 3;
    recipe.next_ids.parameter = 3;
    recipe
}

fn definition(id: PartDefinitionId, name: &str, thickness: f32) -> PartDefinition {
    PartDefinition {
        id,
        name: name.to_owned(),
        tags: BTreeSet::new(),
        geometry: GeometryRecipe {
            source: GeometrySource::Plate {
                size: [1.0, 0.8],
                thickness,
            },
            operations: Vec::new(),
        },
        regions: BTreeMap::new(),
        sockets: BTreeMap::new(),
        local_pivot: Frame3::default(),
        variant_group: Some("body".to_owned()),
        production_hints: None,
    }
}

fn instance(id: PartInstanceId, definition: PartDefinitionId, name: &str) -> PartInstance {
    PartInstance {
        id,
        definition,
        name: name.to_owned(),
        parent: None,
        local_transform: Transform3::default(),
        attachment: None,
        enabled: true,
        tags: BTreeSet::new(),
        generated_by: None,
    }
}

fn optional_instance() -> PartInstance {
    let mut instance = instance(OPTIONAL_INSTANCE, BODY_DEFINITION, "Optional handle");
    instance.local_transform.translation = [1.2, 0.0, 0.0];
    instance
}

fn parameter(
    id: ParameterId,
    path: String,
    label: &str,
    minimum: f32,
    maximum: f32,
    step: f32,
    mutation_sigma: f32,
) -> ParameterDescriptor {
    ParameterDescriptor {
        id,
        path,
        label: label.to_owned(),
        group: "form".to_owned(),
        minimum,
        maximum,
        step,
        mutation_sigma,
        topology_changing: false,
        beginner_description: String::new(),
    }
}

fn scalar(state: &AssetAppState, parameter: ParameterId) -> f32 {
    let descriptor = &state.recipe.parameters[&parameter];
    get_scalar(&state.recipe, &descriptor.path).expect("scalar path should be readable")
}

fn start_job(effects: Vec<AssetAppEffect>) -> AssetJobRequest {
    effects
        .into_iter()
        .find_map(|effect| match effect {
            AssetAppEffect::StartJob(request) => Some(*request),
            AssetAppEffect::SaveProject { .. } | AssetAppEffect::LoadProject(_) => None,
        })
        .expect("start job effect")
}
