//! Visual Foundry theme tokens.

use egui::Color32;

pub(crate) const MIN_BODY_TEXT_CONTRAST: f32 = 4.5;
pub(crate) const MIN_LARGE_TEXT_CONTRAST: f32 = 3.0;

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct VisualFoundryTokens {
    pub colors: VisualFoundryColors,
    pub spacing: VisualFoundrySpacing,
    pub radius: VisualFoundryRadius,
    pub sizing: VisualFoundrySizing,
}

impl Default for VisualFoundryTokens {
    fn default() -> Self {
        Self::dark()
    }
}

impl VisualFoundryTokens {
    #[must_use]
    pub(crate) fn dark() -> Self {
        Self {
            colors: VisualFoundryColors::dark(),
            spacing: VisualFoundrySpacing::default(),
            radius: VisualFoundryRadius::default(),
            sizing: VisualFoundrySizing::default(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct VisualFoundryColors {
    pub app_bg: Color32,
    pub top_bar: Color32,
    pub left_rail: Color32,
    pub center_bg: Color32,
    pub panel: Color32,
    pub panel_elevated: Color32,
    pub panel_subtle: Color32,
    pub stroke: Color32,
    pub stroke_strong: Color32,
    pub text: Color32,
    pub text_muted: Color32,
    pub text_subtle: Color32,
    pub accent: Color32,
    pub accent_hover: Color32,
    pub accent_soft: Color32,
    pub success: Color32,
    pub warning: Color32,
    pub danger: Color32,
    pub disabled_fill: Color32,
}

impl VisualFoundryColors {
    #[must_use]
    pub(crate) const fn dark() -> Self {
        Self {
            app_bg: Color32::from_rgb(7, 11, 15),
            top_bar: Color32::from_rgb(9, 14, 20),
            left_rail: Color32::from_rgb(10, 16, 23),
            center_bg: Color32::from_rgb(8, 13, 18),
            panel: Color32::from_rgb(15, 23, 31),
            panel_elevated: Color32::from_rgb(20, 31, 42),
            panel_subtle: Color32::from_rgb(12, 19, 27),
            stroke: Color32::from_rgb(46, 60, 75),
            stroke_strong: Color32::from_rgb(74, 95, 117),
            text: Color32::from_rgb(238, 244, 250),
            text_muted: Color32::from_rgb(178, 188, 200),
            text_subtle: Color32::from_rgb(133, 146, 161),
            accent: Color32::from_rgb(13, 109, 223),
            accent_hover: Color32::from_rgb(77, 164, 255),
            accent_soft: Color32::from_rgb(21, 65, 112),
            success: Color32::from_rgb(99, 211, 127),
            warning: Color32::from_rgb(238, 184, 82),
            danger: Color32::from_rgb(181, 63, 60),
            disabled_fill: Color32::from_rgb(31, 39, 49),
        }
    }

    #[must_use]
    pub(crate) const fn contrast_pairs(&self) -> [ColorPair; 7] {
        [
            ColorPair::new(self.text, self.app_bg, "text on app background"),
            ColorPair::new(self.text, self.panel, "text on panel"),
            ColorPair::new(self.text_muted, self.panel, "muted text on panel"),
            ColorPair::new(self.text, self.accent_soft, "text on selected panel"),
            ColorPair::new(self.text, self.disabled_fill, "text on disabled fill"),
            ColorPair::new(self.success, self.panel, "success text on panel"),
            ColorPair::new(self.warning, self.panel, "warning text on panel"),
        ]
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct VisualFoundrySpacing {
    pub xs: f32,
    pub sm: f32,
    pub md: f32,
    pub lg: f32,
    pub xl: f32,
    pub section_gap: f32,
}

impl Default for VisualFoundrySpacing {
    fn default() -> Self {
        Self {
            xs: 4.0,
            sm: 8.0,
            md: 12.0,
            lg: 16.0,
            xl: 24.0,
            section_gap: 20.0,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct VisualFoundryRadius {
    pub sm: f32,
    pub md: f32,
    pub lg: f32,
}

impl Default for VisualFoundryRadius {
    fn default() -> Self {
        Self {
            sm: 4.0,
            md: 6.0,
            lg: 8.0,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct VisualFoundrySizing {
    pub top_bar_height: f32,
    pub status_bar_height: f32,
    pub left_rail_width: f32,
    pub right_panel_width: f32,
    pub min_preview_height: f32,
    pub direction_card_height: f32,
    pub option_tile_height: f32,
    pub icon_button: f32,
}

impl Default for VisualFoundrySizing {
    fn default() -> Self {
        Self {
            top_bar_height: 52.0,
            status_bar_height: 32.0,
            left_rail_width: 222.0,
            right_panel_width: 392.0,
            min_preview_height: 360.0,
            direction_card_height: 158.0,
            option_tile_height: 84.0,
            icon_button: 32.0,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct ColorPair {
    pub foreground: Color32,
    pub background: Color32,
    pub label: &'static str,
}

impl ColorPair {
    #[must_use]
    pub(crate) const fn new(foreground: Color32, background: Color32, label: &'static str) -> Self {
        Self {
            foreground,
            background,
            label,
        }
    }

    #[must_use]
    pub(crate) fn contrast_ratio(self) -> f32 {
        contrast_ratio(self.foreground, self.background)
    }
}

#[must_use]
pub(crate) fn contrast_ratio(foreground: Color32, background: Color32) -> f32 {
    let fg = relative_luminance(foreground);
    let bg = relative_luminance(background);
    let (lighter, darker) = if fg >= bg { (fg, bg) } else { (bg, fg) };
    (lighter + 0.05) / (darker + 0.05)
}

#[must_use]
pub(crate) fn relative_luminance(color: Color32) -> f32 {
    fn channel(value: u8) -> f32 {
        let srgb = f32::from(value) / 255.0;
        if srgb <= 0.039_28 {
            srgb / 12.92
        } else {
            ((srgb + 0.055) / 1.055).powf(2.4)
        }
    }

    0.2126 * channel(color.r()) + 0.7152 * channel(color.g()) + 0.0722 * channel(color.b())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dark_theme_text_pairs_meet_body_contrast() {
        let colors = VisualFoundryColors::dark();
        for pair in colors.contrast_pairs() {
            assert!(
                pair.contrast_ratio() >= MIN_BODY_TEXT_CONTRAST,
                "{} contrast was {:.2}",
                pair.label,
                pair.contrast_ratio()
            );
        }
    }

    #[test]
    fn layout_tokens_match_reference_shell_proportions() {
        let sizing = VisualFoundrySizing::default();
        assert_eq!(sizing.left_rail_width, 222.0);
        assert!(sizing.right_panel_width >= 360.0);
        assert!(sizing.min_preview_height >= 340.0);
        assert!(sizing.direction_card_height >= 148.0);
    }
}
