//! Candidate gallery panel.

#![allow(dead_code)]

use std::collections::BTreeMap;

use egui::{
    Align2, Color32, CornerRadius, FontId, Pos2, Rect, Sense, Stroke, StrokeKind, TextureHandle,
    Vec2,
};
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
pub(crate) fn show(
    ui: &mut egui::Ui,
    state: &AppState,
    current_texture: Option<&TextureHandle>,
    candidate_textures: &BTreeMap<CandidateId, TextureHandle>,
) -> Vec<AppCommand> {
    let mut commands = Vec::new();

    if is_generation_visible(state) {
        ui.horizontal(|ui| {
            let progress = state.status.progress.unwrap_or(0.0).clamp(0.0, 1.0);
            ui.add(
                egui::ProgressBar::new(progress)
                    .desired_width(220.0)
                    .text(format!("Finding usable options: {:.0}%", progress * 100.0)),
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
                show_parent_card(ui, state, current_texture);
                for slot in 0..DEFAULT_CANDIDATE_SLOTS {
                    if let Some(preview) = preview_for_slot(&state.candidate_slots, slot) {
                        commands.extend(show_candidate_card(
                            ui,
                            parent_document,
                            preview,
                            candidate_textures.get(&preview.candidate.id),
                        ));
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
        format!("Option {}", slot + 1)
    } else {
        let beginner_label = trimmed
            .replace("direction", "option")
            .replace("Direction", "Option");
        format!("Option {}: {beginner_label}", slot + 1)
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

fn beginner_distance_label(distance: f32) -> &'static str {
    match distance_label(distance) {
        "Change size unknown" => "Change amount unknown",
        "Subtle change" => "Tiny change",
        "Clear change" => "Noticeable change",
        "Strong change" => "Bold change",
        "Large change" => "Very different",
        other => other,
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
                .unwrap_or("Model part");
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

fn beginner_candidate_difference_lines(
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
                .unwrap_or("Model part");
            let parameter = descriptor_labels
                .get(&operation.path)
                .cloned()
                .unwrap_or_else(|| friendly_parameter_label(&operation.path.key));
            change_line(
                node_name,
                &operation.path.key,
                &parameter,
                operation.before,
                operation.after,
            )
        })
        .collect::<Vec<_>>();

    if candidate.edit.operations.len() > lines.len() {
        lines.push(format!(
            "{} more value change(s)",
            candidate.edit.operations.len() - lines.len()
        ));
    }
    if lines.is_empty() {
        lines.push("No listed value changes".to_owned());
    }
    lines
}

fn show_parent_card(ui: &mut egui::Ui, state: &AppState, current_texture: Option<&TextureHandle>) {
    let response = egui::Frame::group(ui.style())
        .show(ui, |ui| {
            ui.set_min_size(Vec2::new(CARD_WIDTH, CARD_HEIGHT));
            ui.heading("Current model");
            ui.label("Unchanged control");
            draw_thumbnail_frame(
                ui,
                current_texture,
                state
                    .current_preview
                    .as_ref()
                    .map(|preview| (preview.image.width, preview.image.height)),
                "Current model\nunchanged",
            );
            ui.label("This is not a result. Use it to compare options against what you have now.");
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
    texture: Option<&TextureHandle>,
) -> Vec<AppCommand> {
    let mut commands = Vec::new();
    let label = stable_candidate_label(preview.slot, &preview.candidate);
    let response = egui::Frame::group(ui.style())
        .show(ui, |ui| {
            ui.set_min_size(Vec2::new(CARD_WIDTH, CARD_HEIGHT));
            ui.heading(label);
            ui.label(beginner_distance_label(
                preview.candidate.distance_from_parent,
            ));
            draw_thumbnail_frame(
                ui,
                texture,
                Some((preview.image.width, preview.image.height)),
                "Thumbnail ready",
            );
            ui.label("What changes");
            if let Some(parent) = parent_document {
                for line in beginner_candidate_difference_lines(parent, &preview.candidate, 3) {
                    ui.small(line);
                }
            } else {
                ui.small("Current model unavailable");
            }
            ui.add_space(4.0);
            if ui.button("Choose This Option").clicked() {
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
            ui.heading(format!("Option {}", slot + 1));
            let text = if state.active_generation.is_some() {
                "Searching"
            } else {
                "No option yet"
            };
            draw_thumbnail_frame(ui, None, None, text);
            ui.label("Generated options will appear here after you press Generate Options.");
        })
        .response;

    paint_hover_outline(ui, &response);
}

fn draw_thumbnail_frame(
    ui: &mut egui::Ui,
    texture: Option<&TextureHandle>,
    image_size: Option<(u32, u32)>,
    label: &str,
) {
    let width = (CARD_WIDTH - 18.0).max(1.0);
    let (rect, _) = ui.allocate_exact_size(Vec2::new(width, THUMBNAIL_HEIGHT), Sense::hover());
    let fill = if texture.is_some() || image_size.is_some() {
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
    if let Some(texture) = texture {
        let image_rect = fit_rect_preserve_aspect(rect.shrink(4.0), texture.size_vec2());
        ui.painter().image(
            texture.id(),
            image_rect,
            Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0)),
            Color32::WHITE,
        );
        if let Some((width, height)) = image_size {
            paint_thumbnail_badge(ui, rect, &format!("{width} x {height}"));
        }
    } else {
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
}

fn paint_thumbnail_badge(ui: &egui::Ui, rect: Rect, text: &str) {
    let padding = Vec2::new(6.0, 3.0);
    let galley = ui.painter().layout_no_wrap(
        text.to_owned(),
        FontId::proportional(11.0),
        Color32::from_rgb(231, 235, 238),
    );
    let badge_rect = Rect::from_min_size(
        rect.left_bottom() + Vec2::new(6.0, -6.0 - galley.size().y - padding.y * 2.0),
        galley.size() + padding * 2.0,
    );
    ui.painter().rect_filled(
        badge_rect,
        CornerRadius::same(4),
        Color32::from_rgba_unmultiplied(18, 22, 25, 190),
    );
    ui.painter().galley(
        badge_rect.min + padding,
        galley,
        Color32::from_rgb(231, 235, 238),
    );
}

fn fit_rect_preserve_aspect(bounds: Rect, image_size: Vec2) -> Rect {
    if bounds.width() <= 0.0
        || bounds.height() <= 0.0
        || image_size.x <= 0.0
        || image_size.y <= 0.0
        || !bounds.width().is_finite()
        || !bounds.height().is_finite()
        || !image_size.x.is_finite()
        || !image_size.y.is_finite()
    {
        return bounds;
    }

    let bounds_aspect = bounds.width() / bounds.height();
    let image_aspect = image_size.x / image_size.y;
    let size = if bounds_aspect > image_aspect {
        Vec2::new(bounds.height() * image_aspect, bounds.height())
    } else {
        Vec2::new(bounds.width(), bounds.width() / image_aspect)
    };
    Rect::from_center_size(bounds.center(), size)
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

fn change_line(node_name: &str, key: &str, parameter: &str, before: f32, after: f32) -> String {
    let trend = value_trend(before, after);
    let rounder = if after > before { "rounder" } else { "sharper" };
    let blend = if after > before { "softer" } else { "crisper" };
    let before = format_scalar(before);
    let after_text = format_scalar(after);
    match key {
        "transform.translation.x" => {
            format!("{node_name} left/right position changes: {before} to {after_text}")
        }
        "transform.translation.y" => {
            format!("{node_name} height position changes: {before} to {after_text}")
        }
        "transform.translation.z" => {
            format!("{node_name} front/back position changes: {before} to {after_text}")
        }
        "transform.rotation_degrees.x" => {
            format!("{node_name} tilt changes: {before} to {after_text}")
        }
        "transform.rotation_degrees.y" => {
            format!("{node_name} turn changes: {before} to {after_text}")
        }
        "transform.rotation_degrees.z" => {
            format!("{node_name} spin changes: {before} to {after_text}")
        }
        "transform.scale.x" | "primitive.half_extents.x" => {
            format!("{node_name} width {trend}: {before} to {after_text}")
        }
        "transform.scale.y" | "primitive.half_extents.y" | "primitive.half_height" => {
            format!("{node_name} height {trend}: {before} to {after_text}")
        }
        "transform.scale.z" | "primitive.half_extents.z" => {
            format!("{node_name} depth {trend}: {before} to {after_text}")
        }
        "primitive.radius" => {
            format!("{node_name} overall size {trend}: {before} to {after_text}")
        }
        "primitive.half_length" => {
            format!("{node_name} length {trend}: {before} to {after_text}")
        }
        "primitive.roundness" => {
            format!("{node_name} edges get {rounder}: {before} to {after_text}")
        }
        "primitive.major_radius" => {
            format!("{node_name} ring width {trend}: {before} to {after_text}")
        }
        "primitive.minor_radius" => {
            format!("{node_name} ring thickness {trend}: {before} to {after_text}")
        }
        "csg.smoothness" => format!("{node_name} blend gets {blend}: {before} to {after_text}"),
        _ => format!("{node_name} {parameter}: {before} to {after_text}"),
    }
}

fn value_trend(before: f32, after: f32) -> &'static str {
    if after > before { "grows" } else { "shrinks" }
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
