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
            .with_inner_size([1280.0, 800.0])
            .with_min_inner_size([900.0, 600.0]),
        ..Default::default()
    };
    eframe::run_native(
        "Shape Lab",
        options,
        Box::new(|_cc| Ok(Box::<FoundryDesktopApp>::default())),
    )
}
