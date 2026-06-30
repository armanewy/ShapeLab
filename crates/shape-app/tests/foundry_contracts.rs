#![forbid(unsafe_code)]
#![allow(dead_code)]

#[path = "../src/foundry/mod.rs"]
mod foundry;

use foundry::{
    FoundryAppCommand, FoundryAppState, FoundryJobEvent, FoundryJobRequest, FoundryPackView,
};
use std::fs;
use std::path::{Path, PathBuf};

use shape_foundry::{
    CatalogContentRef, FoundryAssetDocument, FoundryCommand, FoundryDocumentId,
    GenerateCandidatesRequest, VariationIntent,
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
        strategy_id: Some("box_primitive".to_string()),
        count: 6,
        seed: 9,
        variation_intent: VariationIntent::default(),
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
            profile: "local-pack".to_string(),
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
            profile: "local-file".to_string(),
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
        preference_profile: None,
        variation_intent: VariationIntent::default(),
    };
    let job = FoundryJobRequest::GenerateCandidates {
        job_id: 46,
        document: Box::new(minimal_foundry_document()),
        request,
    };

    assert_eq!(job.job_id(), 46);
    assert_eq!(job.candidate_mode(), Some(FoundryCandidateMode::Explore));
}

#[test]
fn foundry_direct_make_status_docs_agree() {
    let readme = doc_text("README.md");
    let status = doc_text("docs/CURRENT_PRODUCT_STATUS.md");
    let vision = doc_text("docs/PRIMITIVE_DIRECT_MAKE_VISION.md");
    let retirement = doc_text("docs/ACTIVE_VARIATION_UI_RETIREMENT.md");
    let limitations = doc_text("docs/KNOWN_LIMITATIONS.md");

    for text in [&readme, &status] {
        assert!(
            text.contains("Box Primitive is the direct box"),
            "README and status should agree on Box Primitive as direct baseline"
        );
        assert!(
            text.contains("Flat Panel Primitive is the direct"),
            "README and status should agree on Flat Panel Primitive as direct baseline"
        );
        assert!(
            text.contains("Generated idea workflows are retired from active primitive UI"),
            "README and status should agree that generated idea workflows are retired"
        );
        assert!(
            text.contains("Candidate generation is inactive in the current primitive product flow"),
            "README and status should mark candidate generation inactive in active primitives"
        );
        assert!(
            text.contains("Family Studio Lite is paused"),
            "README and status should keep Family Studio Lite paused"
        );
    }

    for text in [&readme, &status, &vision, &retirement, &limitations] {
        assert!(
            text.contains("property-schema")
                || text.contains("immutable property schemas")
                || text.contains("primitive property schemas")
                || text.contains("Primitive editing is property-schema based"),
            "docs should describe primitive editing as schema based"
        );
        assert!(
            text.contains("deterministic property presets"),
            "docs should limit future suggestions to deterministic presets"
        );
        assert!(
            text.contains("vertices") && text.contains("faces"),
            "docs should reject Blender-like vertex and face editing"
        );
        assert!(
            text.contains("runtime LLM"),
            "docs should keep runtime LLM blocked"
        );
        assert!(
            text.contains("UV/texturing") && text.contains("rigging") && text.contains("animation"),
            "docs should keep material, UV, rigging, and animation blocked"
        );
    }

    for text in [&status, &vision, &retirement] {
        assert!(
            text.contains("contact sheet") || text.contains("contact-sheet"),
            "active variation retirement docs should allow internal contact-sheet machinery"
        );
        assert!(
            text.contains("Candidate generation is inactive in the current primitive product flow"),
            "active variation retirement docs should mark candidate generation inactive"
        );
    }
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

fn doc_text(relative_path: &str) -> String {
    fs::read_to_string(repo_root().join(relative_path))
        .unwrap_or_else(|error| panic!("failed to read {relative_path}: {error}"))
}

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .expect("shape-app manifest has workspace root ancestor")
        .to_path_buf()
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
