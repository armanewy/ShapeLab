#![forbid(unsafe_code)]

#[path = "../src/asset/mod.rs"]
mod asset;
#[path = "../src/viewport.rs"]
mod viewport;

use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;

use asset::{
    AssetAppCommand, AssetAppEffect, AssetAppState, AssetAppStateError, AssetGenerationMode,
    AssetJobEvent, AssetJobKind, AssetJobRequest, AssetLockTarget, AssetTemplate, run_asset_job,
};
use shape_asset::{
    AssetId, AssetRecipe, Frame3, GeometryRecipe, GeometrySource, ParameterDescriptor, ParameterId,
    PartDefinition, PartDefinitionId, PartInstance, PartInstanceId, ReplacementGroupHint,
    Transform3, definition_scalar_path, get_scalar, instance_scalar_path,
};

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
    state
        .handle_command(AssetAppCommand::AcceptCandidate(candidate.id))
        .expect("candidate should apply");

    assert_eq!(state.recipe, candidate.recipe);
    assert!(state.candidate_slots.is_empty());
    assert!(state.active_generation.is_none());
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
        .handle_command(AssetAppCommand::LoadTemplate(template(
            "lamp", "Lamp", 0.36,
        )))
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
        [AssetAppEffect::SaveRecipe { path: saved, .. }] if saved == &path
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
            AssetAppEffect::SaveRecipe { .. } | AssetAppEffect::LoadRecipe(_) => None,
        })
        .expect("start job effect")
}
