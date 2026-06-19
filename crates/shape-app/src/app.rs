//! Application shell.

/// Bootstrap application used by Wave 0.
#[derive(Debug, Default)]
pub(crate) struct ShapeLabApp;

impl eframe::App for ShapeLabApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        egui::Frame::central_panel(ui.style()).show(ui, |ui| {
            ui.heading("Shape Lab bootstrap");
        });
    }
}
