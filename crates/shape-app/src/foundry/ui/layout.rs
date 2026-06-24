//! Layout constants for the native Visual Foundry shell.

use super::tokens::VisualFoundrySizing;

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct VisualFoundryLayout {
    pub app_width: f32,
    pub app_height: f32,
    pub left_rail_width: f32,
    pub right_panel_width: f32,
    pub top_bar_height: f32,
    pub status_bar_height: f32,
    pub center_width: f32,
    pub center_height: f32,
}

impl VisualFoundryLayout {
    #[must_use]
    pub(crate) fn for_viewport(width: f32, height: f32) -> Self {
        let sizing = VisualFoundrySizing::default();
        let min_center_width = 520.0;
        let right_panel_width =
            if width >= sizing.left_rail_width + sizing.right_panel_width + min_center_width {
                sizing.right_panel_width
            } else {
                0.0
            };
        let center_width = (width - sizing.left_rail_width - right_panel_width).max(0.0);
        let center_height = (height - sizing.top_bar_height - sizing.status_bar_height).max(0.0);
        Self {
            app_width: width,
            app_height: height,
            left_rail_width: sizing.left_rail_width,
            right_panel_width,
            top_bar_height: sizing.top_bar_height,
            status_bar_height: sizing.status_bar_height,
            center_width,
            center_height,
        }
    }

    #[must_use]
    pub(crate) fn supports_three_column_shell(self) -> bool {
        self.right_panel_width > 0.0 && self.center_width >= 520.0 && self.center_height >= 500.0
    }

    #[must_use]
    pub(crate) fn dominant_preview_height(self) -> f32 {
        let target = self.center_height * 0.52;
        target.clamp(300.0, 460.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reference_viewport_supports_three_column_product_shell() {
        let layout = VisualFoundryLayout::for_viewport(1280.0, 800.0);
        assert!(layout.supports_three_column_shell());
        assert!(layout.dominant_preview_height() >= 360.0);
    }

    #[test]
    fn compact_viewport_keeps_center_usable_by_hiding_context_panel() {
        let layout = VisualFoundryLayout::for_viewport(900.0, 600.0);
        assert_eq!(layout.right_panel_width, 0.0);
        assert!(layout.center_width >= 660.0);
        assert!(layout.center_height >= 500.0);
    }

    #[test]
    fn context_panel_waits_until_center_can_hold_preview() {
        let layout = VisualFoundryLayout::for_viewport(1100.0, 700.0);
        assert_eq!(layout.right_panel_width, 0.0);
        assert!(!layout.supports_three_column_shell());
    }
}
