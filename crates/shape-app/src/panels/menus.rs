//! Application menu bar.

#![allow(dead_code)]

use std::path::PathBuf;

use shape_presets::{PresetId, list_presets};

use crate::commands::AppCommand;
use crate::state::AppState;

/// Path-taking menu actions that use native dialogs in the UI layer.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub(crate) enum PathMenuAction {
    OpenProject,
    SaveAs,
    ExportCurrentObj,
}

/// Direct menu actions that do not require additional data.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub(crate) enum DirectMenuAction {
    Save,
    Exit,
    Undo,
    FitView,
    ClearCandidates,
}

/// Draw the top-level menu bar and return emitted commands.
pub(crate) fn show(ui: &mut egui::Ui, state: &AppState) -> Vec<AppCommand> {
    let mut commands = Vec::new();
    egui::MenuBar::new().ui(ui, |ui| {
        ui.menu_button("File", |ui| {
            ui.menu_button("New From Preset", |ui| {
                for preset in list_presets() {
                    let response = ui
                        .button(&preset.name)
                        .on_hover_text(preset.description.clone());
                    if response.clicked() {
                        commands.push(command_for_preset(preset.id));
                        ui.close();
                    }
                }
            });

            if ui.button("Open Project...").clicked() {
                if let Some(path) = pick_project_file() {
                    commands.push(command_for_path_action(PathMenuAction::OpenProject, path));
                }
                ui.close();
            }
            if ui
                .add_enabled(
                    state.dirty || state.current_file_path.is_some(),
                    egui::Button::new("Save"),
                )
                .clicked()
            {
                commands.push(command_for_direct_action(DirectMenuAction::Save));
                ui.close();
            }
            if ui.button("Save As...").clicked() {
                if let Some(path) = save_project_file(&state.project.title) {
                    commands.push(command_for_path_action(PathMenuAction::SaveAs, path));
                }
                ui.close();
            }
            if ui.button("Export Current OBJ...").clicked() {
                if let Some(path) = export_obj_file(&state.project.title) {
                    commands.push(command_for_path_action(
                        PathMenuAction::ExportCurrentObj,
                        path,
                    ));
                }
                ui.close();
            }
            ui.separator();
            if ui.button("Exit").clicked() {
                commands.push(command_for_direct_action(DirectMenuAction::Exit));
                ui.close();
            }
        });

        ui.menu_button("Edit", |ui| {
            if ui
                .add_enabled(state.project.can_undo(), egui::Button::new("Undo"))
                .clicked()
            {
                commands.push(command_for_direct_action(DirectMenuAction::Undo));
                ui.close();
            }
            if ui.button("Clear Candidates").clicked() {
                commands.push(command_for_direct_action(DirectMenuAction::ClearCandidates));
                ui.close();
            }
        });

        ui.menu_button("View", |ui| {
            if ui.button("Fit View").clicked() {
                commands.push(command_for_direct_action(DirectMenuAction::FitView));
                ui.close();
            }
        });

        ui.menu_button("Help", |ui| {
            ui.strong("About Shape Lab");
            ui.label(about_text());
        });
    });
    commands
}

/// Command emitted by choosing a built-in preset.
pub(crate) fn command_for_preset(preset: PresetId) -> AppCommand {
    AppCommand::LoadPreset(preset)
}

/// Command emitted by a direct menu action.
pub(crate) fn command_for_direct_action(action: DirectMenuAction) -> AppCommand {
    match action {
        DirectMenuAction::Save => AppCommand::Save,
        DirectMenuAction::Exit => AppCommand::Exit,
        DirectMenuAction::Undo => AppCommand::Undo,
        DirectMenuAction::FitView => AppCommand::FitView,
        DirectMenuAction::ClearCandidates => AppCommand::ClearCandidates,
    }
}

/// Command emitted after a native file dialog returns a path.
pub(crate) fn command_for_path_action(action: PathMenuAction, path: PathBuf) -> AppCommand {
    match action {
        PathMenuAction::OpenProject => AppCommand::OpenProject(path),
        PathMenuAction::SaveAs => AppCommand::SaveAs(path),
        PathMenuAction::ExportCurrentObj => AppCommand::ExportCurrentObj(path),
    }
}

/// About text shown directly in the Help menu.
pub(crate) fn about_text() -> &'static str {
    "Native offline preference-guided shape exploration. Geometry, rendering, and project I/O stay local to this application."
}

/// Suggested project filename derived from a user-visible title.
pub(crate) fn suggested_project_file_name(title: &str) -> String {
    format!("{}.shapelab.json", safe_file_stem(title))
}

/// Suggested OBJ filename derived from a user-visible title.
pub(crate) fn suggested_obj_file_name(title: &str) -> String {
    format!("{}.obj", safe_file_stem(title))
}

fn pick_project_file() -> Option<PathBuf> {
    rfd::FileDialog::new()
        .add_filter("Shape Lab project", &["shapelab.json", "json"])
        .pick_file()
}

fn save_project_file(title: &str) -> Option<PathBuf> {
    rfd::FileDialog::new()
        .add_filter("Shape Lab project", &["shapelab.json", "json"])
        .set_file_name(suggested_project_file_name(title))
        .save_file()
}

fn export_obj_file(title: &str) -> Option<PathBuf> {
    rfd::FileDialog::new()
        .add_filter("Wavefront OBJ", &["obj"])
        .set_file_name(suggested_obj_file_name(title))
        .save_file()
}

fn safe_file_stem(title: &str) -> String {
    let mut stem = String::new();
    let mut pending_separator = false;

    for character in title.trim().chars() {
        if character.is_ascii_alphanumeric() {
            if pending_separator && !stem.is_empty() {
                stem.push('-');
            }
            stem.push(character.to_ascii_lowercase());
            pending_separator = false;
        } else if !stem.is_empty() {
            pending_separator = true;
        }

        if stem.len() >= 64 {
            break;
        }
    }

    let stem = stem.trim_matches('-');
    let stem = if stem.is_empty() { "untitled" } else { stem };
    if is_windows_reserved_filename(stem) {
        format!("shape-{stem}")
    } else {
        stem.to_owned()
    }
}

fn is_windows_reserved_filename(stem: &str) -> bool {
    matches!(
        stem,
        "con"
            | "prn"
            | "aux"
            | "nul"
            | "com1"
            | "com2"
            | "com3"
            | "com4"
            | "com5"
            | "com6"
            | "com7"
            | "com8"
            | "com9"
            | "lpt1"
            | "lpt2"
            | "lpt3"
            | "lpt4"
            | "lpt5"
            | "lpt6"
            | "lpt7"
            | "lpt8"
            | "lpt9"
    )
}
