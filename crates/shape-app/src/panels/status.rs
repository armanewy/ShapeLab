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
            ui.label(format!(
                "Status: {}",
                friendly_status_text(state.status.text.trim())
            ));
        }
        if let Some(triangles) = mesh_triangle_count(state) {
            ui.separator();
            ui.label(format!("{triangles} triangles"));
        }
        if let Some(error) = state.recoverable_errors.back() {
            ui.separator();
            ui.colored_label(
                ui.visuals().error_fg_color,
                format!("Needs attention: {}", friendly_error_text(error)),
            );
        }
    });
}

/// Human-facing phase label.
pub(crate) fn phase_label(phase: AppPhase) -> &'static str {
    match phase {
        AppPhase::Idle => "Ready",
        AppPhase::Loading => "Opening project",
        AppPhase::BuildingPreview => "Updating preview",
        AppPhase::GeneratingCandidates => "Finding options",
        AppPhase::Rendering => "Redrawing view",
        AppPhase::Saving => "Saving project",
        AppPhase::Exporting => "Exporting model",
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
        Some(preset) => format!("{file_or_title} / {}", preset_name(preset)),
        None => file_or_title,
    }
}

fn friendly_status_text(text: &str) -> String {
    match text {
        "Ready" => "Ready".to_owned(),
        "Building preview" => "Updating the preview".to_owned(),
        "Preview ready" => "Preview ready".to_owned(),
        "Generating directions" => "Looking for new options".to_owned(),
        "Candidate ready" => "One option card is ready".to_owned(),
        "Generation complete" => "Option search complete".to_owned(),
        "Generation cancelled" | "Job cancelled" => "Search cancelled".to_owned(),
        "Candidate accepted" => "Option chosen; preview is updating".to_owned(),
        "Candidate dismissed" => "Option dismissed".to_owned(),
        "Candidates cleared" => "Option cards cleared".to_owned(),
        "Selection updated" => "Selected part updated".to_owned(),
        "Target updated" => "Change target updated".to_owned(),
        "Search controls updated" => "Allowed value types updated".to_owned(),
        "Search budget updated" => "Option count updated".to_owned(),
        "Exploration mode updated" => "Refine/Explore choice updated".to_owned(),
        "Parameter updated" => "Value changed; preview is updating".to_owned(),
        "Parameter unchanged" => "Value unchanged".to_owned(),
        "Parameter lock updated" => "Keep setting updated".to_owned(),
        "Lock unchanged" => "Keep setting unchanged".to_owned(),
        "Preset loaded" => "Preset loaded; preview is updating".to_owned(),
        "Moved to parent revision" => "Moved back one step; preview is updating".to_owned(),
        "Revision selected" => "History step selected; preview is updating".to_owned(),
        "Saving project" => "Saving project".to_owned(),
        "Project saved" => "Project saved".to_owned(),
        "Loading project" => "Opening project".to_owned(),
        "Rendering view" => "Redrawing view".to_owned(),
        "No preview to fit" => "Preview is not ready yet".to_owned(),
        "No preview to render" => "Preview is not ready yet".to_owned(),
        "Exporting OBJ" => "Exporting model".to_owned(),
        other => other.to_owned(),
    }
}

fn friendly_error_text(error: &str) -> String {
    if error.contains("no mutable parameters") {
        "No unlocked values match the current target. Pick another part, unlock a value, or allow more value types.".to_owned()
    } else if error.contains("there is no active preset to reset") {
        "This project was not opened from a built-in preset, so it cannot be reset to one."
            .to_owned()
    } else if error.contains("not in the project") {
        "The selected part is no longer in this history step. Pick a part from the list again."
            .to_owned()
    } else if error.contains("export requires") {
        "Wait for the preview to finish before exporting.".to_owned()
    } else {
        format!("Could not finish the last action. Details: {error}")
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
