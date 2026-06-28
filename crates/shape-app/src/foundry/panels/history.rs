//! Foundry semantic history panel view data.

#![allow(dead_code)]

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use shape_asset::RevisionId;
use shape_foundry::{
    CatalogContentRef, ControlValue, FoundryAssetDocument, FoundryCandidateId, FoundryCommand,
    FoundryLockMode, FoundryLockTarget, FoundryProjectRevision, FoundryProjectRevisionProgram,
    LocalRecipeOverride, OverrideSurvivalPolicy,
};
use shape_project::foundry::{
    FoundryBuildStaleReason, FoundryProject, FoundryProjectFile, FoundryProjectLoadReport,
};

use crate::foundry::{FoundryAppCommand, FoundryAppState};

const MAX_SUMMARY_PARTS: usize = 3;

/// UI-ready history panel snapshot.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct FoundryHistoryView {
    /// Flattened semantic revision tree.
    pub rows: Vec<FoundryHistoryRow>,
    /// Save/load and recovery state.
    pub status: FoundrySaveLoadStatus,
    /// Top-level actions exposed by the panel.
    pub actions: Vec<FoundryHistoryActionIntent>,
}

/// Stable flattened revision row.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct FoundryHistoryRow {
    /// Revision ID.
    pub revision: RevisionId,
    /// Parent revision, if any.
    pub parent: Option<RevisionId>,
    /// Tree indentation depth.
    pub depth: usize,
    /// Human-facing row label.
    pub label: String,
    /// Concise operation summary.
    pub summary: FoundryHistorySummary,
    /// Direct child count.
    pub child_count: usize,
    /// Branch label for the row.
    pub branch_label: String,
    /// True for the selected/current revision.
    pub selected: bool,
    /// True when the row is on the current path back to root.
    pub on_current_path: bool,
    /// Revision badges.
    pub badges: Vec<FoundryHistoryBadge>,
    /// Switch intent for this revision.
    pub switch_intent: Option<FoundryHistoryActionIntent>,
    /// Branch-from intent for this revision.
    pub branch_intent: Option<FoundryHistoryActionIntent>,
}

/// Small row/status badge.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct FoundryHistoryBadge {
    /// Badge kind for styling.
    pub kind: FoundryHistoryBadgeKind,
    /// Short badge text.
    pub label: String,
    /// Optional tooltip/detail text.
    pub detail: Option<String>,
}

/// Badge kind for history rows and status.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub(crate) enum FoundryHistoryBadgeKind {
    Current,
    Branch,
    LocalOverrides,
    StaleCatalog,
    Recovery,
    VerifiedRecipe,
    Dirty,
    Saved,
    ReadOnly,
    Unsaved,
}

/// Structured operation summary for a revision or command.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct FoundryHistorySummary {
    /// Coarse summary kind.
    pub kind: FoundryHistorySummaryKind,
    /// Short summary text.
    pub label: String,
    /// Optional expanded detail.
    pub detail: Option<String>,
    /// Control IDs touched by this summary.
    pub changed_controls: Vec<String>,
    /// Provider roles touched by this summary.
    pub changed_provider_roles: Vec<String>,
    /// Candidate accepted by this summary.
    pub accepted_candidate: Option<FoundryCandidateId>,
}

/// Coarse summary kind.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub(crate) enum FoundryHistorySummaryKind {
    Start,
    ControlEdit,
    ProviderChange,
    CandidateAcceptance,
    StyleChange,
    LockChange,
    CandidateGeneration,
    RuntimeAction,
    CommandProgram,
}

/// Action intent emitted by the history panel.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct FoundryHistoryActionIntent {
    /// Action kind for styling and dispatch.
    pub kind: FoundryHistoryActionKind,
    /// Button/menu label.
    pub label: String,
    /// Whether the action is currently enabled.
    pub enabled: bool,
    /// Dispatch payload to emit when enabled.
    pub dispatch: Option<FoundryHistoryActionDispatch>,
}

/// Dispatch payload emitted by a history action.
#[derive(Debug, Clone, PartialEq)]
#[allow(clippy::large_enum_variant)]
pub(crate) enum FoundryHistoryActionDispatch {
    /// Emit a concrete app command.
    Command(FoundryAppCommand),
    /// Ask the native shell for a Save As destination before saving.
    RequestSaveAsPath,
    /// Ask the native shell for a project path before loading.
    RequestLoadPath,
}

/// High-level history action kind.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub(crate) enum FoundryHistoryActionKind {
    Undo,
    SwitchRevision,
    BranchFromRevision,
    Save,
    SaveAs,
    Load,
}

/// Save/load state formatted for the history panel.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct FoundrySaveLoadStatus {
    /// Coarse save/load state.
    pub state: FoundrySaveLoadState,
    /// Short status label.
    pub label: String,
    /// Optional detail text.
    pub detail: Option<String>,
    /// Project path label, if known.
    pub path_label: Option<String>,
    /// Whether Save should be enabled.
    pub can_save: bool,
    /// Whether Save As should be enabled.
    pub can_save_as: bool,
    /// Whether Load can be offered.
    pub can_load: bool,
    /// Status badges.
    pub badges: Vec<FoundryHistoryBadge>,
}

/// Coarse save/load state.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub(crate) enum FoundrySaveLoadState {
    NoProject,
    CleanSaved,
    DirtySaved,
    Unsaved,
    ReadOnlyRecovery,
}

/// Build the full history panel view data from app state.
#[must_use]
pub(crate) fn build_history_view(state: &FoundryAppState) -> FoundryHistoryView {
    let project_file = state.project_file.as_ref();
    let load_report = state
        .load_report
        .as_ref()
        .or_else(|| project_file.map(|file| &file.load_report));
    let rows = project_file
        .map(|file| build_history_rows_with_load_report(&file.project, load_report))
        .unwrap_or_default();
    let status = save_load_status_for_state(state);
    let mut actions = Vec::new();
    let can_undo = project_file.is_some_and(|file| file.project.can_undo());
    actions.push(undo_intent(can_undo));
    actions.push(save_intent(status.can_save));
    actions.push(save_as_intent(status.can_save_as));
    actions.push(load_request_intent(status.can_load));
    FoundryHistoryView {
        rows,
        status,
        actions,
    }
}

/// Build rows for a branchable semantic revision tree.
#[must_use]
pub(crate) fn build_history_rows(project: &FoundryProject) -> Vec<FoundryHistoryRow> {
    build_history_rows_with_load_report(project, None)
}

/// Build rows for a branchable semantic revision tree with load diagnostics.
#[must_use]
pub(crate) fn build_history_rows_with_load_report(
    project: &FoundryProject,
    load_report: Option<&FoundryProjectLoadReport>,
) -> Vec<FoundryHistoryRow> {
    let mut children = BTreeMap::<Option<RevisionId>, Vec<&FoundryProjectRevision>>::new();
    for revision in project.revisions.values() {
        children.entry(revision.parent).or_default().push(revision);
    }
    for items in children.values_mut() {
        items.sort_by_key(|revision| revision.id);
    }

    let current_path = current_path_set(project);
    let mut rows = Vec::new();
    let mut visited = BTreeSet::new();
    let context = RevisionRowBuildContext {
        project,
        load_report,
        current_path: &current_path,
        children: &children,
    };
    append_revision_rows(&context, None, 0, &mut visited, &mut rows);
    for revision in project.revisions.values() {
        if !visited.contains(&revision.id) {
            append_revision_row(&context, revision, 0, &mut visited, &mut rows);
        }
    }
    rows
}

/// Return revisions that have more than one direct child.
#[must_use]
pub(crate) fn branch_points(project: &FoundryProject) -> Vec<RevisionId> {
    project
        .revisions
        .keys()
        .copied()
        .filter(|revision| project.children_of(*revision).len() > 1)
        .collect()
}

/// Label how many direct child branches leave a revision.
#[must_use]
pub(crate) fn branch_label(project: &FoundryProject, revision: RevisionId) -> String {
    branch_count_label(project.children_of(revision).len())
}

/// Label a direct child count.
#[must_use]
pub(crate) fn branch_count_label(child_count: usize) -> String {
    match child_count {
        0 => String::new(),
        1 => "1 child".to_owned(),
        count => format!("{count} branches"),
    }
}

/// Build an undo command.
#[must_use]
pub(crate) fn undo_command() -> FoundryAppCommand {
    FoundryAppCommand::run(FoundryCommand::Undo)
}

/// Build an undo intent with enabled state.
#[must_use]
pub(crate) fn undo_intent(can_undo: bool) -> FoundryHistoryActionIntent {
    FoundryHistoryActionIntent {
        kind: FoundryHistoryActionKind::Undo,
        label: "Undo".to_owned(),
        enabled: can_undo,
        dispatch: can_undo.then(|| FoundryHistoryActionDispatch::Command(undo_command())),
    }
}

/// Build a revision switch command.
#[must_use]
pub(crate) fn switch_revision_command(revision: RevisionId) -> FoundryAppCommand {
    FoundryAppCommand::run(FoundryCommand::SwitchRevision {
        revision_id: revision,
    })
}

/// Build a switch intent only when the target is not already selected.
#[must_use]
pub(crate) fn switch_revision_intent(
    revision: RevisionId,
    selected: bool,
) -> Option<FoundryHistoryActionIntent> {
    (!selected).then(|| FoundryHistoryActionIntent {
        kind: FoundryHistoryActionKind::SwitchRevision,
        label: format!("Switch to revision {}", revision.0),
        enabled: true,
        dispatch: Some(FoundryHistoryActionDispatch::Command(
            switch_revision_command(revision),
        )),
    })
}

/// Build an intent for selecting a previous revision before accepting a new branch.
#[must_use]
pub(crate) fn branch_from_revision_intent(
    revision: RevisionId,
    selected: bool,
) -> Option<FoundryHistoryActionIntent> {
    (!selected).then(|| FoundryHistoryActionIntent {
        kind: FoundryHistoryActionKind::BranchFromRevision,
        label: format!("Branch from revision {}", revision.0),
        enabled: true,
        dispatch: Some(FoundryHistoryActionDispatch::Command(
            switch_revision_command(revision),
        )),
    })
}

/// Build a save command.
#[must_use]
pub(crate) fn save_command() -> FoundryAppCommand {
    FoundryAppCommand::Save
}

/// Build a save intent with enabled state.
#[must_use]
pub(crate) fn save_intent(can_save: bool) -> FoundryHistoryActionIntent {
    FoundryHistoryActionIntent {
        kind: FoundryHistoryActionKind::Save,
        label: "Save".to_owned(),
        enabled: can_save,
        dispatch: can_save.then(|| FoundryHistoryActionDispatch::Command(save_command())),
    }
}

/// Build a save-as command.
#[must_use]
pub(crate) fn save_as_command(path: impl Into<PathBuf>) -> FoundryAppCommand {
    FoundryAppCommand::SaveAs(path.into())
}

/// Build a save-as intent. The concrete path is supplied later by the shell UI.
#[must_use]
pub(crate) fn save_as_intent(can_save_as: bool) -> FoundryHistoryActionIntent {
    FoundryHistoryActionIntent {
        kind: FoundryHistoryActionKind::SaveAs,
        label: "Save As".to_owned(),
        enabled: can_save_as,
        dispatch: can_save_as.then_some(FoundryHistoryActionDispatch::RequestSaveAsPath),
    }
}

/// Build a load command.
#[must_use]
pub(crate) fn load_command(path: impl Into<PathBuf>) -> FoundryAppCommand {
    FoundryAppCommand::Load(path.into())
}

/// Build a load intent for a known path.
#[must_use]
pub(crate) fn load_intent(path: impl Into<PathBuf>) -> FoundryHistoryActionIntent {
    let path = path.into();
    FoundryHistoryActionIntent {
        kind: FoundryHistoryActionKind::Load,
        label: format!("Load {}", path_label(Some(path.as_path()))),
        enabled: true,
        dispatch: Some(FoundryHistoryActionDispatch::Command(load_command(path))),
    }
}

/// Build a load request intent. The concrete path is supplied later by the shell UI.
#[must_use]
pub(crate) fn load_request_intent(can_load: bool) -> FoundryHistoryActionIntent {
    FoundryHistoryActionIntent {
        kind: FoundryHistoryActionKind::Load,
        label: "Load".to_owned(),
        enabled: can_load,
        dispatch: can_load.then_some(FoundryHistoryActionDispatch::RequestLoadPath),
    }
}

/// Summarize one project revision.
#[must_use]
pub(crate) fn revision_summary(revision: &FoundryProjectRevision) -> FoundryHistorySummary {
    revision
        .program
        .as_ref()
        .map(program_summary)
        .unwrap_or_else(start_summary)
}

/// Summarize one stored revision program.
#[must_use]
pub(crate) fn program_summary(program: &FoundryProjectRevisionProgram) -> FoundryHistorySummary {
    let commands = program.commands();
    if commands.is_empty() {
        return FoundryHistorySummary {
            kind: FoundryHistorySummaryKind::CommandProgram,
            label: non_empty_label(program.label(), "No semantic changes"),
            detail: None,
            changed_controls: Vec::new(),
            changed_provider_roles: Vec::new(),
            accepted_candidate: None,
        };
    }

    if let [command] = commands {
        let mut summary = command_summary(command);
        let label = program.label().trim();
        if !label.is_empty() && label != summary.label && history_program_label_is_safe(label) {
            summary.detail = Some(summary.label);
            summary.label = label.to_owned();
        }
        return summary;
    }

    let command_summaries = commands.iter().map(command_summary).collect::<Vec<_>>();
    let mut changed_controls = BTreeSet::new();
    let mut changed_provider_roles = BTreeSet::new();
    let mut accepted_candidate = None;
    for summary in &command_summaries {
        changed_controls.extend(summary.changed_controls.iter().cloned());
        changed_provider_roles.extend(summary.changed_provider_roles.iter().cloned());
        accepted_candidate = accepted_candidate.or_else(|| summary.accepted_candidate.clone());
    }
    let detail = concise_join(
        command_summaries
            .iter()
            .map(|summary| summary.label.as_str())
            .collect::<Vec<_>>()
            .as_slice(),
        commands.len(),
    );

    FoundryHistorySummary {
        kind: FoundryHistorySummaryKind::CommandProgram,
        label: safe_history_program_label(program.label(), "Project changes"),
        detail: Some(detail),
        changed_controls: changed_controls.into_iter().collect(),
        changed_provider_roles: changed_provider_roles.into_iter().collect(),
        accepted_candidate,
    }
}

/// Summarize one foundry command.
#[must_use]
pub(crate) fn command_summary(command: &FoundryCommand) -> FoundryHistorySummary {
    match command {
        FoundryCommand::SetControl { control_id, value } => control_edit_summary(control_id, value),
        FoundryCommand::ResetControl { control_id } => FoundryHistorySummary {
            kind: FoundryHistorySummaryKind::ControlEdit,
            label: format!("Reset {}", friendly_history_label(control_id)),
            detail: None,
            changed_controls: vec![control_id.clone()],
            changed_provider_roles: Vec::new(),
            accepted_candidate: None,
        },
        FoundryCommand::SelectProvider { role, provider_ref } => {
            provider_change_summary(role, provider_ref)
        }
        FoundryCommand::SetRolePresence { role, enabled } => FoundryHistorySummary {
            kind: FoundryHistorySummaryKind::ControlEdit,
            label: if *enabled {
                format!("Enabled {}", friendly_history_label(role))
            } else {
                format!("Disabled {}", friendly_history_label(role))
            },
            detail: Some("Role presence changed".to_owned()),
            changed_controls: vec![role.clone()],
            changed_provider_roles: Vec::new(),
            accepted_candidate: None,
        },
        FoundryCommand::SetStyle { .. } => FoundryHistorySummary {
            kind: FoundryHistorySummaryKind::StyleChange,
            label: "Changed visual style".to_owned(),
            detail: Some("Style updated".to_owned()),
            changed_controls: Vec::new(),
            changed_provider_roles: Vec::new(),
            accepted_candidate: None,
        },
        FoundryCommand::SetLock { lock } => lock_change_summary(lock),
        FoundryCommand::ClearLock { target } => FoundryHistorySummary {
            kind: FoundryHistorySummaryKind::LockChange,
            label: format!("Cleared {}", lock_target_label(target)),
            detail: None,
            changed_controls: match target {
                FoundryLockTarget::Control(control) => vec![control.clone()],
                _ => Vec::new(),
            },
            changed_provider_roles: Vec::new(),
            accepted_candidate: None,
        },
        FoundryCommand::SetVariationIntent { intent } => FoundryHistorySummary {
            kind: FoundryHistorySummaryKind::ControlEdit,
            label: format!("Set {}", intent.human_label),
            detail: Some(intent.human_summary.clone()),
            changed_controls: Vec::new(),
            changed_provider_roles: Vec::new(),
            accepted_candidate: None,
        },
        FoundryCommand::SetVariationScope { scope } => FoundryHistorySummary {
            kind: FoundryHistorySummaryKind::ControlEdit,
            label: format!("Set scope to {}", scope.display_label()),
            detail: None,
            changed_controls: Vec::new(),
            changed_provider_roles: Vec::new(),
            accepted_candidate: None,
        },
        FoundryCommand::SetVariationChannels { channels } => FoundryHistorySummary {
            kind: FoundryHistorySummaryKind::ControlEdit,
            label: "Set variation mode".to_owned(),
            detail: Some(
                channels
                    .iter()
                    .map(|channel| channel.display_label())
                    .collect::<Vec<_>>()
                    .join(", "),
            ),
            changed_controls: Vec::new(),
            changed_provider_roles: Vec::new(),
            accepted_candidate: None,
        },
        FoundryCommand::ClearVariationFocus | FoundryCommand::ClearFocusPartGroup => {
            FoundryHistorySummary {
                kind: FoundryHistorySummaryKind::ControlEdit,
                label: "Cleared part focus".to_owned(),
                detail: Some("Returned to whole-asset variation".to_owned()),
                changed_controls: Vec::new(),
                changed_provider_roles: Vec::new(),
                accepted_candidate: None,
            }
        }
        FoundryCommand::SetFocusPartGroup { group_id } => FoundryHistorySummary {
            kind: FoundryHistorySummaryKind::ControlEdit,
            label: "Set focused part".to_owned(),
            detail: Some(friendly_history_label(group_id)),
            changed_controls: Vec::new(),
            changed_provider_roles: Vec::new(),
            accepted_candidate: None,
        },
        FoundryCommand::GenerateCandidates(request) => FoundryHistorySummary {
            kind: FoundryHistorySummaryKind::CandidateGeneration,
            label: format!("Generated {} directions", request.count),
            detail: None,
            changed_controls: Vec::new(),
            changed_provider_roles: Vec::new(),
            accepted_candidate: None,
        },
        FoundryCommand::GenerateFocusedPartCandidates { group_id, .. } => FoundryHistorySummary {
            kind: FoundryHistorySummaryKind::CandidateGeneration,
            label: "Generated focused directions".to_owned(),
            detail: Some(friendly_history_label(group_id)),
            changed_controls: Vec::new(),
            changed_provider_roles: Vec::new(),
            accepted_candidate: None,
        },
        FoundryCommand::AcceptCandidate { candidate_id } => {
            candidate_acceptance_summary(candidate_id)
        }
        FoundryCommand::RejectCandidate { .. } => FoundryHistorySummary {
            kind: FoundryHistorySummaryKind::RuntimeAction,
            label: "Rejected a direction".to_owned(),
            detail: None,
            changed_controls: Vec::new(),
            changed_provider_roles: Vec::new(),
            accepted_candidate: None,
        },
        FoundryCommand::Undo => FoundryHistorySummary {
            kind: FoundryHistorySummaryKind::RuntimeAction,
            label: "Undo".to_owned(),
            detail: Some("Moved to the parent revision".to_owned()),
            changed_controls: Vec::new(),
            changed_provider_roles: Vec::new(),
            accepted_candidate: None,
        },
        FoundryCommand::SwitchRevision { .. } => FoundryHistorySummary {
            kind: FoundryHistorySummaryKind::RuntimeAction,
            label: "Switched to an earlier step".to_owned(),
            detail: None,
            changed_controls: Vec::new(),
            changed_provider_roles: Vec::new(),
            accepted_candidate: None,
        },
        FoundryCommand::Export { .. } => FoundryHistorySummary {
            kind: FoundryHistorySummaryKind::RuntimeAction,
            label: "Exported current asset".to_owned(),
            detail: None,
            changed_controls: Vec::new(),
            changed_provider_roles: Vec::new(),
            accepted_candidate: None,
        },
        FoundryCommand::AddCurrentToPack { .. } => FoundryHistorySummary {
            kind: FoundryHistorySummaryKind::RuntimeAction,
            label: "Added current asset to pack".to_owned(),
            detail: None,
            changed_controls: Vec::new(),
            changed_provider_roles: Vec::new(),
            accepted_candidate: None,
        },
    }
}

/// Summarize a control value edit.
#[must_use]
pub(crate) fn control_edit_summary(
    control_id: &str,
    value: &ControlValue,
) -> FoundryHistorySummary {
    FoundryHistorySummary {
        kind: FoundryHistorySummaryKind::ControlEdit,
        label: format!("Changed {}", friendly_history_label(control_id)),
        detail: Some(format!("Value set to {}", friendly_control_value(value))),
        changed_controls: vec![control_id.to_owned()],
        changed_provider_roles: Vec::new(),
        accepted_candidate: None,
    }
}

/// Summarize a provider change.
#[must_use]
pub(crate) fn provider_change_summary(
    role: &str,
    _provider_ref: &CatalogContentRef,
) -> FoundryHistorySummary {
    FoundryHistorySummary {
        kind: FoundryHistorySummaryKind::ProviderChange,
        label: format!("Changed {}", friendly_history_label(role)),
        detail: Some("Option updated".to_owned()),
        changed_controls: Vec::new(),
        changed_provider_roles: vec![role.to_owned()],
        accepted_candidate: None,
    }
}

/// Summarize candidate acceptance.
#[must_use]
pub(crate) fn candidate_acceptance_summary(
    candidate_id: &FoundryCandidateId,
) -> FoundryHistorySummary {
    FoundryHistorySummary {
        kind: FoundryHistorySummaryKind::CandidateAcceptance,
        label: "Chose a direction".to_owned(),
        detail: None,
        changed_controls: Vec::new(),
        changed_provider_roles: Vec::new(),
        accepted_candidate: Some(candidate_id.clone()),
    }
}

/// Return a local-override badge for a document, if needed.
#[must_use]
pub(crate) fn local_override_badge(document: &FoundryAssetDocument) -> Option<FoundryHistoryBadge> {
    local_override_marker(&document.local_recipe_overrides)
}

/// Return a local-override badge for override rows, if needed.
#[must_use]
pub(crate) fn local_override_marker(
    overrides: &[LocalRecipeOverride],
) -> Option<FoundryHistoryBadge> {
    if overrides.is_empty() {
        return None;
    }

    Some(FoundryHistoryBadge {
        kind: FoundryHistoryBadgeKind::LocalOverrides,
        label: match overrides.len() {
            1 => "1 local override".to_owned(),
            count => format!("{count} local overrides"),
        },
        detail: Some(local_override_detail(overrides)),
    })
}

/// Format stale catalog/build warnings for a revision.
#[must_use]
pub(crate) fn stale_catalog_warning(
    revision: RevisionId,
    load_report: &FoundryProjectLoadReport,
) -> Option<String> {
    let reasons = load_report.stale_builds.get(&revision)?;
    Some(format!(
        "Stored build may be stale: {}",
        stale_reason_summary(reasons)
    ))
}

/// Summarize stale-build reasons.
#[must_use]
pub(crate) fn stale_reason_summary(reasons: &[FoundryBuildStaleReason]) -> String {
    if reasons.is_empty() {
        return "no stale reason recorded".to_owned();
    }
    concise_join(
        reasons
            .iter()
            .map(stale_reason_label)
            .collect::<Vec<_>>()
            .as_slice(),
        reasons.len(),
    )
}

/// Format save/load state from the full app state.
#[must_use]
pub(crate) fn save_load_status_for_state(state: &FoundryAppState) -> FoundrySaveLoadStatus {
    let project_file = state.project_file.as_ref();
    let project_path = state
        .project_path
        .as_deref()
        .or_else(|| project_file.and_then(|file| file.path.as_deref()));
    let load_report = state
        .load_report
        .as_ref()
        .or_else(|| project_file.map(|file| &file.load_report));
    let dirty = state.dirty || project_file.is_some_and(FoundryProjectFile::is_dirty);
    let read_only = state.read_only || load_report.is_some_and(|report| report.read_only_recovery);
    let mut status = save_load_status(project_path, project_file.is_some(), dirty, read_only);
    if let Some(message) = &state.status {
        status.detail = Some(match status.detail {
            Some(detail) => format!("{detail}; {message}"),
            None => message.clone(),
        });
    }
    status
}

/// Format save/load state from primitive inputs.
#[must_use]
pub(crate) fn save_load_status(
    path: Option<&Path>,
    has_project: bool,
    dirty: bool,
    read_only: bool,
) -> FoundrySaveLoadStatus {
    if !has_project {
        return FoundrySaveLoadStatus {
            state: FoundrySaveLoadState::NoProject,
            label: "Choose a template to start".to_owned(),
            detail: Some("Start from a template or open a saved project.".to_owned()),
            path_label: None,
            can_save: false,
            can_save_as: false,
            can_load: true,
            badges: vec![FoundryHistoryBadge {
                kind: FoundryHistoryBadgeKind::Unsaved,
                label: "Choose template".to_owned(),
                detail: None,
            }],
        };
    }

    let path_label = path.map(|path| path_label(Some(path)));
    if read_only {
        return FoundrySaveLoadStatus {
            state: FoundrySaveLoadState::ReadOnlyRecovery,
            label: "Read-only recovery".to_owned(),
            detail: Some("Loaded from embedded catalog snapshots; save is disabled.".to_owned()),
            path_label,
            can_save: false,
            can_save_as: false,
            can_load: true,
            badges: vec![FoundryHistoryBadge {
                kind: FoundryHistoryBadgeKind::ReadOnly,
                label: "Read only".to_owned(),
                detail: None,
            }],
        };
    }

    match (dirty, path.is_some()) {
        (true, true) => FoundrySaveLoadStatus {
            state: FoundrySaveLoadState::DirtySaved,
            label: "Unsaved changes".to_owned(),
            detail: Some("Project has changes since the last save.".to_owned()),
            path_label,
            can_save: true,
            can_save_as: true,
            can_load: true,
            badges: vec![FoundryHistoryBadge {
                kind: FoundryHistoryBadgeKind::Dirty,
                label: "Dirty".to_owned(),
                detail: None,
            }],
        },
        (true, false) => FoundrySaveLoadStatus {
            state: FoundrySaveLoadState::Unsaved,
            label: "Unsaved project".to_owned(),
            detail: Some("Choose a path before saving.".to_owned()),
            path_label,
            can_save: false,
            can_save_as: true,
            can_load: true,
            badges: vec![FoundryHistoryBadge {
                kind: FoundryHistoryBadgeKind::Unsaved,
                label: "Unsaved".to_owned(),
                detail: None,
            }],
        },
        (false, false) => FoundrySaveLoadStatus {
            state: FoundrySaveLoadState::Unsaved,
            label: "Unsaved project".to_owned(),
            detail: Some("Choose a path before saving.".to_owned()),
            path_label,
            can_save: false,
            can_save_as: true,
            can_load: true,
            badges: vec![FoundryHistoryBadge {
                kind: FoundryHistoryBadgeKind::Unsaved,
                label: "Unsaved".to_owned(),
                detail: None,
            }],
        },
        (false, true) => FoundrySaveLoadStatus {
            state: FoundrySaveLoadState::CleanSaved,
            label: "Saved".to_owned(),
            detail: None,
            path_label,
            can_save: false,
            can_save_as: true,
            can_load: true,
            badges: vec![FoundryHistoryBadge {
                kind: FoundryHistoryBadgeKind::Saved,
                label: "Saved".to_owned(),
                detail: None,
            }],
        },
    }
}

struct RevisionRowBuildContext<'a> {
    project: &'a FoundryProject,
    load_report: Option<&'a FoundryProjectLoadReport>,
    current_path: &'a BTreeSet<RevisionId>,
    children: &'a BTreeMap<Option<RevisionId>, Vec<&'a FoundryProjectRevision>>,
}

fn append_revision_rows(
    context: &RevisionRowBuildContext<'_>,
    parent: Option<RevisionId>,
    depth: usize,
    visited: &mut BTreeSet<RevisionId>,
    rows: &mut Vec<FoundryHistoryRow>,
) {
    let Some(items) = context.children.get(&parent) else {
        return;
    };
    for revision in items {
        if append_revision_row(context, revision, depth, visited, rows) {
            append_revision_rows(context, Some(revision.id), depth + 1, visited, rows);
        }
    }
}

fn append_revision_row(
    context: &RevisionRowBuildContext<'_>,
    revision: &FoundryProjectRevision,
    depth: usize,
    visited: &mut BTreeSet<RevisionId>,
    rows: &mut Vec<FoundryHistoryRow>,
) -> bool {
    if !visited.insert(revision.id) {
        return false;
    }

    let selected = revision.id == context.project.current_revision;
    let child_count = context.project.children_of(revision.id).len();
    let badges = revision_badges(context.project, revision, child_count, context.load_report);
    rows.push(FoundryHistoryRow {
        revision: revision.id,
        parent: revision.parent,
        depth,
        label: revision_label(revision),
        summary: revision_summary(revision),
        child_count,
        branch_label: branch_count_label(child_count),
        selected,
        on_current_path: context.current_path.contains(&revision.id),
        badges,
        switch_intent: switch_revision_intent(revision.id, selected),
        branch_intent: branch_from_revision_intent(revision.id, selected),
    });
    true
}

fn revision_badges(
    project: &FoundryProject,
    revision: &FoundryProjectRevision,
    child_count: usize,
    load_report: Option<&FoundryProjectLoadReport>,
) -> Vec<FoundryHistoryBadge> {
    let mut badges = Vec::new();
    if revision.id == project.current_revision {
        badges.push(FoundryHistoryBadge {
            kind: FoundryHistoryBadgeKind::Current,
            label: "Current".to_owned(),
            detail: None,
        });
    }
    if child_count > 1 {
        badges.push(FoundryHistoryBadge {
            kind: FoundryHistoryBadgeKind::Branch,
            label: format!("{child_count} branches"),
            detail: Some("Multiple child revisions start here.".to_owned()),
        });
    }
    if let Some(badge) = local_override_badge(&revision.document) {
        badges.push(badge);
    }
    if let Some(load_report) = load_report {
        if let Some(warning) = stale_catalog_warning(revision.id, load_report) {
            badges.push(FoundryHistoryBadge {
                kind: FoundryHistoryBadgeKind::StaleCatalog,
                label: "Stale catalog".to_owned(),
                detail: Some(warning),
            });
        }
        if load_report.recovery_revisions.contains(&revision.id) {
            badges.push(FoundryHistoryBadge {
                kind: FoundryHistoryBadgeKind::Recovery,
                label: "Recovered".to_owned(),
                detail: Some("Opened from embedded catalog snapshots.".to_owned()),
            });
        }
        if load_report.verified_recipe_revisions.contains(&revision.id) {
            badges.push(FoundryHistoryBadge {
                kind: FoundryHistoryBadgeKind::VerifiedRecipe,
                label: "Recipe verified".to_owned(),
                detail: None,
            });
        }
    }
    badges
}

fn start_summary() -> FoundryHistorySummary {
    FoundryHistorySummary {
        kind: FoundryHistorySummaryKind::Start,
        label: "Starting asset".to_owned(),
        detail: None,
        changed_controls: Vec::new(),
        changed_provider_roles: Vec::new(),
        accepted_candidate: None,
    }
}

fn lock_change_summary(lock: &shape_foundry::FoundryLock) -> FoundryHistorySummary {
    let target = lock_target_label(&lock.target);
    let mode = match lock.mode {
        FoundryLockMode::Locked => "Locked",
        FoundryLockMode::SearchProtected => "Protected",
    };
    FoundryHistorySummary {
        kind: FoundryHistorySummaryKind::LockChange,
        label: format!("{mode} {target}"),
        detail: lock.reason.clone(),
        changed_controls: Vec::new(),
        changed_provider_roles: Vec::new(),
        accepted_candidate: None,
    }
}

fn lock_target_label(target: &FoundryLockTarget) -> String {
    match target {
        FoundryLockTarget::Control(control) => format!("control {control}"),
        FoundryLockTarget::Role(role) => format!("role {role}"),
        FoundryLockTarget::Provider(role) => format!("provider {role}"),
        FoundryLockTarget::Override(id) => format!("override {id}"),
        FoundryLockTarget::ExportProfile(profile) => format!("export profile {profile}"),
        FoundryLockTarget::VariationScope(scope) => {
            format!("variation scope {}", scope.display_label())
        }
        FoundryLockTarget::VariationChannel(channel) => {
            format!("variation channel {}", channel.display_label())
        }
        FoundryLockTarget::FocusPartGroup(group_id) => {
            format!("focus part {}", friendly_history_label(group_id))
        }
        FoundryLockTarget::MaterialSlot(slot_id) => {
            format!("material slot {}", friendly_history_label(slot_id))
        }
        FoundryLockTarget::Custom(target) => target.clone(),
    }
}

fn current_path_set(project: &FoundryProject) -> BTreeSet<RevisionId> {
    project
        .revision_path_to_root()
        .unwrap_or_default()
        .into_iter()
        .collect()
}

fn revision_label(revision: &FoundryProjectRevision) -> String {
    let trimmed = revision.label.trim();
    if trimmed.is_empty() {
        format!("Revision {}", revision.id.0)
    } else {
        format!("Revision {}: {trimmed}", revision.id.0)
    }
}

fn local_override_detail(overrides: &[LocalRecipeOverride]) -> String {
    let mut pinned = 0usize;
    let mut revalidate = 0usize;
    let mut drop_on_style_change = 0usize;
    for override_row in overrides {
        match override_row.survival_policy {
            OverrideSurvivalPolicy::Pinned => pinned += 1,
            OverrideSurvivalPolicy::Revalidate => revalidate += 1,
            OverrideSurvivalPolicy::DropOnStyleChange => drop_on_style_change += 1,
        }
    }
    let mut parts = Vec::new();
    if pinned > 0 {
        parts.push(format!("{pinned} pinned"));
    }
    if revalidate > 0 {
        parts.push(format!("{revalidate} revalidate"));
    }
    if drop_on_style_change > 0 {
        parts.push(format!("{drop_on_style_change} drop on style change"));
    }
    parts.join(", ")
}

fn stale_reason_label(reason: &FoundryBuildStaleReason) -> String {
    match reason {
        FoundryBuildStaleReason::CatalogVersionChanged { stored, current } => {
            format!("catalog version {stored} -> {current}")
        }
        FoundryBuildStaleReason::CatalogCompilerVersionChanged { stored, current } => {
            format!("catalog compiler {stored} -> {current}")
        }
        FoundryBuildStaleReason::CatalogReferenceChanged { key } => {
            format!("catalog reference changed for {key}")
        }
        FoundryBuildStaleReason::FoundryVersionChanged { stored, current } => {
            format!("foundry {stored} -> {current}")
        }
        FoundryBuildStaleReason::FamilyCompileVersionChanged { stored, current } => {
            format!("family compiler {stored} -> {current}")
        }
    }
}

fn concise_join(parts: &[impl AsRef<str>], total_count: usize) -> String {
    let mut out = parts
        .iter()
        .take(MAX_SUMMARY_PARTS)
        .map(AsRef::as_ref)
        .collect::<Vec<_>>()
        .join(", ");
    if total_count > MAX_SUMMARY_PARTS {
        if !out.is_empty() {
            out.push_str(", ");
        }
        out.push_str(&format!("{} more", total_count - MAX_SUMMARY_PARTS));
    }
    out
}

fn non_empty_label(value: &str, fallback: &str) -> String {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        fallback.to_owned()
    } else {
        trimmed.to_owned()
    }
}

fn safe_history_program_label(value: &str, fallback: &str) -> String {
    let trimmed = value.trim();
    if trimmed.is_empty() || !history_program_label_is_safe(trimmed) {
        fallback.to_owned()
    } else {
        trimmed.to_owned()
    }
}

fn history_program_label_is_safe(value: &str) -> bool {
    let lowercase = value.to_ascii_lowercase();
    ![
        "provider",
        "candidate",
        "revision",
        "fingerprint",
        "schema",
        "catalog",
        "recipe",
        "stable id",
        "semantic id",
    ]
    .iter()
    .any(|marker| lowercase.contains(marker))
}

fn friendly_history_label(value: &str) -> String {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return "asset option".to_owned();
    }
    let words = trimmed
        .split(|character: char| {
            character == '_'
                || character == '-'
                || character == '.'
                || character == '/'
                || character == ':'
                || character.is_whitespace()
        })
        .filter(|word| !word.trim().is_empty())
        .filter(|word| !looks_generated_history_token(word))
        .map(|word| {
            let mut chars = word.chars();
            match chars.next() {
                Some(first) => {
                    let mut output = first.to_uppercase().collect::<String>();
                    output.push_str(&chars.as_str().to_ascii_lowercase());
                    output
                }
                None => String::new(),
            }
        })
        .collect::<Vec<_>>();
    if words.is_empty() {
        "asset option".to_owned()
    } else {
        words.join(" ")
    }
}

fn friendly_control_value(value: &ControlValue) -> String {
    match value {
        ControlValue::Scalar(value) => format_scalar(*value),
        ControlValue::Integer(value) => value.to_string(),
        ControlValue::Toggle(true) => "On".to_owned(),
        ControlValue::Toggle(false) => "Off".to_owned(),
        ControlValue::Choice(value) | ControlValue::Provider(value) => {
            friendly_history_label(value)
        }
    }
}

fn looks_generated_history_token(word: &str) -> bool {
    word.len() >= 8 && word.chars().all(|character| character.is_ascii_hexdigit())
}

fn format_control_value(value: &ControlValue) -> String {
    match value {
        ControlValue::Scalar(value) => format_scalar(*value),
        ControlValue::Integer(value) => value.to_string(),
        ControlValue::Toggle(true) => "on".to_owned(),
        ControlValue::Toggle(false) => "off".to_owned(),
        ControlValue::Choice(value) | ControlValue::Provider(value) => value.clone(),
    }
}

fn format_scalar(value: f32) -> String {
    if value.fract() == 0.0 {
        format!("{value:.0}")
    } else if value.abs() >= 100.0 {
        format!("{value:.1}")
    } else if value.abs() >= 10.0 {
        format!("{value:.2}")
    } else {
        format!("{value:.3}")
            .trim_end_matches('0')
            .trim_end_matches('.')
            .to_owned()
    }
}

fn short_fingerprint(content_ref: &CatalogContentRef) -> String {
    content_ref
        .fingerprint
        .0
        .to_hex()
        .chars()
        .take(12)
        .collect()
}

fn path_label(path: Option<&Path>) -> String {
    let Some(path) = path else {
        return "Untitled".to_owned();
    };
    path.file_name()
        .and_then(|name| name.to_str())
        .filter(|name| !name.trim().is_empty())
        .map(ToOwned::to_owned)
        .unwrap_or_else(|| path.display().to_string())
}
