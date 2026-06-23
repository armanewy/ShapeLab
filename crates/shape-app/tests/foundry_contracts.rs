#![forbid(unsafe_code)]
#![allow(dead_code)]

#[path = "../src/foundry/mod.rs"]
mod foundry;

use foundry::{
    FoundryAppCommand, FoundryAppState, FoundryJobEvent, FoundryJobRequest, FoundryPackView,
};
use shape_foundry::{
    CatalogContentRef, FoundryAssetDocument, FoundryCommand, FoundryDocumentId,
    GenerateCandidatesRequest,
};
use shape_render::OrbitCamera;
use shape_search::foundry::{FoundryCandidateMode, FoundryCandidateRequest};

#[test]
fn foundry_app_command_wraps_generic_foundry_command() {
    let command = FoundryAppCommand::run(FoundryCommand::Undo);

    assert_eq!(
        command.single_foundry_command(),
        Some(&FoundryCommand::Undo)
    );

    let generate = FoundryCommand::GenerateCandidates(GenerateCandidatesRequest {
        strategy_id: Some("novice_bridge".to_string()),
        count: 6,
        seed: 9,
    });
    let program = FoundryAppCommand::RunFoundryCommandProgram {
        label: "generate".to_string(),
        commands: vec![generate],
    };

    match program {
        FoundryAppCommand::RunFoundryCommandProgram { commands, .. } => {
            assert!(matches!(
                commands.as_slice(),
                [FoundryCommand::GenerateCandidates(_)]
            ));
        }
        _ => panic!("expected command program"),
    }
}

#[test]
fn foundry_app_state_starts_without_current_document() {
    let state = FoundryAppState::default();

    assert!(state.document.is_none());
    assert!(state.current_output.is_none());
    assert!(state.active_jobs.is_empty());
    assert!(!state.dirty);
    assert!(!state.advanced_recipe_open);
}

#[test]
fn foundry_job_events_expose_matching_job_id() {
    let events = [
        FoundryJobEvent::PackCompiled {
            job_id: 42,
            pack: Box::new(FoundryPackView::default()),
        },
        FoundryJobEvent::PackExportFinished {
            job_id: 43,
            profile: "game-ready".to_string(),
            out_dir: std::path::PathBuf::from("pack-out"),
            member_count: 3,
        },
        FoundryJobEvent::PreviewRendered {
            job_id: 44,
            preview_id: "front".to_string(),
            rgba8: Vec::new(),
            width: 0,
            height: 0,
            camera: OrbitCamera::default(),
            build: None,
        },
        FoundryJobEvent::ExportFinished {
            job_id: 45,
            profile: "blender".to_string(),
            out_dir: std::path::PathBuf::from("out"),
        },
        FoundryJobEvent::Failed {
            job_id: 46,
            message: "failed".to_string(),
        },
    ];

    assert_eq!(
        events
            .iter()
            .map(FoundryJobEvent::job_id)
            .collect::<Vec<_>>(),
        vec![42, 43, 44, 45, 46]
    );
}

#[test]
fn foundry_candidate_job_mode_comes_from_request() {
    let request = FoundryCandidateRequest {
        seed: 11,
        proposal_count: 24,
        result_count: 6,
        mode: FoundryCandidateMode::Explore,
        strategy_id: Some("default".to_string()),
    };
    let job = FoundryJobRequest::GenerateCandidates {
        job_id: 46,
        document: Box::new(minimal_foundry_document()),
        request,
    };

    assert_eq!(job.job_id(), 46);
    assert_eq!(job.candidate_mode(), Some(FoundryCandidateMode::Explore));
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
