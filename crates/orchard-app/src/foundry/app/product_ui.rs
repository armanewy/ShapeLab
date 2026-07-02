use super::*;

pub(super) fn button_with_disabled_reason(
    ui: &mut egui::Ui,
    enabled: bool,
    label: &str,
    disabled_reason: &str,
) -> egui::Response {
    let spec = if enabled {
        ActionSpec::enabled(label, ButtonTone::Secondary)
    } else {
        ActionSpec::disabled(label, ButtonTone::Secondary, disabled_reason)
    };
    action_button(ui, &spec)
}

pub(super) fn action_spec<'a>(
    enabled: bool,
    label: &'a str,
    tone: ButtonTone,
    disabled_reason: &'a str,
) -> ActionSpec<'a> {
    if enabled {
        ActionSpec::enabled(label, tone)
    } else {
        ActionSpec::disabled(label, tone, disabled_reason)
    }
}

pub(super) fn product_control_summary(
    control: &crate::foundry::view_model::FoundryControlView,
) -> &'static str {
    if control.numeric_range.is_some() {
        numeric_control_summary(control)
    } else if control.options.len() > 1 {
        "Whole-model options"
    } else {
        "Primary control"
    }
}

pub(super) fn numeric_control_summary(
    control: &crate::foundry::view_model::FoundryControlView,
) -> &'static str {
    match control.label.as_str() {
        "Width" | "Panel Width" | "Knob Width" => "Controls width.",
        "Depth" | "Knob Depth" => "Controls depth.",
        "Height" | "Panel Height" | "Knob Height" => "Controls height.",
        "Thickness" | "Panel Thickness" => "Controls thickness.",
        "Edge Softness" | "Panel Edge Softness" => "Controls corner softness.",
        "Front Flatten" | "Knob Front Flatten" => "Controls front flattening.",
        "Back Flatten" | "Knob Back Flatten" => "Controls back flattening.",
        "Knob Horizontal Position" | "Knob Vertical Position" => {
            "Keeps knob position within the safe anchor area."
        }
        "Lid Seam" | "Hinge Edge" => "Keep within authored safe range.",
        _ => "Keep within authored safe range.",
    }
}

pub(super) fn product_card<R>(
    ui: &mut egui::Ui,
    selected: bool,
    add_contents: impl FnOnce(&mut egui::Ui) -> R,
) -> egui::InnerResponse<R> {
    let tokens = VisualFoundryTokens::dark();
    let stroke_color = if selected {
        tokens.colors.accent_hover
    } else {
        tokens.colors.stroke
    };
    egui::Frame::new()
        .fill(if selected {
            tokens.colors.accent_soft
        } else {
            tokens.colors.panel
        })
        .stroke(egui::Stroke::new(1.0, stroke_color))
        .corner_radius(egui::CornerRadius::same(tokens.radius.lg as u8))
        .inner_margin(egui::Margin::symmetric(12, 10))
        .show(ui, add_contents)
}

pub(super) fn product_stage<R>(
    ui: &mut egui::Ui,
    add_contents: impl FnOnce(&mut egui::Ui) -> R,
) -> egui::InnerResponse<R> {
    let tokens = VisualFoundryTokens::dark();
    egui::Frame::new()
        .fill(tokens.colors.center_bg)
        .stroke(egui::Stroke::new(1.0, tokens.colors.stroke))
        .corner_radius(egui::CornerRadius::same(tokens.radius.lg as u8))
        .inner_margin(egui::Margin::symmetric(18, 16))
        .show(ui, |ui| ui.vertical_centered(|ui| add_contents(ui)).inner)
}

pub(super) fn draw_busy_overlay(ui: &egui::Ui, rect: egui::Rect, label: &str) {
    let colors = VisualFoundryTokens::dark().colors;
    let overlay = rect.shrink2(egui::vec2(18.0, 48.0));
    ui.painter().rect_filled(
        overlay,
        egui::CornerRadius::same(10),
        egui::Color32::from_rgba_premultiplied(8, 13, 18, 190),
    );
    ui.painter().rect_stroke(
        overlay,
        egui::CornerRadius::same(10),
        egui::Stroke::new(1.0, colors.accent_hover),
        egui::StrokeKind::Inside,
    );
    ui.painter().text(
        overlay.center(),
        egui::Align2::CENTER_CENTER,
        label,
        egui::FontId::proportional(18.0),
        colors.text,
    );
}

pub(super) fn show_make_view_controls(
    ui: &mut egui::Ui,
    current_preview: Option<&FoundryPreviewImage>,
    current_preview_orbit: &mut CurrentPreviewOrbitState,
) -> Vec<FoundryAppCommand> {
    let mut commands = Vec::new();
    product_card(ui, false, |ui| {
        ui.horizontal_wrapped(|ui| {
            let _ = status_pill(
                ui,
                StatusPillSpec::new(VIEW_ORBIT_LABEL, StatusTone::Neutral),
            );
            if action_button(
                ui,
                &action_spec(
                    current_preview.is_some(),
                    VIEW_RESET_LABEL,
                    ButtonTone::Secondary,
                    PREVIEW_PREPARING_REASON,
                ),
            )
            .clicked()
                && let Some(command) = reset_current_preview_view_command(current_preview)
            {
                *current_preview_orbit = CurrentPreviewOrbitState::default();
                commands.push(command);
            }
            let _ = status_pill(
                ui,
                StatusPillSpec::new(VIEW_AXIS_LABEL, make_view_axis_default_tone()),
            );
        });
    });
    commands
}

pub(super) fn make_view_axis_default_tone() -> StatusTone {
    StatusTone::Neutral
}

pub(super) fn reset_current_preview_view_command(
    current_preview: Option<&FoundryPreviewImage>,
) -> Option<FoundryAppCommand> {
    let preview = current_preview?;
    Some(FoundryAppCommand::RequestPreview {
        width: preview.width,
        height: preview.height,
        camera: Some(OrbitCamera::default()),
    })
}

pub(super) fn draw_focus_callout(ui: &egui::Ui, rect: egui::Rect, label: &str) {
    let colors = VisualFoundryTokens::dark().colors;
    let focus_rect = focus_callout_rect(rect, label);
    ui.painter().rect_stroke(
        focus_rect,
        egui::CornerRadius::same(10),
        egui::Stroke::new(2.0, colors.accent_hover),
        egui::StrokeKind::Inside,
    );
    let tag = egui::Rect::from_min_size(
        egui::pos2(focus_rect.left(), focus_rect.top() - 30.0),
        egui::vec2((label.len() as f32 * 8.0).clamp(74.0, 140.0), 24.0),
    );
    ui.painter()
        .rect_filled(tag, egui::CornerRadius::same(8), colors.accent_soft);
    ui.painter().rect_stroke(
        tag,
        egui::CornerRadius::same(8),
        egui::Stroke::new(1.0, colors.accent_hover),
        egui::StrokeKind::Inside,
    );
    ui.painter().text(
        tag.center(),
        egui::Align2::CENTER_CENTER,
        label,
        egui::FontId::proportional(13.0),
        colors.text,
    );
}

pub(super) fn focus_callout_rect(rect: egui::Rect, label: &str) -> egui::Rect {
    let center = rect.center();
    let width = (rect.width() * 0.24).clamp(96.0, 190.0);
    let height = (rect.height() * 0.18).clamp(68.0, 130.0);
    let lower = label.to_ascii_lowercase();
    let offset = if lower.contains("handle") {
        egui::vec2(rect.width() * 0.20, rect.height() * 0.06)
    } else if lower.contains("vent") {
        egui::vec2(rect.width() * 0.04, -rect.height() * 0.15)
    } else if lower.contains("body") {
        egui::vec2(0.0, 0.0)
    } else {
        egui::vec2(-rect.width() * 0.10, -rect.height() * 0.02)
    };
    egui::Rect::from_center_size(center + offset, egui::vec2(width, height))
}

pub(super) fn workflow_tab_button(
    ui: &mut egui::Ui,
    index: usize,
    label: &str,
    detail: &str,
    selected: bool,
) -> egui::Response {
    let tokens = VisualFoundryTokens::dark();
    let colors = tokens.colors;
    let fill = if selected {
        colors.accent_soft
    } else {
        colors.panel
    };
    let stroke = if selected {
        egui::Stroke::new(1.0, colors.accent_hover)
    } else {
        egui::Stroke::new(1.0, colors.stroke)
    };
    let text = if selected {
        colors.text
    } else {
        colors.text_muted
    };
    let title = format!("{index}. {label}");
    let response = egui::Frame::new()
        .fill(fill)
        .stroke(stroke)
        .corner_radius(egui::CornerRadius::same(8))
        .inner_margin(egui::Margin::symmetric(14, 8))
        .show(ui, |ui| {
            ui.set_min_width(124.0);
            ui.vertical(|ui| {
                ui.label(RichText::new(title).color(text).strong());
                ui.label(RichText::new(detail).color(colors.text_subtle).small());
            });
        })
        .response
        .interact(egui::Sense::click());
    response.on_hover_text(detail)
}

pub(super) fn variation_mode_button(
    ui: &mut egui::Ui,
    label: &str,
    selected: bool,
    enabled: bool,
    disabled_reason: &str,
) -> egui::Response {
    let tokens = VisualFoundryTokens::dark();
    let colors = tokens.colors;
    let fill = if selected {
        colors.accent_soft
    } else if enabled {
        colors.panel_elevated
    } else {
        colors.panel_subtle
    };
    let stroke = if selected {
        egui::Stroke::new(1.0, colors.accent_hover)
    } else {
        egui::Stroke::new(1.0, colors.stroke)
    };
    let text = if enabled {
        colors.text
    } else {
        colors.text_subtle
    };
    let response = ui.add_enabled(
        enabled,
        egui::Button::new(RichText::new(label).color(text).strong())
            .fill(fill)
            .stroke(stroke)
            .corner_radius(egui::CornerRadius::same(8))
            .min_size(egui::vec2(118.0, 36.0)),
    );
    if enabled {
        response
    } else {
        response.on_disabled_hover_text(disabled_reason)
    }
}

pub(super) fn product_empty_state(ui: &mut egui::Ui, title: &str, message: &str) {
    ui.allocate_ui_with_layout(
        egui::vec2(ui.available_width(), 190.0),
        egui::Layout::top_down(egui::Align::Min),
        |ui| {
            product_card(ui, false, |ui| {
                let colors = VisualFoundryTokens::dark().colors;
                ui.set_min_height(150.0);
                ui.with_layout(egui::Layout::top_down(egui::Align::Center), |ui| {
                    ui.add_space(28.0);
                    ui.label(RichText::new(title).color(colors.text).size(17.0).strong());
                    ui.add(
                        egui::Label::new(RichText::new(message).color(colors.text_muted)).wrap(),
                    );
                });
            });
        },
    );
}

pub(super) fn product_compact_empty_state(ui: &mut egui::Ui, title: &str, message: &str) {
    product_card(ui, false, |ui| {
        let colors = VisualFoundryTokens::dark().colors;
        ui.set_min_height(64.0);
        ui.horizontal_wrapped(|ui| {
            ui.label(RichText::new(title).color(colors.text).strong());
            ui.add_space(8.0);
            ui.add(
                egui::Label::new(RichText::new(message).color(colors.text_muted).small()).wrap(),
            );
        });
    });
}

pub(super) fn compact_section_header(
    ui: &mut egui::Ui,
    eyebrow: &str,
    title: &str,
    subtitle: &str,
) {
    let colors = VisualFoundryTokens::dark().colors;
    ui.horizontal_wrapped(|ui| {
        ui.label(
            RichText::new(eyebrow.to_ascii_uppercase())
                .color(colors.accent_hover)
                .small(),
        );
        ui.label(RichText::new(title).color(colors.text).strong());
        ui.label(RichText::new(subtitle).color(colors.text_muted).small());
    });
}
