#![forbid(unsafe_code)]

mod app;
mod asset;
mod commands;
mod desktop;
mod foundry;
mod jobs;
mod panels;
mod state;
mod viewport;

use desktop::ShapeLabDesktopApp;

fn main() -> eframe::Result<()> {
    let _ = env_logger::try_init();
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1280.0, 800.0])
            .with_min_inner_size([900.0, 600.0]),
        ..Default::default()
    };
    eframe::run_native(
        "Shape Lab",
        options,
        Box::new(|_cc| Ok(Box::<ShapeLabDesktopApp>::default())),
    )
}
