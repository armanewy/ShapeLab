#![forbid(unsafe_code)]
#![allow(dead_code)]

#[path = "../src/foundry/mod.rs"]
mod foundry;

use foundry::{
    FoundryAppCommand, FoundryAppEffect, FoundryAppState, FoundryJobEvent, FoundryJobRequest,
};
use shape_foundry::{
    CatalogContentRef, ControlValue, FoundryAssetDocument, FoundryCatalogLock, FoundryCommand,
    FoundryConformanceSummary, FoundryDocumentId, FoundryEdit, FoundryProjectRevisionProgram,
};
use shape_project::foundry::FoundryProjectFile;
use shape_search::foundry::{FoundryCandidateMode, FoundryCandidateRequest};

#[test]
fn request_build_schedules_compile_job_without_compiling_inline() {
    let mut state = FoundryAppState::new(minimal_foundry_document()).expect("valid state");

    let effects = state.request_build().expect("build should schedule");

    let [FoundryAppEffect::StartJob(job)] = effects.as_slice() else {
        panic!("expected one compile job effect");
    };
    assert!(matches!(
        job.as_ref(),
        FoundryJobRequest::CompileCurrent { job_id: 1, .. }
    ));
    assert!(state.active_jobs.contains_key(&1));
    assert!(state.current_output.is_none());
}

#[test]
fn semantic_command_schedules_apply_edit_and_stales_superseded_jobs() {
    let mut state = FoundryAppState::new(minimal_foundry_document()).expect("valid state");
    state.request_build().expect("first build should schedule");

    let effects = state
        .handle_command(FoundryAppCommand::run(FoundryCommand::SetControl {
            control_id: "span".to_owned(),
            value: ControlValue::Scalar(0.7),
        }))
        .expect("edit should schedule");

    assert!(state.stale_jobs.contains(&1));
    assert!(!state.active_jobs.contains_key(&1));
    let [FoundryAppEffect::StartJob(job)] = effects.as_slice() else {
        panic!("expected one apply-edit job effect");
    };
    match job.as_ref() {
        FoundryJobRequest::ApplyEdit { job_id, edit, .. } => {
            assert_eq!(*job_id, 2);
            assert!(matches!(
                edit.commands.as_slice(),
                [FoundryCommand::SetControl { control_id, .. }] if control_id == "span"
            ));
        }
        other => panic!("expected apply edit job, got {other:?}"),
    }
}

#[test]
fn candidate_job_mode_comes_from_explicit_candidate_request() {
    let mut state = FoundryAppState::new(minimal_foundry_document()).expect("valid state");
    let request = FoundryCandidateRequest {
        seed: 99,
        proposal_count: 24,
        result_count: 4,
        mode: FoundryCandidateMode::Explore,
        strategy_id: Some("broad".to_owned()),
    };

    let effects = state
        .request_candidates(request)
        .expect("candidate generation should schedule");

    let [FoundryAppEffect::StartJob(job)] = effects.as_slice() else {
        panic!("expected one candidate job effect");
    };
    assert_eq!(job.candidate_mode(), Some(FoundryCandidateMode::Explore));
}

#[test]
fn accepting_candidate_schedules_its_replayable_foundry_edit() {
    let mut state = FoundryAppState::new(minimal_foundry_document()).expect("valid state");
    let candidate_id = shape_foundry::FoundryCandidateId("candidate-a".to_owned());
    let edit = FoundryEdit {
        label: "Candidate A".to_owned(),
        commands: vec![FoundryCommand::SetControl {
            control_id: "span".to_owned(),
            value: ControlValue::Scalar(0.25),
        }],
    };
    state
        .candidate_edits
        .insert(candidate_id.clone(), edit.clone());

    let effects = state
        .handle_command(FoundryAppCommand::run(FoundryCommand::AcceptCandidate {
            candidate_id,
        }))
        .expect("candidate should schedule edit application");

    let [FoundryAppEffect::StartJob(job)] = effects.as_slice() else {
        panic!("expected one apply-edit job effect");
    };
    match job.as_ref() {
        FoundryJobRequest::ApplyEdit {
            edit: scheduled, ..
        } => {
            assert_eq!(scheduled.as_ref(), &edit);
        }
        other => panic!("expected apply edit job, got {other:?}"),
    }
}

#[test]
fn stale_job_event_is_rejected_without_touching_current_job() {
    let mut state = FoundryAppState::new(minimal_foundry_document()).expect("valid state");
    state.request_build().expect("first build should schedule");
    state.request_build().expect("second build should schedule");

    let accepted = state.handle_job_event(FoundryJobEvent::Failed {
        job_id: 1,
        message: "old compile failed".to_owned(),
    });

    assert!(!accepted);
    assert!(state.stale_jobs.contains(&1));
    assert!(state.active_jobs.contains_key(&2));
    assert!(state.status.is_none());
}

#[test]
fn add_current_to_pack_tracks_membership_and_schedules_pack_compile() {
    let mut state = FoundryAppState::new(minimal_foundry_document()).expect("valid state");

    let effects = state
        .handle_command(FoundryAppCommand::run(FoundryCommand::AddCurrentToPack {
            pack_id: "props".to_owned(),
            member_id: "crate".to_owned(),
        }))
        .expect("pack membership should schedule compile");

    assert_eq!(state.pack.pack_id.as_deref(), Some("props"));
    assert!(state.pack.members.contains_key("crate"));
    assert_eq!(state.selected_pack_member.as_deref(), Some("crate"));
    let [FoundryAppEffect::StartJob(job)] = effects.as_slice() else {
        panic!("expected one pack compile job effect");
    };
    assert!(matches!(
        job.as_ref(),
        FoundryJobRequest::CompilePack { job_id: 1, .. }
    ));
}

#[test]
fn undo_switches_to_parent_revision_and_rebuilds_off_thread() {
    let root = minimal_foundry_document();
    let edit = FoundryEdit {
        label: "Set span".to_owned(),
        commands: vec![FoundryCommand::SetControl {
            control_id: "span".to_owned(),
            value: ControlValue::Scalar(0.5),
        }],
    };
    let mut child = root.clone();
    child
        .control_state
        .insert("span".to_owned(), ControlValue::Scalar(0.5));

    let mut project_file = FoundryProjectFile::new(
        "History",
        root.clone(),
        FoundryCatalogLock::from_document_refs(&root),
        None,
        None,
        FoundryConformanceSummary::default(),
    )
    .expect("project should be valid");
    project_file
        .accept_program(
            FoundryProjectRevisionProgram::from_edit(edit),
            child,
            FoundryCatalogLock::from_document_refs(&root),
            None,
            None,
            FoundryConformanceSummary::default(),
        )
        .expect("child revision should be accepted");
    let mut state = FoundryAppState::from_project_file(project_file).expect("state should load");

    let effects = state
        .handle_command(FoundryAppCommand::run(FoundryCommand::Undo))
        .expect("undo should schedule rebuild");

    assert_eq!(state.current_revision, Some(shape_asset::RevisionId(0)));
    assert!(
        state
            .document
            .as_ref()
            .is_some_and(|document| !document.control_state.contains_key("span"))
    );
    assert!(state.current_output.is_none());
    let [FoundryAppEffect::StartJob(job)] = effects.as_slice() else {
        panic!("expected one compile job effect");
    };
    assert!(matches!(
        job.as_ref(),
        FoundryJobRequest::CompileCurrent { job_id: 1, .. }
    ));
}

fn minimal_foundry_document() -> FoundryAssetDocument {
    FoundryAssetDocument::new(
        FoundryDocumentId("doc".to_string()),
        content_ref("family", 1),
        content_ref("style", 2),
        content_ref("family_impl", 3),
        content_ref("style_impl", 4),
        content_ref("profile", 5),
    )
}

fn content_ref(stable_id: &str, byte: u8) -> CatalogContentRef {
    let fingerprint = format!("{byte:02x}").repeat(32);
    serde_json::from_value(serde_json::json!({
        "stable_id": stable_id,
        "schema_version": 1,
        "fingerprint": fingerprint,
    }))
    .expect("test catalog reference is valid")
}
