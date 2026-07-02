#![forbid(unsafe_code)]
#![cfg_attr(target_os = "windows", windows_subsystem = "windows")]

fn main() -> eframe::Result<()> {
    orchard_app::run_native_app()
}
