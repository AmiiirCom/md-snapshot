#![windows_subsystem = "windows"]

mod app;
mod config;
mod formatters;
mod scanner;
mod tree;
mod ui;

use app::MdSnapshotApp;
use eframe::egui;

fn main() -> eframe::Result<()> {
    let native_options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1120.0, 740.0])
            .with_min_inner_size([880.0, 560.0])
            .with_title("md-snapshot — Codebase to LLM Context"),
        ..Default::default()
    };

    eframe::run_native(
        "md-snapshot",
        native_options,
        Box::new(|cc| {
            ui::theme::apply_modern_theme(&cc.egui_ctx);
            Ok(Box::new(MdSnapshotApp::new(cc)))
        }),
    )
}