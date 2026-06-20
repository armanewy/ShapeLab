//! Revision history panel.

#![allow(dead_code)]

use shape_core::{EditProgram, RevisionId};
use shape_project::{Project, Revision};

use crate::commands::AppCommand;
use crate::state::AppState;

/// Draw the branchable revision history.
pub(crate) fn show(ui: &mut egui::Ui, state: &AppState) -> Vec<AppCommand> {
    let mut commands = Vec::new();
    let project = &state.project;
    let current = project.current_revision;

    ui.horizontal(|ui| {
        ui.heading(format!("Revision {}", current.0));
        if ui
            .add_enabled(project.can_undo(), egui::Button::new("Undo"))
            .clicked()
        {
            commands.push(undo_command());
        }
    });

    if let Ok(revision) = project.current() {
        ui.label(&revision.label);
        ui.small(edit_summary(revision.edit.as_ref()));
    }

    ui.separator();
    ui.label("Path to start");
    match project.revision_path_to_root() {
        Ok(mut path) => {
            path.reverse();
            ui.horizontal_wrapped(|ui| {
                for revision_id in path {
                    if let Some(command) = revision_button(ui, project, revision_id, current) {
                        commands.push(command);
                    }
                }
            });
        }
        Err(error) => {
            ui.colored_label(
                ui.visuals().error_fg_color,
                format!("History error: {error}"),
            );
        }
    }

    ui.separator();
    ui.label("Directions from this revision");
    let children = project.children_of(current);
    if children.is_empty() {
        ui.small("No child directions yet.");
    } else {
        for child in children {
            if let Some(command) = revision_row(ui, project, child, current) {
                commands.push(command);
            }
        }
    }

    ui.separator();
    ui.label("Branch points");
    let mut branch_count = 0usize;
    for revision in project.revisions.values() {
        if project.children_of(revision.id).len() > 1 {
            branch_count += 1;
            ui.horizontal(|ui| {
                if let Some(command) = revision_button(ui, project, revision.id, current) {
                    commands.push(command);
                }
                ui.small(branch_label(project, revision.id));
            });
        }
    }
    if branch_count == 0 {
        ui.small("No branch points yet.");
    }

    commands
}

/// Build the history switch command.
pub(crate) fn switch_revision_command(revision: RevisionId) -> AppCommand {
    AppCommand::SwitchRevision(revision)
}

/// Build the history undo command.
pub(crate) fn undo_command() -> AppCommand {
    AppCommand::Undo
}

/// Format a concise edit summary for a revision.
pub(crate) fn edit_summary(edit: Option<&EditProgram>) -> String {
    let Some(edit) = edit else {
        return "Starting shape".to_owned();
    };
    if edit.operations.is_empty() {
        return "No parameter changes".to_owned();
    }

    let mut parts = edit
        .operations
        .iter()
        .take(3)
        .map(|operation| {
            format!(
                "{} {} -> {}",
                friendly_parameter_label(&operation.path.key),
                format_scalar(operation.before),
                format_scalar(operation.after)
            )
        })
        .collect::<Vec<_>>();
    if edit.operations.len() > parts.len() {
        parts.push(format!("{} more", edit.operations.len() - parts.len()));
    }
    parts.join(", ")
}

/// Label how many direct child branches leave a revision.
pub(crate) fn branch_label(project: &Project, revision: RevisionId) -> String {
    match project.children_of(revision).len() {
        0 => "No child directions".to_owned(),
        1 => "1 child direction".to_owned(),
        count => format!("Branch point: {count} directions"),
    }
}

fn revision_button(
    ui: &mut egui::Ui,
    project: &Project,
    revision_id: RevisionId,
    current: RevisionId,
) -> Option<AppCommand> {
    let revision = project.revisions.get(&revision_id)?;
    let label = short_revision_label(revision);
    let response = ui.selectable_label(revision_id == current, label);
    response
        .on_hover_text(edit_summary(revision.edit.as_ref()))
        .clicked()
        .then(|| switch_revision_command(revision_id))
}

fn revision_row(
    ui: &mut egui::Ui,
    project: &Project,
    revision_id: RevisionId,
    current: RevisionId,
) -> Option<AppCommand> {
    let revision = project.revisions.get(&revision_id)?;
    let mut command = None;
    ui.horizontal(|ui| {
        if let Some(next) = revision_button(ui, project, revision_id, current) {
            command = Some(next);
        }
        ui.small(edit_summary(revision.edit.as_ref()));
        let label = branch_label(project, revision_id);
        if label.starts_with("Branch point") {
            ui.small(label);
        }
    });
    command
}

fn short_revision_label(revision: &Revision) -> String {
    let trimmed = revision.label.trim();
    if trimmed.is_empty() {
        format!("#{}", revision.id.0)
    } else {
        format!("#{} {trimmed}", revision.id.0)
    }
}

fn friendly_parameter_label(key: &str) -> String {
    match key {
        "transform.translation.x" => "Position X".to_owned(),
        "transform.translation.y" => "Position Y".to_owned(),
        "transform.translation.z" => "Position Z".to_owned(),
        "transform.rotation_degrees.x" => "Rotation X".to_owned(),
        "transform.rotation_degrees.y" => "Rotation Y".to_owned(),
        "transform.rotation_degrees.z" => "Rotation Z".to_owned(),
        "transform.scale.x" => "Scale X".to_owned(),
        "transform.scale.y" => "Scale Y".to_owned(),
        "transform.scale.z" => "Scale Z".to_owned(),
        "primitive.radius" => "Radius".to_owned(),
        "primitive.half_extents.x" => "Half Width".to_owned(),
        "primitive.half_extents.y" => "Half Height".to_owned(),
        "primitive.half_extents.z" => "Half Depth".to_owned(),
        "primitive.roundness" => "Roundness".to_owned(),
        "primitive.half_length" => "Half Length".to_owned(),
        "primitive.half_height" => "Half Height".to_owned(),
        "primitive.major_radius" => "Major Radius".to_owned(),
        "primitive.minor_radius" => "Minor Radius".to_owned(),
        "csg.smoothness" => "Blend Smoothness".to_owned(),
        _ => key.rsplit('.').next().unwrap_or("Parameter").to_owned(),
    }
}

fn format_scalar(value: f32) -> String {
    if value.abs() >= 100.0 {
        format!("{value:.0}")
    } else if value.abs() >= 10.0 {
        format!("{value:.1}")
    } else {
        format!("{value:.2}")
    }
}
