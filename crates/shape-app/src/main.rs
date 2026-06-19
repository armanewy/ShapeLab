#![forbid(unsafe_code)]

mod app;
mod commands;
mod jobs;
mod panels;
mod state;
mod viewport;

use app::ShapeLabApp;

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
        Box::new(|_cc| Ok(Box::<ShapeLabApp>::default())),
    )
}
