//! Status line panel.

#![allow(dead_code)]

use std::path::Path;

use shape_presets::{PresetId, list_presets};

use crate::state::{AppPhase, AppState};

/// Draw the bottom status line.
pub(crate) fn show(ui: &mut egui::Ui, state: &AppState) {
    ui.horizontal_wrapped(|ui| {
        ui.label(project_label(state));
        ui.separator();
        ui.label(if state.dirty {
            "Unsaved changes"
        } else {
            "Saved"
        });
        ui.separator();
        ui.label(format!("Phase: {}", phase_label(state.status.phase)));
        if let Some(progress) = state.status.progress {
            ui.add(
                egui::ProgressBar::new(progress.clamp(0.0, 1.0))
                    .desired_width(96.0)
                    .show_percentage(),
            );
        }
        if !state.status.text.trim().is_empty() {
            ui.separator();
            ui.label(format!("Last: {}", state.status.text.trim()));
        }
        if let Some(triangles) = mesh_triangle_count(state) {
            ui.separator();
            ui.label(format!("{triangles} triangles"));
        }
        if let Some(error) = state.recoverable_errors.back() {
            ui.separator();
            ui.colored_label(ui.visuals().error_fg_color, format!("Error: {error}"));
        }
    });
}

/// Human-facing phase label.
pub(crate) fn phase_label(phase: AppPhase) -> &'static str {
    match phase {
        AppPhase::Idle => "Idle",
        AppPhase::Loading => "Loading",
        AppPhase::BuildingPreview => "Building preview",
        AppPhase::GeneratingCandidates => "Generating directions",
        AppPhase::Rendering => "Rendering",
        AppPhase::Saving => "Saving",
        AppPhase::Exporting => "Exporting",
        AppPhase::Error => "Needs attention",
    }
}

/// Return current mesh triangle count when available.
pub(crate) fn mesh_triangle_count(state: &AppState) -> Option<usize> {
    state
        .current_preview
        .as_ref()
        .map(|preview| preview.mesh.indices.len() / 3)
}

/// Compact current project/preset label.
pub(crate) fn project_label(state: &AppState) -> String {
    let file_or_title = state
        .current_file_path
        .as_deref()
        .and_then(file_name)
        .map(str::to_owned)
        .unwrap_or_else(|| state.project.title.clone());

    match &state.active_preset {
        Some(preset) => format!("{file_or_title} - {}", preset_name(preset)),
        None => file_or_title,
    }
}

fn preset_name(preset: &PresetId) -> String {
    list_presets()
        .into_iter()
        .find(|metadata| metadata.id == *preset)
        .map(|metadata| metadata.name)
        .unwrap_or_else(|| preset.0.clone())
}

fn file_name(path: &Path) -> Option<&str> {
    path.file_name().and_then(|name| name.to_str())
}
