//! egui styling for the Visual Foundry native app.

use egui::{FontId, TextStyle};

use super::tokens::VisualFoundryTokens;

/// Apply the Visual Foundry style to an egui context.
pub(crate) fn apply_visual_foundry_theme(ctx: &egui::Context) {
    let tokens = VisualFoundryTokens::dark();
    let mut style = (*ctx.global_style()).clone();

    style.spacing.item_spacing = egui::vec2(tokens.spacing.sm, tokens.spacing.sm);
    style.spacing.button_padding = egui::vec2(tokens.spacing.md, tokens.spacing.sm);
    style.spacing.indent = tokens.spacing.lg;
    style.spacing.slider_width = 220.0;
    style.spacing.combo_width = 148.0;

    style.text_styles.insert(
        TextStyle::Heading,
        FontId::new(22.0, egui::FontFamily::Proportional),
    );
    style.text_styles.insert(
        TextStyle::Body,
        FontId::new(14.0, egui::FontFamily::Proportional),
    );
    style.text_styles.insert(
        TextStyle::Button,
        FontId::new(14.0, egui::FontFamily::Proportional),
    );
    style.text_styles.insert(
        TextStyle::Small,
        FontId::new(12.0, egui::FontFamily::Proportional),
    );
    style.text_styles.insert(
        TextStyle::Name("foundry.title".into()),
        FontId::new(26.0, egui::FontFamily::Proportional),
    );
    style.text_styles.insert(
        TextStyle::Name("foundry.section".into()),
        FontId::new(13.0, egui::FontFamily::Proportional),
    );

    let colors = tokens.colors;
    style.visuals = egui::Visuals::dark();
    style.visuals.override_text_color = Some(colors.text);
    style.visuals.panel_fill = colors.app_bg;
    style.visuals.window_fill = colors.panel;
    style.visuals.extreme_bg_color = colors.center_bg;
    style.visuals.faint_bg_color = colors.panel_subtle;
    style.visuals.hyperlink_color = colors.accent_hover;
    style.visuals.warn_fg_color = colors.warning;
    style.visuals.error_fg_color = colors.danger;
    style.visuals.selection.bg_fill = colors.accent_soft;
    style.visuals.selection.stroke = egui::Stroke::new(1.0, colors.text);
    style.visuals.widgets.noninteractive.bg_fill = colors.panel;
    style.visuals.widgets.noninteractive.bg_stroke = egui::Stroke::new(1.0, colors.stroke);
    style.visuals.widgets.inactive.bg_fill = colors.panel_elevated;
    style.visuals.widgets.inactive.bg_stroke = egui::Stroke::new(1.0, colors.stroke);
    style.visuals.widgets.hovered.bg_fill = colors.accent_soft;
    style.visuals.widgets.hovered.bg_stroke = egui::Stroke::new(1.0, colors.accent_hover);
    style.visuals.widgets.active.bg_fill = colors.accent;
    style.visuals.widgets.active.bg_stroke = egui::Stroke::new(1.0, colors.accent_hover);
    style.visuals.widgets.open.bg_fill = colors.panel_elevated;
    style.visuals.widgets.open.bg_stroke = egui::Stroke::new(1.0, colors.stroke_strong);

    ctx.set_global_style(style);
}

#[cfg(test)]
mod tests {
    use crate::foundry::ui::tokens::{MIN_BODY_TEXT_CONTRAST, contrast_ratio};

    use super::*;

    #[test]
    fn visual_foundry_theme_registers_named_text_styles() {
        let ctx = egui::Context::default();
        apply_visual_foundry_theme(&ctx);
        let style = ctx.global_style();
        assert!(
            style
                .text_styles
                .contains_key(&TextStyle::Name("foundry.title".into()))
        );
        assert!(
            style
                .text_styles
                .contains_key(&TextStyle::Name("foundry.section".into()))
        );
        assert_eq!(
            style.visuals.selection.stroke.color,
            VisualFoundryTokens::dark().colors.text
        );
        assert!(
            contrast_ratio(
                style.visuals.selection.stroke.color,
                style.visuals.selection.bg_fill
            ) >= MIN_BODY_TEXT_CONTRAST
        );
    }
}
