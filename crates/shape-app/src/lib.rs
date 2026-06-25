#![forbid(unsafe_code)]

mod foundry;
mod product_gate;

use foundry::app::FoundryDesktopApp;

pub use product_gate::{
    ProductUiForbiddenTermFinding, ProductUiGateReport, ProductUiProfileGate,
    visual_foundry_product_ui_gate_report,
};

pub fn run_native_app() -> eframe::Result<()> {
    let _ = env_logger::try_init();
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_min_inner_size([1180.0, 720.0])
            .with_decorations(true)
            .with_fullscreen(false)
            .with_maximized(true),
        ..Default::default()
    };
    eframe::run_native(
        "Shape Lab",
        options,
        Box::new(|_cc| Ok(Box::<FoundryDesktopApp>::default())),
    )
}
