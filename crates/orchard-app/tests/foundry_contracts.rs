#![forbid(unsafe_code)]
#![allow(dead_code)]

#[path = "../src/foundry/mod.rs"]
mod foundry;

use foundry::{
    FoundryAppCommand, FoundryAppState, FoundryJobEvent, FoundryJobRequest, FoundryPackView,
};
use std::fs;
use std::path::{Path, PathBuf};

use orchard_foundry::{
    CatalogContentRef, FoundryAssetDocument, FoundryCommand, FoundryDocumentId,
    GenerateCandidatesRequest, VariationIntent,
};
use orchard_render::OrbitCamera;
use orchard_search_internal::foundry::{FoundryCandidateMode, FoundryCandidateRequest};

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
            text.contains("Family Studio Lite v0"),
            "README and status should describe the scoped Family Studio Lite v0 boundary"
        );
        assert!(
            text.contains("Draft / Personal Kits"),
            "README and status should limit Family Studio Lite v0 to Draft / Personal Kits"
        );
        assert!(
            text.contains("generated candidate trays"),
            "README and status should keep generated candidate trays blocked"
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

#[test]
fn phase_a_contract_boundary_docs_are_authoritative() {
    let readme = doc_text("README.md");
    let status = doc_text("docs/CURRENT_PRODUCT_STATUS.md");
    let limitations = doc_text("docs/KNOWN_LIMITATIONS.md");
    let boundaries = doc_text("docs/CONTRACT_BOUNDARIES.md");
    let orchard_core_legacy = doc_text("docs/ORCHARD_CORE_LEGACY_BOUNDARY.md");
    let architecture = doc_text("docs/ARCHITECTURE_STATUS.md");
    let authoritative_docs = [
        &status,
        &limitations,
        &boundaries,
        &orchard_core_legacy,
        &architecture,
    ];
    let joined = authoritative_docs
        .iter()
        .map(|text| text.as_str())
        .collect::<Vec<_>>()
        .join("\n")
        .to_ascii_lowercase();

    for text in [&status, &boundaries, &architecture] {
        assert!(
            text.contains("semantic asset compiler"),
            "Phase A docs should name the semantic asset compiler target"
        );
        assert!(
            text.contains("AssetRecipe") && text.contains("Orchard IR"),
            "Phase A docs should name AssetRecipe / Orchard IR as the canonical lane"
        );
    }

    for text in [
        &status,
        &limitations,
        &boundaries,
        &orchard_core_legacy,
        &architecture,
    ] {
        assert!(
            text.contains("orchard-core-legacy::ShapeDocument")
                || text.contains("ShapeDocument")
                || text.contains("orchard-core-legacy"),
            "Phase A docs should mention the orchard-core-legacy legacy boundary"
        );
        assert!(
            text.contains("not the new canonical product IR")
                || text.contains("must not receive new canonical product semantics")
                || text.contains("not the target product backbone"),
            "Phase A docs should block ShapeDocument as the new product IR"
        );
    }

    for forbidden in [
        "shapedocument is the new canonical product ir",
        "orchard-core-legacy::shapedocument is the canonical product ir",
        "orchard-core-legacy shapedocument is the canonical product ir",
        "terrain can be represented as only a generic mesh primitive",
        "terrain is only a generic mesh primitive",
        "game-ready is approved",
        "runtime llm is approved",
        "public catalog publishing is approved",
    ] {
        assert!(
            !joined.contains(forbidden),
            "authoritative docs must not imply blocked Phase A claim: {forbidden}"
        );
    }

    assert!(
        joined.contains("terrain remains blocked")
            || joined.contains("terrain must not be collapsed"),
        "Phase A docs should keep terrain contract-blocked"
    );
    assert!(
        joined.contains("runtime llm") && joined.contains("blocked"),
        "Phase A docs should keep runtime LLM blocked"
    );
    assert!(
        joined.contains("public catalog publishing") && joined.contains("blocked"),
        "Phase A docs should keep public catalog publishing blocked"
    );
    assert!(
        joined.contains("game-ready") && joined.contains("blocked"),
        "Phase A docs should keep game-ready claims blocked"
    );
    assert!(
        readme.contains("Surface/material work, UV/texturing, rigging, animation, runtime LLM"),
        "README should still carry the public unsupported-work boundary"
    );
}

#[test]
fn foundry_active_docs_index_links_existing_docs_only() {
    let root = repo_root();
    let index = doc_text("docs/README.md");
    let links = active_doc_links(&index);

    assert!(!links.is_empty(), "docs README should link active docs");

    for link in links {
        let path = root.join("docs").join(&link);
        assert!(
            path.is_file(),
            "docs/README.md references missing active doc: {link}"
        );
    }
}

#[test]
fn foundry_active_docs_do_not_reactivate_obsolete_pivots() {
    let mut active_docs = vec!["README.md".to_owned(), "docs/README.md".to_owned()];
    active_docs.extend(
        active_doc_links(&doc_text("docs/README.md"))
            .into_iter()
            .map(|path| format!("docs/{path}")),
    );
    active_docs.sort();
    active_docs.dedup();

    let joined = active_docs
        .iter()
        .map(|path| doc_text(path))
        .collect::<Vec<_>>()
        .join("\n")
        .to_ascii_lowercase();

    for forbidden in [
        "sci-fi crate is the flagship",
        "sci-fi crate remains the flagship",
        "flagship sci-fi crate",
        "cargo case is the active direction",
        "cargo case remains the active direction",
        "active direction is cargo case",
        "generated variation ui is active",
        "generated variations are active",
        "generated variation tray is active",
        "try ideas is active",
        "material support is approved",
        "material editor is supported",
        "uv support is approved",
        "uv editor is supported",
        "collision support is approved",
        "collision-enabled output is available",
        "rigging is supported",
        "animation is supported",
        "godot-ready output is available",
        "game-ready output is supported",
        "game-ready output is available without proof",
        "full game-ready support",
    ] {
        assert!(
            !joined.contains(forbidden),
            "active docs must not reactivate obsolete claim: {forbidden}"
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

fn active_doc_links(index: &str) -> Vec<String> {
    index
        .lines()
        .filter_map(|line| {
            let start = line.find("](")?;
            let after_start = &line[start + 2..];
            let end = after_start.find(')')?;
            let link = &after_start[..end];
            if link.ends_with(".md") {
                Some(link.to_owned())
            } else {
                None
            }
        })
        .collect()
}

fn doc_text(relative_path: &str) -> String {
    fs::read_to_string(repo_root().join(relative_path))
        .unwrap_or_else(|error| panic!("failed to read {relative_path}: {error}"))
}

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .expect("orchard-app manifest has workspace root ancestor")
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
