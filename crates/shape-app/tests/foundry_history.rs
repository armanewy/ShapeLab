#![forbid(unsafe_code)]
#![allow(dead_code)]

#[path = "../src/foundry/mod.rs"]
mod foundry;

use std::path::{Path, PathBuf};

use foundry::{FoundryAppCommand, FoundryAppState, panels::history};
use shape_asset::RevisionId;
use shape_foundry::{
    CatalogContentRef, ControlValue, FoundryAssetDocument, FoundryCandidateId, FoundryCatalogLock,
    FoundryCommand, FoundryConformanceSummary, FoundryDocumentId, LocalRecipeOverride,
    ProviderOverride,
};
use shape_project::foundry::{
    FoundryBuildStaleReason, FoundryProject, FoundryProjectFile, FoundryProjectLoadReport,
};

#[test]
fn semantic_revision_tree_rows_keep_branch_structure() {
    let project = branched_project();

    let rows = history::build_history_rows(&project);

    assert_eq!(
        rows.iter()
            .map(|row| (row.revision, row.depth))
            .collect::<Vec<_>>(),
        vec![(RevisionId(0), 0), (RevisionId(1), 1), (RevisionId(2), 1)]
    );
    assert_eq!(rows[0].branch_label, "2 branches");
    assert!(
        rows[0]
            .badges
            .iter()
            .any(|badge| badge.kind == history::FoundryHistoryBadgeKind::Branch)
    );
    assert_eq!(rows[1].summary.label, "Height change");
    assert_eq!(rows[1].summary.detail.as_deref(), Some("Changed Height"));
    assert_eq!(rows[2].summary.label, "Changed Body");
    assert_eq!(rows[2].summary.detail.as_deref(), Some("Option updated"));
    assert!(rows[2].selected);
    assert!(rows[2].on_current_path);
    assert!(!rows[1].on_current_path);
    assert_eq!(history::branch_points(&project), vec![RevisionId(0)]);
}

#[test]
fn summaries_identify_controls_providers_and_accepted_candidates() {
    let control = history::command_summary(&FoundryCommand::SetControl {
        control_id: "height".to_owned(),
        value: ControlValue::Scalar(0.75),
    });
    assert_eq!(
        control.kind,
        history::FoundryHistorySummaryKind::ControlEdit
    );
    assert_eq!(control.label, "Changed Height");
    assert_eq!(control.detail.as_deref(), Some("Value set to 0.75"));
    assert_eq!(control.changed_controls, vec!["height"]);

    let provider_ref = content_ref("body.round", 9);
    let provider = history::command_summary(&FoundryCommand::SelectProvider {
        role: "body".to_owned(),
        provider_ref,
    });
    assert_eq!(
        provider.kind,
        history::FoundryHistorySummaryKind::ProviderChange
    );
    assert_eq!(provider.label, "Changed Body");
    assert_eq!(provider.detail.as_deref(), Some("Option updated"));
    assert_eq!(provider.changed_provider_roles, vec!["body"]);

    let candidate_id = FoundryCandidateId("candidate-7".to_owned());
    let accepted = history::command_summary(&FoundryCommand::AcceptCandidate {
        candidate_id: candidate_id.clone(),
    });
    assert_eq!(
        accepted.kind,
        history::FoundryHistorySummaryKind::CandidateAcceptance
    );
    assert_eq!(accepted.label, "Chose a direction");
    assert_eq!(accepted.accepted_candidate, Some(candidate_id));
}

#[test]
fn local_overrides_and_stale_catalogs_surface_as_badges() {
    let mut document = minimal_foundry_document("override-doc");
    document
        .local_recipe_overrides
        .push(local_override("body-nudge", "Pinned"));
    document
        .local_recipe_overrides
        .push(local_override("edge-replay", "Revalidate"));
    let project = FoundryProject::new(
        "Override asset",
        document.clone(),
        FoundryCatalogLock::from_document_refs(&document),
        None,
        None,
        accepted_conformance(),
    )
    .expect("project should be valid");

    let mut load_report = FoundryProjectLoadReport::default();
    load_report.stale_builds.insert(
        RevisionId(0),
        vec![
            FoundryBuildStaleReason::CatalogVersionChanged {
                stored: 1,
                current: 2,
            },
            FoundryBuildStaleReason::CatalogReferenceChanged {
                key: "style".to_owned(),
            },
        ],
    );
    load_report.recovery_revisions.insert(RevisionId(0));
    load_report.read_only_recovery = true;
    load_report.verified_recipe_revisions.push(RevisionId(0));

    let rows = history::build_history_rows_with_load_report(&project, Some(&load_report));
    let badges = &rows[0].badges;

    assert!(badges.iter().any(|badge| {
        badge.kind == history::FoundryHistoryBadgeKind::LocalOverrides
            && badge.label == "2 local overrides"
            && badge.detail.as_deref() == Some("1 pinned, 1 revalidate")
    }));
    assert!(
        badges
            .iter()
            .any(|badge| badge.kind == history::FoundryHistoryBadgeKind::StaleCatalog)
    );
    assert!(
        badges
            .iter()
            .any(|badge| badge.kind == history::FoundryHistoryBadgeKind::Recovery)
    );
    assert!(
        badges
            .iter()
            .any(|badge| badge.kind == history::FoundryHistoryBadgeKind::VerifiedRecipe)
    );
    assert_eq!(
        history::stale_catalog_warning(RevisionId(0), &load_report).as_deref(),
        Some(
            "Stored build may be stale: catalog version 1 -> 2, catalog reference changed for style"
        )
    );
}

#[test]
fn action_helpers_emit_foundry_app_commands() {
    assert_eq!(
        history::undo_command().single_foundry_command(),
        Some(&FoundryCommand::Undo)
    );
    assert_eq!(
        history::switch_revision_command(RevisionId(3)).single_foundry_command(),
        Some(&FoundryCommand::SwitchRevision {
            revision_id: RevisionId(3)
        })
    );
    assert_eq!(history::undo_intent(false).dispatch, None);
    assert_eq!(history::switch_revision_intent(RevisionId(3), true), None);
    assert_eq!(history::save_command(), FoundryAppCommand::Save);
    assert_eq!(
        history::load_command("asset.shapelab-foundry.json"),
        FoundryAppCommand::Load("asset.shapelab-foundry.json".into())
    );
    assert_eq!(
        history::save_as_command("asset-copy.shapelab-foundry.json"),
        FoundryAppCommand::SaveAs("asset-copy.shapelab-foundry.json".into())
    );
    assert_eq!(
        history::load_intent("asset.shapelab-foundry.json").dispatch,
        Some(history::FoundryHistoryActionDispatch::Command(
            FoundryAppCommand::Load("asset.shapelab-foundry.json".into())
        ))
    );
}

#[test]
fn history_view_is_reachable_from_native_foundry_boundary() {
    let path = PathBuf::from("history-asset.shapelab-foundry.json");
    let state = state_with_clean_project(path, false);

    let view = foundry::build_foundry_history_view(&state);

    assert_eq!(view.rows.len(), 3);
    assert_eq!(view.rows[0].branch_label, "2 branches");
    assert!(view.actions.iter().any(|action| {
        action.kind == history::FoundryHistoryActionKind::Load
            && action.dispatch.as_ref()
                == Some(&history::FoundryHistoryActionDispatch::RequestLoadPath)
    }));
}

#[test]
fn history_view_actions_include_load_and_enabled_dispatches() {
    let path = PathBuf::from("history-asset.shapelab-foundry.json");
    let state = state_with_clean_project(path, true);

    let view = history::build_history_view(&state);

    assert_eq!(
        view.actions
            .iter()
            .map(|action| action.kind)
            .collect::<Vec<_>>(),
        vec![
            history::FoundryHistoryActionKind::Undo,
            history::FoundryHistoryActionKind::Save,
            history::FoundryHistoryActionKind::SaveAs,
            history::FoundryHistoryActionKind::Load,
        ]
    );
    assert!(view.actions.iter().all(|action| {
        if action.enabled {
            action.dispatch.is_some()
        } else {
            action.dispatch.is_none()
        }
    }));
    assert_eq!(
        action(&view, history::FoundryHistoryActionKind::Save)
            .dispatch
            .as_ref(),
        Some(&history::FoundryHistoryActionDispatch::Command(
            FoundryAppCommand::Save
        ))
    );
    assert_eq!(
        action(&view, history::FoundryHistoryActionKind::SaveAs)
            .dispatch
            .as_ref(),
        Some(&history::FoundryHistoryActionDispatch::RequestSaveAsPath)
    );
    assert_eq!(
        action(&view, history::FoundryHistoryActionKind::Load)
            .dispatch
            .as_ref(),
        Some(&history::FoundryHistoryActionDispatch::RequestLoadPath)
    );
}

fn state_with_clean_project(path: PathBuf, dirty: bool) -> FoundryAppState {
    let mut state = FoundryAppState::default();
    state.project_file = Some(FoundryProjectFile::clean(
        branched_project(),
        Some(path.clone()),
    ));
    state.project_path = Some(path);
    state.dirty = dirty;
    state
}

#[test]
fn save_load_status_formats_dirty_unsaved_and_recovery_states() {
    let clean = history::save_load_status(
        Some(Path::new("asset.shapelab-foundry.json")),
        true,
        false,
        false,
    );
    assert_eq!(clean.state, history::FoundrySaveLoadState::CleanSaved);
    assert_eq!(
        clean.path_label.as_deref(),
        Some("asset.shapelab-foundry.json")
    );
    assert!(!clean.can_save);

    let dirty = history::save_load_status(
        Some(Path::new("asset.shapelab-foundry.json")),
        true,
        true,
        false,
    );
    assert_eq!(dirty.state, history::FoundrySaveLoadState::DirtySaved);
    assert!(dirty.can_save);

    let unsaved = history::save_load_status(None, true, true, false);
    assert_eq!(unsaved.state, history::FoundrySaveLoadState::Unsaved);
    assert!(!unsaved.can_save);
    assert!(unsaved.can_save_as);

    let clean_without_path = history::save_load_status(None, true, false, false);
    assert_eq!(
        clean_without_path.state,
        history::FoundrySaveLoadState::Unsaved
    );
    assert_eq!(clean_without_path.label, "Unsaved project");
    assert!(!clean_without_path.can_save);
    assert!(clean_without_path.can_save_as);

    let recovery = history::save_load_status(
        Some(Path::new("asset.shapelab-foundry.json")),
        true,
        true,
        true,
    );
    assert_eq!(
        recovery.state,
        history::FoundrySaveLoadState::ReadOnlyRecovery
    );
    assert!(!recovery.can_save);
    assert!(!recovery.can_save_as);

    let empty = history::save_load_status(None, false, false, false);
    assert_eq!(empty.state, history::FoundrySaveLoadState::NoProject);
    assert!(!empty.can_save);
    assert!(empty.can_load);
}

fn branched_project() -> FoundryProject {
    let root_document = minimal_foundry_document("root-doc");
    let mut project = FoundryProject::new(
        "History asset",
        root_document.clone(),
        FoundryCatalogLock::from_document_refs(&root_document),
        None,
        None,
        accepted_conformance(),
    )
    .expect("project should be valid");

    let mut control_document = root_document.clone();
    control_document
        .control_state
        .insert("height".to_owned(), ControlValue::Scalar(0.75));
    project
        .accept_commands(
            "Height change",
            vec![FoundryCommand::SetControl {
                control_id: "height".to_owned(),
                value: ControlValue::Scalar(0.75),
            }],
            control_document.clone(),
            FoundryCatalogLock::from_document_refs(&control_document),
            None,
            None,
            accepted_conformance(),
        )
        .expect("control revision should be accepted");

    project.undo().expect("root should be undo target");

    let provider_ref = content_ref("body.round", 9);
    let mut provider_document = root_document;
    provider_document.provider_overrides.insert(
        "body".to_owned(),
        ProviderOverride {
            role: "body".to_owned(),
            provider_ref: provider_ref.clone(),
        },
    );
    project
        .accept_commands(
            "Provider change",
            vec![FoundryCommand::SelectProvider {
                role: "body".to_owned(),
                provider_ref,
            }],
            provider_document.clone(),
            FoundryCatalogLock::from_document_refs(&provider_document),
            None,
            None,
            accepted_conformance(),
        )
        .expect("provider revision should be accepted");

    project
}

fn action(
    view: &history::FoundryHistoryView,
    kind: history::FoundryHistoryActionKind,
) -> &history::FoundryHistoryActionIntent {
    view.actions
        .iter()
        .find(|action| action.kind == kind)
        .expect("history action should exist")
}

fn minimal_foundry_document(document_id: &str) -> FoundryAssetDocument {
    FoundryAssetDocument::new(
        FoundryDocumentId(document_id.to_owned()),
        content_ref("family", 1),
        content_ref("style", 2),
        content_ref("family-impl", 3),
        content_ref("style-impl", 4),
        content_ref("profile", 5),
    )
}

fn accepted_conformance() -> FoundryConformanceSummary {
    FoundryConformanceSummary {
        accepted: true,
        ..FoundryConformanceSummary::default()
    }
}

fn local_override(id: &str, policy: &str) -> LocalRecipeOverride {
    serde_json::from_value(serde_json::json!({
        "id": id,
        "base_geometry_fingerprint": fingerprint_hex(12),
        "edit_program": {
            "label": "override",
            "seed": 0,
            "operations": [
                { "SetScalar": { "parameter": 1, "value": 0.5 } }
            ]
        },
        "touched_targets": [
            { "FamilySlot": "height" }
        ],
        "survival_policy": policy
    }))
    .expect("local override should deserialize")
}

fn content_ref(stable_id: &str, byte: u8) -> CatalogContentRef {
    serde_json::from_value(serde_json::json!({
        "stable_id": stable_id,
        "schema_version": 1,
        "fingerprint": fingerprint_hex(byte),
    }))
    .expect("catalog reference should deserialize")
}

fn fingerprint_hex(byte: u8) -> String {
    format!("{byte:02x}").repeat(32)
}
