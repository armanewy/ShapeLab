//! Candidate gallery panel.

#![allow(dead_code)]

use std::collections::BTreeMap;

use egui::{Align2, Color32, CornerRadius, FontId, Sense, Stroke, StrokeKind, Vec2};
use shape_core::{CandidateId, ParamPath, ShapeDocument};
use shape_search::Candidate;

use crate::commands::AppCommand;
use crate::jobs::CandidatePreview;
use crate::state::{AppPhase, AppState};

const DEFAULT_CANDIDATE_SLOTS: usize = 6;
const CARD_WIDTH: f32 = 220.0;
const CARD_HEIGHT: f32 = 290.0;
const THUMBNAIL_HEIGHT: f32 = 112.0;

/// Draw the horizontally scrollable candidate gallery.
pub(crate) fn show(ui: &mut egui::Ui, state: &AppState) -> Vec<AppCommand> {
    let mut commands = Vec::new();

    if is_generation_visible(state) {
        ui.horizontal(|ui| {
            let progress = state.status.progress.unwrap_or(0.0).clamp(0.0, 1.0);
            ui.add(
                egui::ProgressBar::new(progress)
                    .desired_width(220.0)
                    .text(format!("Preparing directions: {:.0}%", progress * 100.0)),
            );
            if ui.button("Cancel").clicked() {
                commands.push(AppCommand::CancelActiveGeneration);
            }
        });
    }

    let parent_document = state.project.current_document().ok();
    egui::ScrollArea::horizontal()
        .auto_shrink([false, true])
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                show_parent_card(ui, state);
                for slot in 0..DEFAULT_CANDIDATE_SLOTS {
                    if let Some(preview) = preview_for_slot(&state.candidate_slots, slot) {
                        commands.extend(show_candidate_card(ui, parent_document, preview));
                    } else {
                        show_loading_slot(ui, slot, state);
                    }
                }
            });
        });

    commands
}

/// Build the explicit accept command for a candidate card.
pub(crate) fn accept_candidate_command(candidate: CandidateId) -> AppCommand {
    AppCommand::AcceptCandidate(candidate)
}

/// Build the optional dismiss command for a candidate card.
pub(crate) fn dismiss_candidate_command(candidate: CandidateId) -> AppCommand {
    AppCommand::DismissCandidate(candidate)
}

/// Return a stable human-facing label for a candidate slot.
pub(crate) fn stable_candidate_label(slot: usize, candidate: &Candidate) -> String {
    let trimmed = candidate.edit.label.trim();
    if trimmed.is_empty() {
        format!("Direction {}", slot + 1)
    } else {
        format!("Direction {}: {trimmed}", slot + 1)
    }
}

/// Translate descriptor distance into beginner-facing language.
pub(crate) fn distance_label(distance: f32) -> &'static str {
    if !distance.is_finite() {
        "Change size unknown"
    } else if distance < 0.08 {
        "Subtle change"
    } else if distance < 0.18 {
        "Clear change"
    } else if distance < 0.35 {
        "Strong change"
    } else {
        "Large change"
    }
}

/// Format up to `limit` changed parameters from a candidate edit.
pub(crate) fn candidate_difference_lines(
    parent: &ShapeDocument,
    candidate: &Candidate,
    limit: usize,
) -> Vec<String> {
    let descriptor_labels = descriptor_labels(parent);
    let mut lines = candidate
        .edit
        .operations
        .iter()
        .take(limit)
        .map(|operation| {
            let node_name = parent
                .nodes
                .get(&operation.path.node)
                .or_else(|| candidate.document.nodes.get(&operation.path.node))
                .map(|node| node.name.as_str())
                .unwrap_or("Shape part");
            let parameter = descriptor_labels
                .get(&operation.path)
                .cloned()
                .unwrap_or_else(|| friendly_parameter_label(&operation.path.key));
            format!(
                "{node_name} {parameter}: {} -> {}",
                format_scalar(operation.before),
                format_scalar(operation.after)
            )
        })
        .collect::<Vec<_>>();

    if lines.is_empty() {
        lines.push("No visible parameter changes".to_owned());
    }
    lines
}

fn show_parent_card(ui: &mut egui::Ui, state: &AppState) {
    let response = egui::Frame::group(ui.style())
        .show(ui, |ui| {
            ui.set_min_size(Vec2::new(CARD_WIDTH, CARD_HEIGHT));
            ui.heading("Current shape");
            ui.label("Control card");
            draw_thumbnail_frame(
                ui,
                state
                    .current_preview
                    .as_ref()
                    .map(|preview| (preview.image.width, preview.image.height)),
                "Unchanged",
            );
            ui.label("Use this to compare every direction against the shape you have now.");
            if let Some(preview) = &state.current_preview {
                ui.label(format!("{} triangles", preview.mesh.indices.len() / 3));
            } else {
                ui.label("Preview waiting");
            }
        })
        .response;

    paint_hover_outline(ui, &response);
}

fn show_candidate_card(
    ui: &mut egui::Ui,
    parent_document: Option<&ShapeDocument>,
    preview: &CandidatePreview,
) -> Vec<AppCommand> {
    let mut commands = Vec::new();
    let label = stable_candidate_label(preview.slot, &preview.candidate);
    let response = egui::Frame::group(ui.style())
        .show(ui, |ui| {
            ui.set_min_size(Vec2::new(CARD_WIDTH, CARD_HEIGHT));
            ui.heading(label);
            ui.label(distance_label(preview.candidate.distance_from_parent));
            draw_thumbnail_frame(
                ui,
                Some((preview.image.width, preview.image.height)),
                "Thumbnail ready",
            );
            ui.label("Top changes");
            if let Some(parent) = parent_document {
                for line in candidate_difference_lines(parent, &preview.candidate, 3) {
                    ui.small(line);
                }
            } else {
                ui.small("Current project unavailable");
            }
            ui.add_space(4.0);
            if ui.button("Choose This Direction").clicked() {
                commands.push(accept_candidate_command(preview.candidate.id));
            }
            if ui.small_button("Dismiss").clicked() {
                commands.push(dismiss_candidate_command(preview.candidate.id));
            }
        })
        .response;

    paint_hover_outline(ui, &response);
    commands
}

fn show_loading_slot(ui: &mut egui::Ui, slot: usize, state: &AppState) {
    let response = egui::Frame::group(ui.style())
        .show(ui, |ui| {
            ui.set_min_size(Vec2::new(CARD_WIDTH, CARD_HEIGHT));
            ui.heading(format!("Direction {}", slot + 1));
            let text = if state.active_generation.is_some() {
                "Generating"
            } else {
                "Empty slot"
            };
            draw_thumbnail_frame(ui, None, text);
            ui.label("A generated direction will appear here.");
        })
        .response;

    paint_hover_outline(ui, &response);
}

fn draw_thumbnail_frame(ui: &mut egui::Ui, image_size: Option<(u32, u32)>, label: &str) {
    let width = (CARD_WIDTH - 18.0).max(1.0);
    let (rect, _) = ui.allocate_exact_size(Vec2::new(width, THUMBNAIL_HEIGHT), Sense::hover());
    let fill = if image_size.is_some() {
        Color32::from_rgb(48, 55, 61)
    } else {
        Color32::from_rgb(35, 37, 40)
    };
    ui.painter().rect_filled(rect, CornerRadius::same(6), fill);
    ui.painter().rect_stroke(
        rect,
        CornerRadius::same(6),
        Stroke::new(1.0, Color32::from_rgb(92, 101, 108)),
        StrokeKind::Inside,
    );
    let text = match image_size {
        Some((width, height)) => format!("{label}\n{width} x {height}"),
        None => label.to_owned(),
    };
    ui.painter().text(
        rect.center(),
        Align2::CENTER_CENTER,
        text,
        FontId::proportional(12.0),
        Color32::from_rgb(218, 222, 226),
    );
}

fn paint_hover_outline(ui: &egui::Ui, response: &egui::Response) {
    if response.hovered() {
        ui.painter().rect_stroke(
            response.rect,
            CornerRadius::same(7),
            Stroke::new(2.0, ui.visuals().selection.stroke.color),
            StrokeKind::Outside,
        );
    }
}

fn is_generation_visible(state: &AppState) -> bool {
    state.active_generation.is_some() || state.status.phase == AppPhase::GeneratingCandidates
}

fn preview_for_slot(slots: &[CandidatePreview], slot: usize) -> Option<&CandidatePreview> {
    slots.iter().find(|preview| preview.slot == slot)
}

fn descriptor_labels(document: &ShapeDocument) -> BTreeMap<ParamPath, String> {
    shape_core::enumerate_parameters(document)
        .into_iter()
        .map(|descriptor| (descriptor.path, descriptor.label))
        .collect()
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
        _ => key
            .rsplit('.')
            .next()
            .map(title_case_token)
            .unwrap_or_else(|| "Parameter".to_owned()),
    }
}

fn title_case_token(value: &str) -> String {
    let mut chars = value.chars();
    let Some(first) = chars.next() else {
        return String::new();
    };
    let mut result = first.to_uppercase().collect::<String>();
    result.push_str(chars.as_str());
    result
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
