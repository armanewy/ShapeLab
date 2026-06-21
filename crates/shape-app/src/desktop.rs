//! Top-level native app mode switcher.

use crate::app::ShapeLabApp;
use crate::asset::app::AssetModelingLabApp;

/// Native Shape Lab desktop app with Asset Modeling Lab as the startup mode.
pub(crate) struct ShapeLabDesktopApp {
    mode: NativeMode,
    asset: Option<AssetModelingLabApp>,
    legacy: Option<ShapeLabApp>,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
enum NativeMode {
    AssetModelingLab,
    LegacyImplicit,
}

impl Default for ShapeLabDesktopApp {
    fn default() -> Self {
        Self {
            mode: NativeMode::AssetModelingLab,
            asset: None,
            legacy: None,
        }
    }
}

impl ShapeLabDesktopApp {
    fn asset_app(&mut self) -> &mut AssetModelingLabApp {
        self.asset.get_or_insert_with(AssetModelingLabApp::default)
    }

    fn legacy_app(&mut self) -> &mut ShapeLabApp {
        self.legacy.get_or_insert_with(ShapeLabApp::default)
    }
}

impl eframe::App for ShapeLabDesktopApp {
    fn ui(&mut self, ui: &mut egui::Ui, frame: &mut eframe::Frame) {
        egui::Panel::top("shape_lab_mode_switcher").show_inside(ui, |ui| {
            ui.horizontal(|ui| {
                ui.selectable_value(
                    &mut self.mode,
                    NativeMode::AssetModelingLab,
                    "Asset Modeling Lab",
                );
                ui.selectable_value(
                    &mut self.mode,
                    NativeMode::LegacyImplicit,
                    "Legacy Implicit Mode",
                );
            });
        });
        match self.mode {
            NativeMode::AssetModelingLab => self.asset_app().ui(ui, frame),
            NativeMode::LegacyImplicit => self.legacy_app().ui(ui, frame),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn inactive_desktop_modes_are_lazy() {
        let mut app = ShapeLabDesktopApp::default();

        assert_eq!(app.mode, NativeMode::AssetModelingLab);
        assert!(app.asset.is_none());
        assert!(app.legacy.is_none());

        let _ = app.asset_app();
        assert!(app.asset.is_some());
        assert!(app.legacy.is_none());

        app.mode = NativeMode::LegacyImplicit;
        let _ = app.legacy_app();
        assert!(app.asset.is_some());
        assert!(app.legacy.is_some());
    }
}
