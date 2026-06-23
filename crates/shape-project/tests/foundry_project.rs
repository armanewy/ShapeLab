use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use serde_json::json;
use shape_asset::{AssetId, AssetRecipe, RevisionId};
use shape_foundry::{
    CatalogContentRef, ControlValue, EmbeddedCatalogSnapshot, FoundryAssetDocument,
    FoundryCatalogLock, FoundryCommand, FoundryConformanceSummary, FoundryDocumentId,
    GeneratedRecipeSnapshot, ProviderOverride, catalog_content_fingerprint_from_json,
    document_catalog_refs,
};
use shape_project::foundry::{
    FOUNDRY_PROJECT_FILE_SUFFIX, FoundryBuildStaleReason, FoundryProject, FoundryProjectError,
    FoundryProjectFile, FoundryProjectLoadContext, recovery_snapshot_path,
};

#[test]
fn save_load_preserves_replayable_branch_history() {
    let temp_dir = tempdir();
    let path = temp_dir.path().join("bridge.shapelab-foundry.json");
    let root_document = document_fixture();
    let root_recipe = recipe("Generated Bridge");
    let root_snapshot = GeneratedRecipeSnapshot::from_recipe(&root_recipe).unwrap();
    let mut project = FoundryProject::new(
        "Bridge Foundry",
        root_document.clone(),
        catalog_lock_for(&root_document),
        None,
        Some(root_snapshot),
        conformance(),
    )
    .unwrap();

    let first_document = document_with_span(&root_document, 4.0);
    let first = project
        .accept_commands(
            "Wider span",
            vec![set_span(4.0)],
            first_document.clone(),
            catalog_lock_for(&first_document),
            None,
            Some(GeneratedRecipeSnapshot::from_recipe(&root_recipe).unwrap()),
            conformance(),
        )
        .unwrap();
    project.undo().unwrap();
    let second_document = document_with_span(&root_document, 2.0);
    let second = project
        .accept_commands(
            "Narrower span",
            vec![set_span(2.0)],
            second_document.clone(),
            catalog_lock_for(&second_document),
            None,
            Some(GeneratedRecipeSnapshot::from_recipe(&root_recipe).unwrap()),
            conformance(),
        )
        .unwrap();

    assert_eq!(project.children_of(RevisionId(0)), vec![first, second]);
    project.switch_to(first).unwrap();
    assert_eq!(
        project.revision_path_to_root().unwrap(),
        vec![first, RevisionId(0)]
    );

    project.save_json(&path).unwrap();
    let loaded = FoundryProject::load_json(&path).unwrap();

    assert_eq!(loaded, project);
    assert_eq!(loaded.current_revision, first);
    assert_eq!(loaded.children_of(RevisionId(0)), vec![first, second]);
}

#[test]
fn role_presence_program_replays_as_persisted_toggle_state() {
    let temp_dir = tempdir();
    let path = temp_dir.path().join("role-presence.shapelab-foundry.json");
    let root_document = document_fixture();
    let mut project = FoundryProject::new(
        "Bridge Foundry",
        root_document.clone(),
        catalog_lock_for(&root_document),
        None,
        None,
        conformance(),
    )
    .unwrap();
    let mut child_document = root_document.clone();
    child_document
        .control_state
        .insert("side_rail".to_owned(), ControlValue::Toggle(false));
    child_document.catalog_lock = None;
    child_document.build_stamp = None;

    project
        .accept_commands(
            "Hide side rail",
            vec![FoundryCommand::SetRolePresence {
                role: "side_rail".to_owned(),
                enabled: false,
            }],
            child_document,
            catalog_lock_for(&root_document),
            None,
            None,
            conformance(),
        )
        .unwrap();

    project.save_json(&path).unwrap();
    let loaded = FoundryProject::load_json(&path).unwrap();
    assert_eq!(
        loaded
            .current_document()
            .unwrap()
            .control_state
            .get("side_rail"),
        Some(&ControlValue::Toggle(false))
    );
}

#[test]
fn load_rejects_revision_when_replay_does_not_match_child_snapshot() {
    let temp_dir = tempdir();
    let path = temp_dir.path().join("bad-replay.shapelab-foundry.json");
    let root_document = document_fixture();
    let mut project = FoundryProject::new(
        "Bridge Foundry",
        root_document.clone(),
        catalog_lock_for(&root_document),
        None,
        None,
        conformance(),
    )
    .unwrap();
    let child_document = document_with_span(&root_document, 4.0);
    let child = project
        .accept_commands(
            "Wider span",
            vec![set_span(4.0)],
            child_document.clone(),
            catalog_lock_for(&child_document),
            None,
            None,
            conformance(),
        )
        .unwrap();

    project
        .revisions
        .get_mut(&child)
        .unwrap()
        .document
        .control_state
        .insert("span_length".to_owned(), ControlValue::Scalar(9.0));
    fs::write(&path, serde_json::to_vec_pretty(&project).unwrap()).unwrap();

    assert!(matches!(
        FoundryProject::load_json(&path),
        Err(FoundryProjectError::ReplayMismatch(revision)) if revision == child
    ));
}

#[test]
fn catalog_mismatch_with_embedded_snapshot_loads_read_only_and_marks_stale() {
    let temp_dir = tempdir();
    let path = temp_dir.path().join("recovery.shapelab-foundry.json");
    let mut root_document = document_fixture();
    let embedded_json = r#"{"style":"embedded"}"#;
    root_document.style_content_ref = catalog_ref_for_json("roman-style", embedded_json);
    let mut lock = catalog_lock_for(&root_document);
    lock.embedded_snapshots.push(EmbeddedCatalogSnapshot {
        content_ref: root_document.style_content_ref.clone(),
        canonical_json: embedded_json.to_owned(),
    });
    let project = FoundryProject::new(
        "Bridge Foundry",
        root_document.clone(),
        lock,
        None,
        None,
        conformance(),
    )
    .unwrap();
    project.save_json(&path).unwrap();

    let mut context = FoundryProjectLoadContext::default();
    context
        .catalog_refs
        .insert("style".to_owned(), content_ref("roman-style", 99));
    let outcome = FoundryProject::load_json_with_context(&path, &context).unwrap();

    assert!(outcome.report.read_only_recovery);
    assert!(outcome.report.recovery_revisions.contains(&RevisionId(0)));
    assert_eq!(
        outcome.report.stale_builds[&RevisionId(0)],
        vec![FoundryBuildStaleReason::CatalogReferenceChanged {
            key: "style".to_owned()
        }]
    );
}

#[test]
fn corrupted_embedded_catalog_snapshot_is_rejected() {
    let mut root_document = document_fixture();
    let embedded_json = r#"{"style":"embedded"}"#;
    root_document.style_content_ref = catalog_ref_for_json("roman-style", embedded_json);
    let mut lock = catalog_lock_for(&root_document);
    lock.embedded_snapshots.push(EmbeddedCatalogSnapshot {
        content_ref: root_document.style_content_ref.clone(),
        canonical_json: r#"{"style":"corrupted"}"#.to_owned(),
    });

    assert!(matches!(
        FoundryProject::new(
            "Bridge Foundry",
            root_document,
            lock,
            None,
            None,
            conformance(),
        ),
        Err(FoundryProjectError::InvalidProject(message))
            if message.contains("embedded catalog snapshot fingerprint mismatch")
    ));
}

#[test]
fn provider_override_refs_are_part_of_the_exact_catalog_lock() {
    let mut root_document = document_fixture();
    root_document.provider_overrides.insert(
        "support".to_owned(),
        ProviderOverride {
            role: "support".to_owned(),
            provider_ref: content_ref("timber-support", 6),
        },
    );
    let mut missing_provider_lock = catalog_lock_for(&root_document);
    missing_provider_lock.exact_refs.remove("provider.support");

    assert!(matches!(
        FoundryProject::new(
            "Bridge Foundry",
            root_document.clone(),
            missing_provider_lock,
            None,
            None,
            conformance(),
        ),
        Err(FoundryProjectError::InvalidDocument { report, .. })
            if report
                .issues
                .iter()
                .any(|issue| issue.subject == "catalog_lock.exact_refs.provider.support"
                    && issue.code == "missing_catalog_lock_ref")
    ));

    let mut extra_lock = catalog_lock_for(&root_document);
    extra_lock
        .exact_refs
        .insert("provider.extra".to_owned(), content_ref("extra", 7));
    assert!(matches!(
        FoundryProject::new(
            "Bridge Foundry",
            root_document,
            extra_lock,
            None,
            None,
            conformance(),
        ),
        Err(FoundryProjectError::InvalidDocument { report, .. })
            if report
                .issues
                .iter()
                .any(|issue| issue.subject == "catalog_lock.exact_refs.provider.extra"
                    && issue.code == "extra_catalog_lock_ref")
    ));
}

#[test]
fn catalog_mismatch_without_embedded_snapshot_is_rejected() {
    let temp_dir = tempdir();
    let path = temp_dir
        .path()
        .join("catalog-mismatch.shapelab-foundry.json");
    let root_document = document_fixture();
    let project = FoundryProject::new(
        "Bridge Foundry",
        root_document.clone(),
        catalog_lock_for(&root_document),
        None,
        None,
        conformance(),
    )
    .unwrap();
    project.save_json(&path).unwrap();

    let mut context = FoundryProjectLoadContext::default();
    context
        .catalog_refs
        .insert("style".to_owned(), content_ref("roman-style", 99));

    assert!(matches!(
        FoundryProject::load_json_with_context(&path, &context),
        Err(FoundryProjectError::CatalogFingerprintMismatch { revision, key })
            if revision == RevisionId(0) && key == "style"
    ));
}

#[test]
fn available_recipe_inputs_must_match_exact_stored_snapshot() {
    let temp_dir = tempdir();
    let path = temp_dir.path().join("recipe.shapelab-foundry.json");
    let root_document = document_fixture();
    let recipe = recipe("Generated Bridge");
    let project = FoundryProject::new(
        "Bridge Foundry",
        root_document.clone(),
        catalog_lock_for(&root_document),
        None,
        Some(GeneratedRecipeSnapshot::from_recipe(&recipe).unwrap()),
        conformance(),
    )
    .unwrap();
    project.save_json(&path).unwrap();

    let mut context = FoundryProjectLoadContext::default();
    context
        .available_recipes
        .insert(RevisionId(0), recipe.clone());
    let outcome = FoundryProject::load_json_with_context(&path, &context).unwrap();
    assert_eq!(
        outcome.report.verified_recipe_revisions,
        vec![RevisionId(0)]
    );

    let mut changed_recipe = recipe;
    changed_recipe.title = "Different Generated Bridge".to_owned();
    context
        .available_recipes
        .insert(RevisionId(0), changed_recipe);
    assert!(matches!(
        FoundryProject::load_json_with_context(&path, &context),
        Err(FoundryProjectError::RecipeSnapshotMismatch(RevisionId(0)))
    ));
}

#[test]
fn project_file_enforces_suffix_and_writes_recovery_snapshot() {
    let temp_dir = tempdir();
    let root_document = document_fixture();
    let mut file = FoundryProjectFile::new(
        "Bridge Foundry",
        root_document.clone(),
        catalog_lock_for(&root_document),
        None,
        None,
        conformance(),
    )
    .unwrap();

    assert!(matches!(
        file.save_as(temp_dir.path().join("bridge.json")),
        Err(FoundryProjectError::InvalidProjectPath { .. })
    ));

    let project_path = temp_dir
        .path()
        .join(format!("bridge{FOUNDRY_PROJECT_FILE_SUFFIX}"));
    file.save_as(&project_path).unwrap();
    assert!(!file.is_dirty());

    let recovery_path = recovery_snapshot_path(&project_path);
    let snapshot = file.save_recovery_snapshot(&recovery_path).unwrap();

    assert_eq!(snapshot.path, recovery_path);
    assert_eq!(
        FoundryProject::load_json(&snapshot.path).unwrap(),
        file.project
    );
}

fn document_fixture() -> FoundryAssetDocument {
    let mut document = FoundryAssetDocument::new(
        FoundryDocumentId("doc-bridge".to_owned()),
        content_ref("bridge-family", 1),
        content_ref("roman-style", 2),
        content_ref("bridge-family-impl", 3),
        content_ref("roman-style-impl", 4),
        content_ref("bridge-profile", 5),
    );
    document
        .control_state
        .insert("span_length".to_owned(), ControlValue::Scalar(3.0));
    document.seed = 11;
    document
}

fn document_with_span(document: &FoundryAssetDocument, span: f32) -> FoundryAssetDocument {
    let mut document = document.clone();
    document
        .control_state
        .insert("span_length".to_owned(), ControlValue::Scalar(span));
    document.catalog_lock = None;
    document.build_stamp = None;
    document
}

fn set_span(span: f32) -> FoundryCommand {
    FoundryCommand::SetControl {
        control_id: "span_length".to_owned(),
        value: ControlValue::Scalar(span),
    }
}

fn catalog_lock_for(document: &FoundryAssetDocument) -> FoundryCatalogLock {
    FoundryCatalogLock {
        exact_refs: document_catalog_refs(document),
        embedded_snapshots: Vec::new(),
        compiler_version: "compiler-a".to_owned(),
        catalog_version: 1,
    }
}

fn catalog_ref_for_json(stable_id: &str, canonical_json: &str) -> CatalogContentRef {
    CatalogContentRef {
        stable_id: stable_id.to_owned(),
        schema_version: 1,
        fingerprint: catalog_content_fingerprint_from_json(stable_id, canonical_json).unwrap(),
    }
}

fn content_ref(stable_id: &str, byte: u8) -> CatalogContentRef {
    serde_json::from_value(json!({
        "stable_id": stable_id,
        "schema_version": 1,
        "fingerprint": hex_fingerprint(byte),
    }))
    .unwrap()
}

fn hex_fingerprint(byte: u8) -> String {
    format!("{byte:02x}").repeat(32)
}

fn recipe(title: &str) -> AssetRecipe {
    AssetRecipe::new(AssetId(77), title)
}

fn conformance() -> FoundryConformanceSummary {
    FoundryConformanceSummary {
        accepted: true,
        required_failure_count: 0,
        advisory_issue_count: 0,
        runtime_deferred_count: 0,
    }
}

struct TestTempDir {
    path: PathBuf,
}

impl TestTempDir {
    fn path(&self) -> &Path {
        &self.path
    }
}

impl Drop for TestTempDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}

fn tempdir() -> TestTempDir {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or(0);
    let path = std::env::temp_dir().join(format!("shape-lab-foundry-project-tests-{nonce}"));
    fs::create_dir(&path).unwrap();
    TestTempDir { path }
}
