//! Top-level native app mode switcher.

use crate::app::ShapeLabApp;
use crate::asset::app::AssetModelingLabApp;

/// Native Shape Lab desktop app with Asset Modeling Lab as the startup mode.
pub(crate) struct ShapeLabDesktopApp {
    mode: NativeMode,
    asset: AssetModelingLabApp,
    legacy: ShapeLabApp,
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
            asset: AssetModelingLabApp::default(),
            legacy: ShapeLabApp::default(),
        }
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
            NativeMode::AssetModelingLab => self.asset.ui(ui, frame),
            NativeMode::LegacyImplicit => self.legacy.ui(ui, frame),
        }
    }
}
