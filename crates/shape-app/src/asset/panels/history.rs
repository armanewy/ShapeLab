//! Branchable asset revision history panel.

#![allow(dead_code)]

use std::collections::{BTreeMap, BTreeSet};

use egui::RichText;

use crate::asset::{AssetAppCommand, AssetHistoryRevision, AssetRevisionId, AssetUiState};

/// Stable flattened revision row.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct HistoryRow {
    pub revision: AssetRevisionId,
    pub depth: usize,
    pub label: String,
    pub operation_summary: String,
    pub branch_label: String,
    pub selected: bool,
}

/// Render branchable history and emit revision commands.
pub(crate) fn show(ui: &mut egui::Ui, state: &AssetUiState) -> Vec<AssetAppCommand> {
    let mut commands = Vec::new();
    ui.heading("History");

    let rows = build_history_rows(&state.history);
    if rows.is_empty() {
        ui.weak(empty_history_message());
        return commands;
    }

    for row in rows {
        ui.horizontal(|ui| {
            ui.add_space(row.depth as f32 * 14.0);
            let label = if row.selected {
                RichText::new(&row.label).strong()
            } else {
                RichText::new(&row.label)
            };
            if ui
                .selectable_label(row.selected, label)
                .on_hover_text(format!("revision.{}", row.revision.0))
                .clicked()
            {
                commands.extend(switch_revision_command(row.revision, row.selected));
            }
            ui.small(row.operation_summary);
            if !row.branch_label.is_empty() {
                ui.small(row.branch_label);
            }
        });
    }

    if ui.button("Undo").clicked() {
        commands.extend(undo_command(can_undo(&state.history)));
    }
    commands
}

/// Build rows for a branchable revision tree.
#[must_use]
pub(crate) fn build_history_rows(revisions: &[AssetHistoryRevision]) -> Vec<HistoryRow> {
    let mut children = BTreeMap::<Option<AssetRevisionId>, Vec<&AssetHistoryRevision>>::new();
    for revision in revisions {
        children.entry(revision.parent).or_default().push(revision);
    }
    for items in children.values_mut() {
        items.sort_by_key(|revision| revision.id);
    }

    let mut rows = Vec::new();
    let mut visited = BTreeSet::new();
    append_revision_rows(None, 0, &children, &mut visited, &mut rows);
    for revision in revisions {
        if !visited.contains(&revision.id) {
            append_revision_row(revision, 0, &mut visited, &mut rows);
        }
    }
    rows
}

/// Emit a switch command only when the row is not already selected.
#[must_use]
pub(crate) fn switch_revision_command(
    revision: AssetRevisionId,
    selected: bool,
) -> Option<AssetAppCommand> {
    (!selected).then_some(AssetAppCommand::SwitchBranch(revision))
}

/// Emit undo only when there is a selected revision with a parent.
#[must_use]
pub(crate) fn undo_command(can_undo: bool) -> Option<AssetAppCommand> {
    can_undo.then_some(AssetAppCommand::Undo)
}

/// Return true when undo should be enabled.
#[must_use]
pub(crate) fn can_undo(revisions: &[AssetHistoryRevision]) -> bool {
    revisions
        .iter()
        .any(|revision| revision.selected && revision.parent.is_some())
}

/// Concise branch status label.
#[must_use]
pub(crate) fn branch_label(child_count: usize) -> String {
    match child_count {
        0 => String::new(),
        1 => "1 child".to_owned(),
        _ => format!("{child_count} branches"),
    }
}

/// Empty-state copy for tests and the UI.
#[must_use]
pub(crate) fn empty_history_message() -> &'static str {
    "No asset revisions yet."
}

fn append_revision_rows(
    parent: Option<AssetRevisionId>,
    depth: usize,
    children: &BTreeMap<Option<AssetRevisionId>, Vec<&AssetHistoryRevision>>,
    visited: &mut BTreeSet<AssetRevisionId>,
    rows: &mut Vec<HistoryRow>,
) {
    let Some(items) = children.get(&parent) else {
        return;
    };
    for revision in items {
        if append_revision_row(revision, depth, visited, rows) {
            append_revision_rows(Some(revision.id), depth + 1, children, visited, rows);
        }
    }
}

fn append_revision_row(
    revision: &AssetHistoryRevision,
    depth: usize,
    visited: &mut BTreeSet<AssetRevisionId>,
    rows: &mut Vec<HistoryRow>,
) -> bool {
    if !visited.insert(revision.id) {
        return false;
    }

    rows.push(HistoryRow {
        revision: revision.id,
        depth,
        label: history_label(revision),
        operation_summary: concise_operation_summary(&revision.operation_summary),
        branch_label: branch_label(revision.child_count),
        selected: revision.selected,
    });
    true
}

fn history_label(revision: &AssetHistoryRevision) -> String {
    if revision.label.trim().is_empty() {
        format!("Revision {}", revision.id.0)
    } else {
        format!("Revision {}: {}", revision.id.0, revision.label)
    }
}

fn concise_operation_summary(summary: &str) -> String {
    let trimmed = summary.trim();
    if trimmed.is_empty() {
        "No operation summary".to_owned()
    } else if trimmed.len() > 52 {
        format!("{}...", &trimmed[..49])
    } else {
        trimmed.to_owned()
    }
}
