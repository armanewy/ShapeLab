//! Candidate direction gallery for explicit asset recipes.

#![allow(dead_code)]

use egui::{Color32, RichText};

use crate::asset::{
    AssetAppCommand, AssetCandidate, AssetCandidateEdit, AssetCandidateId, AssetJobKind,
    AssetUiState,
};

pub(crate) const VISIBLE_CANDIDATE_SLOTS: usize = 6;

/// Stable card summary consumed by the UI and tests.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct CandidateCardSummary {
    pub title: String,
    pub structural_summary: String,
    pub numeric_summary: String,
    pub edit_lines: Vec<String>,
    pub validation: String,
}

/// Render the unchanged parent card and up to six candidate cards.
pub(crate) fn show(ui: &mut egui::Ui, state: &AssetUiState) -> Vec<AssetAppCommand> {
    let mut commands = Vec::new();
    ui.heading("Directions");

    if let Some(progress) = generation_progress_label(state) {
        ui.label(progress);
    }

    egui::ScrollArea::horizontal()
        .auto_shrink([false, true])
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                render_parent_card(ui, state);
                for slot in candidate_slots(&state.candidates) {
                    match slot {
                        Some(candidate) => commands.extend(render_candidate_card(ui, candidate)),
                        None => render_empty_candidate_card(ui, state),
                    }
                }
            });
        });

    commands
}

/// Return exactly six visible candidate slots.
#[must_use]
pub(crate) fn candidate_slots(candidates: &[AssetCandidate]) -> Vec<Option<&AssetCandidate>> {
    (0..VISIBLE_CANDIDATE_SLOTS)
        .map(|index| candidates.get(index))
        .collect()
}

/// Summarize one candidate card.
#[must_use]
pub(crate) fn candidate_summary(candidate: &AssetCandidate) -> CandidateCardSummary {
    CandidateCardSummary {
        title: if candidate.title.trim().is_empty() {
            format!("Candidate {}", candidate.id.0)
        } else {
            candidate.title.clone()
        },
        structural_summary: structural_summary(candidate.structural_changes),
        numeric_summary: numeric_summary(candidate.numeric_changes),
        edit_lines: candidate_edit_lines(candidate, 4),
        validation: candidate.validation.label().to_owned(),
    }
}

/// Build the explicit accept command for a candidate card.
#[must_use]
pub(crate) fn accept_candidate_command(candidate: AssetCandidateId) -> AssetAppCommand {
    AssetAppCommand::AcceptCandidate(candidate)
}

/// Build the explicit reject command for a candidate card.
#[must_use]
pub(crate) fn reject_candidate_command(candidate: AssetCandidateId) -> AssetAppCommand {
    AssetAppCommand::RejectCandidate(candidate)
}

/// Return generation progress copy when candidate search is active.
#[must_use]
pub(crate) fn generation_progress_label(state: &AssetUiState) -> Option<String> {
    let progress = state.active_job.as_ref()?;
    (progress.kind == AssetJobKind::CandidateSearch).then(|| {
        format!(
            "{}: {}/{} ({:.0}%)",
            progress.phase,
            progress.completed,
            progress.total,
            progress.fraction() * 100.0
        )
    })
}

/// Format candidate edit lines and preserve an explicit empty explanation.
#[must_use]
pub(crate) fn candidate_edit_lines(candidate: &AssetCandidate, limit: usize) -> Vec<String> {
    let mut lines = candidate
        .edits
        .iter()
        .take(limit)
        .map(edit_line)
        .collect::<Vec<_>>();

    if candidate.edits.len() > lines.len() {
        lines.push(format!(
            "{} more change(s)",
            candidate.edits.len() - lines.len()
        ));
    }
    if lines.is_empty() {
        lines.push(empty_explanatory_edit_list().to_owned());
    }
    lines
}

/// Empty candidate explanation copy.
#[must_use]
pub(crate) fn empty_explanatory_edit_list() -> &'static str {
    "No explanatory edit list"
}

fn render_parent_card(ui: &mut egui::Ui, state: &AssetUiState) {
    egui::Frame::group(ui.style()).show(ui, |ui| {
        ui.set_min_width(210.0);
        ui.label(RichText::new("Current asset").strong());
        ui.label("Unchanged parent");
        ui.small(format!("{} part(s)", state.parts.len()));
        if state.validation.is_empty() {
            ui.small("Validation: no warnings");
        } else {
            ui.colored_label(
                Color32::from_rgb(168, 112, 42),
                format!("{} validation note(s)", state.validation.len()),
            );
        }
    });
}

fn render_candidate_card(ui: &mut egui::Ui, candidate: &AssetCandidate) -> Vec<AssetAppCommand> {
    let mut commands = Vec::new();
    let summary = candidate_summary(candidate);
    egui::Frame::group(ui.style()).show(ui, |ui| {
        ui.set_min_width(230.0);
        ui.label(RichText::new(&summary.title).strong());
        ui.small(summary.structural_summary);
        ui.small(summary.numeric_summary);
        ui.label("Changes");
        for line in &summary.edit_lines {
            ui.small(line);
        }
        let validation_response = match &candidate.validation.detail() {
            Some(detail) => ui
                .label(format!("Validation: {}", summary.validation))
                .on_hover_text(*detail),
            None => ui.label(format!("Validation: {}", summary.validation)),
        };
        let _ = validation_response;
        if ui.button("Choose This Direction").clicked() {
            commands.push(accept_candidate_command(candidate.id));
        }
        if ui.small_button("Dismiss").clicked() {
            commands.push(reject_candidate_command(candidate.id));
        }
    });
    commands
}

fn render_empty_candidate_card(ui: &mut egui::Ui, state: &AssetUiState) {
    egui::Frame::group(ui.style()).show(ui, |ui| {
        ui.set_min_width(210.0);
        ui.label(RichText::new("Candidate").strong());
        if state.active_job.is_some() {
            ui.small("Generating");
        } else {
            ui.small("No direction yet");
        }
    });
}

fn structural_summary(count: usize) -> String {
    match count {
        0 => "No structural changes".to_owned(),
        1 => "1 structural change".to_owned(),
        _ => format!("{count} structural changes"),
    }
}

fn numeric_summary(count: usize) -> String {
    match count {
        0 => "No value changes".to_owned(),
        1 => "1 value change".to_owned(),
        _ => format!("{count} value changes"),
    }
}

fn edit_line(edit: &AssetCandidateEdit) -> String {
    match (edit.before, edit.after) {
        (Some(before), Some(after)) => {
            let direction = if after >= before {
                "increases"
            } else {
                "decreases"
            };
            format!(
                "{} {} {}: {} -> {}",
                edit.subject,
                edit.label,
                direction,
                format_scalar(before),
                format_scalar(after)
            )
        }
        _ if edit.structural => format!("{} {}", edit.subject, edit.label),
        _ => format!("{} {}", edit.subject, edit.label),
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
