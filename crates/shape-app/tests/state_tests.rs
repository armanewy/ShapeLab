#![forbid(unsafe_code)]

#[path = "../src/commands.rs"]
mod commands;
#[path = "../src/jobs.rs"]
mod jobs;
#[path = "../src/state.rs"]
mod state;
#[path = "../src/viewport.rs"]
mod viewport;

use std::path::PathBuf;

use commands::{AppCommand, AppEffect};
use jobs::{CandidatePreview, GenerationId, JobEvent, JobId, JobRequest};
use shape_core::{
    Aabb, CandidateId, EditProgram, ParamDescriptor, ParamPath, RevisionId, SetScalarEdit,
    apply_edit, enumerate_parameters, get_scalar,
};
use shape_mesh::TriangleMesh;
use shape_presets::PresetId;
use shape_render::RenderedImage;
use shape_search::{Candidate, ShapeDescriptor};
use state::{AppState, AppStateError, CurrentPreview};

fn editable_parameter(state: &AppState) -> ParamDescriptor {
    let document = state.project.current_document().unwrap();
    enumerate_parameters(document)
        .into_iter()
        .find(|descriptor| descriptor.path.key.starts_with("primitive."))
        .unwrap()
}

fn radius_path(state: &AppState) -> ParamPath {
    let document = state.project.current_document().unwrap();
    enumerate_parameters(document)
        .into_iter()
        .find(|descriptor| descriptor.path.key == "primitive.radius")
        .map(|descriptor| descriptor.path)
        .unwrap()
}

fn next_value(state: &AppState, descriptor: &ParamDescriptor, magnitude: f32) -> f32 {
    let before = get_scalar(state.project.current_document().unwrap(), &descriptor.path).unwrap();
    if before + magnitude <= descriptor.maximum {
        before + magnitude
    } else {
        before - magnitude
    }
}

fn candidate_preview(
    state: &AppState,
    candidate_id: u64,
    slot: usize,
    label: &str,
    magnitude: f32,
) -> CandidatePreview {
    let descriptor = editable_parameter(state);
    let document = state.project.current_document().unwrap();
    let before = get_scalar(document, &descriptor.path).unwrap();
    let after = next_value(state, &descriptor, magnitude);
    let edit = EditProgram {
        label: label.to_owned(),
        seed: candidate_id,
        operations: vec![SetScalarEdit {
            path: descriptor.path,
            before,
            after,
        }],
    };
    let document = apply_edit(document, &edit).unwrap();
    CandidatePreview {
        slot,
        candidate: Candidate {
            id: CandidateId(candidate_id),
            document,
            edit,
            descriptor: ShapeDescriptor {
                values: vec![after],
            },
            distance_from_parent: (after - before).abs(),
        },
        mesh: empty_mesh(),
        image: blank_image(),
    }
}

fn empty_mesh() -> TriangleMesh {
    TriangleMesh {
        positions: Vec::new(),
        normals: Vec::new(),
        indices: Vec::new(),
        bounds: Aabb::empty(),
    }
}

fn blank_image() -> RenderedImage {
    RenderedImage {
        width: 1,
        height: 1,
        rgba8: vec![0, 0, 0, 255],
    }
}

fn build_preview_effect(effects: &[AppEffect]) -> JobId {
    match effects {
        [AppEffect::StartJob(request)] => match &**request {
            JobRequest::BuildCurrentPreview { job_id, .. } => *job_id,
            other => panic!("expected preview job, got {other:?}"),
        },
        other => panic!("expected one start-job effect, got {other:?}"),
    }
}

fn generation_effect(effects: &[AppEffect]) -> (JobId, GenerationId, u64) {
    match effects {
        [AppEffect::StartJob(request)] => match &**request {
            JobRequest::GenerateCandidates {
                job_id,
                generation_id,
                request,
                ..
            } => (*job_id, *generation_id, request.seed),
            other => panic!("expected generation job, got {other:?}"),
        },
        other => panic!("expected one start-job effect, got {other:?}"),
    }
}

#[test]
fn command_sequence_edits_parameter_and_schedules_preview() {
    let mut state = AppState::default();
    let descriptor = editable_parameter(&state);
    let after = next_value(&state, &descriptor, 0.05);

    let effects = state
        .handle_command(AppCommand::SetScalar {
            path: descriptor.path.clone(),
            value: after,
        })
        .unwrap();

    build_preview_effect(&effects);
    assert!(state.dirty);
    assert!(state.current_preview.is_none());
    assert_eq!(
        get_scalar(state.project.current_document().unwrap(), &descriptor.path).unwrap(),
        after
    );
}

#[test]
fn invalid_parameter_edit_does_not_mutate_project() {
    let mut state = AppState::default();
    let before = state.project.clone();
    let path = radius_path(&state);

    let error = state
        .handle_command(AppCommand::SetScalar { path, value: -1.0 })
        .unwrap_err();

    assert!(matches!(error, AppStateError::Core(_)));
    assert_eq!(state.project, before);
    assert_eq!(state.status.phase, state::AppPhase::Error);
}

#[test]
fn accepting_candidate_creates_revision_and_rebuilds_preview() {
    let mut state = AppState::default();
    state
        .candidate_slots
        .push(candidate_preview(&state, 10, 0, "Broader direction", 0.04));

    let effects = state
        .handle_command(AppCommand::AcceptCandidate(CandidateId(10)))
        .unwrap();

    build_preview_effect(&effects);
    assert_eq!(state.project.current_revision, RevisionId(1));
    assert_eq!(
        state.project.children_of(RevisionId(0)),
        vec![RevisionId(1)]
    );
    assert!(state.candidate_slots.is_empty());
    assert!(state.current_preview.is_none());
    assert!(state.dirty);
}

#[test]
fn undo_then_accepting_another_candidate_creates_branch() {
    let mut state = AppState::default();
    state
        .candidate_slots
        .push(candidate_preview(&state, 11, 0, "First direction", 0.04));
    state
        .handle_command(AppCommand::AcceptCandidate(CandidateId(11)))
        .unwrap();
    state.handle_command(AppCommand::Undo).unwrap();
    state
        .candidate_slots
        .push(candidate_preview(&state, 12, 0, "Second direction", 0.08));

    state
        .handle_command(AppCommand::AcceptCandidate(CandidateId(12)))
        .unwrap();

    assert_eq!(
        state.project.children_of(RevisionId(0)),
        vec![RevisionId(1), RevisionId(2)]
    );
    assert_eq!(state.project.current_revision, RevisionId(2));
}

#[test]
fn preset_reset_rebuilds_from_active_preset() {
    let mut state = AppState::default();
    state
        .handle_command(AppCommand::LoadPreset(PresetId("toy-submarine".to_owned())))
        .unwrap();
    let title_after_load = state.project.title.clone();
    let descriptor = editable_parameter(&state);
    let after = next_value(&state, &descriptor, 0.03);
    state
        .handle_command(AppCommand::SetScalar {
            path: descriptor.path,
            value: after,
        })
        .unwrap();

    let effects = state
        .handle_command(AppCommand::ResetCurrentPreset)
        .unwrap();

    build_preview_effect(&effects);
    assert_eq!(title_after_load, "Toy Submarine");
    assert_eq!(state.project.title, "Toy Submarine");
    assert_eq!(
        state.active_preset,
        Some(PresetId("toy-submarine".to_owned()))
    );
    assert!(state.dirty);
}

#[test]
fn locks_block_parameter_edits_until_unlocked() {
    let mut state = AppState::default();
    let descriptor = editable_parameter(&state);
    let path = descriptor.path.clone();
    let after = next_value(&state, &descriptor, 0.05);

    state
        .handle_command(AppCommand::ToggleLock {
            path: path.clone(),
            locked: true,
        })
        .unwrap();
    assert!(
        state
            .project
            .current_document()
            .unwrap()
            .locks
            .contains(&path)
    );

    let error = state
        .handle_command(AppCommand::SetScalar {
            path: path.clone(),
            value: after,
        })
        .unwrap_err();
    assert!(matches!(error, AppStateError::LockedParameter(_)));

    state
        .handle_command(AppCommand::ToggleLock {
            path: path.clone(),
            locked: false,
        })
        .unwrap();
    assert!(
        !state
            .project
            .current_document()
            .unwrap()
            .locks
            .contains(&path)
    );
    assert!(
        state
            .handle_command(AppCommand::SetScalar { path, value: after })
            .is_ok()
    );
}

#[test]
fn stale_candidate_generation_is_cleared_and_ignored() {
    let mut state = AppState::default();
    let effects = state
        .handle_command(AppCommand::GenerateDirections)
        .unwrap();
    let (job_id, generation_id, _) = generation_effect(&effects);
    state
        .candidate_slots
        .push(candidate_preview(&state, 20, 0, "Old direction", 0.04));
    let descriptor = editable_parameter(&state);
    let after = next_value(&state, &descriptor, 0.05);

    state
        .handle_command(AppCommand::SetScalar {
            path: descriptor.path,
            value: after,
        })
        .unwrap();

    assert!(state.active_generation.is_none());
    assert!(state.candidate_slots.is_empty());
    let applied = state.handle_job_event(JobEvent::CandidatePreviewReady {
        job_id,
        generation_id,
        preview: candidate_preview(&state, 21, 0, "Stale direction", 0.04),
    });
    assert!(!applied);
    assert!(state.candidate_slots.is_empty());
}

#[test]
fn generation_uses_deterministic_seed_increments() {
    let mut state = AppState::default();
    state.handle_command(AppCommand::SetSeed(900)).unwrap();

    let first = state
        .handle_command(AppCommand::GenerateDirections)
        .unwrap();
    let (_, first_generation, first_seed) = generation_effect(&first);
    let second = state
        .handle_command(AppCommand::GenerateDirections)
        .unwrap();
    let (_, second_generation, second_seed) = generation_effect(&second);

    assert_eq!(first_generation, GenerationId(1));
    assert_eq!(second_generation, GenerationId(2));
    assert_eq!(first_seed, 900);
    assert_eq!(second_seed, 901);
    assert_eq!(state.seed, 902);
}

#[test]
fn save_load_and_export_commands_emit_effects_only() {
    let mut state = AppState::default();
    assert!(matches!(
        state.handle_command(AppCommand::Save).unwrap_err(),
        AppStateError::MissingSavePath
    ));

    let save_path = PathBuf::from("project.shapelab.json");
    assert_eq!(
        state
            .handle_command(AppCommand::SaveAs(save_path.clone()))
            .unwrap(),
        vec![AppEffect::SaveProject(save_path.clone())]
    );
    state.mark_saved(save_path.clone());
    assert!(!state.dirty);
    assert_eq!(
        state.handle_command(AppCommand::Save).unwrap(),
        vec![AppEffect::SaveProject(save_path.clone())]
    );

    let open_path = PathBuf::from("other.shapelab.json");
    assert_eq!(
        state
            .handle_command(AppCommand::OpenProject(open_path.clone()))
            .unwrap(),
        vec![AppEffect::LoadProject(open_path)]
    );

    let export_path = PathBuf::from("model.obj");
    assert!(matches!(
        state
            .handle_command(AppCommand::ExportCurrentObj(export_path.clone()))
            .unwrap_err(),
        AppStateError::MissingPreviewForExport
    ));
    state.current_preview = Some(CurrentPreview {
        mesh: empty_mesh(),
        image: blank_image(),
    });
    assert_eq!(
        state
            .handle_command(AppCommand::ExportCurrentObj(export_path.clone()))
            .unwrap(),
        vec![AppEffect::ExportCurrentObj(export_path)]
    );
}
