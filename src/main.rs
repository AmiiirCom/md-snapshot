#![windows_subsystem = "windows"]

mod app;
mod config;
mod formatters;
mod scanner;
mod tree;
mod ui;

use app::MdSnapshotApp;
use eframe::egui;

/// لود کردن بایت‌های آیکون در زمان کامپایل و تبدیل به فرمت مورد نیاز eframe
fn load_app_icon() -> Option<egui::IconData> {
    let icon_bytes = include_bytes!("../icon.ico");
    let img = image::load_from_memory(icon_bytes).ok()?.into_rgba8();
    let (width, height) = img.dimensions();
    Some(egui::IconData {
        rgba: img.into_raw(),
        width,
        height,
    })
}

fn main() -> eframe::Result<()> {
    let mut viewport = egui::ViewportBuilder::default()
        .with_inner_size([1120.0, 740.0])
        .with_min_inner_size([880.0, 560.0])
        .with_title("md-snapshot — Codebase to LLM Context");

    // ست کردن آیکون اختصاصی روی نوار عنوان و تسک‌بار
    if let Some(icon) = load_app_icon() {
        viewport = viewport.with_icon(icon);
    }

    let native_options = eframe::NativeOptions {
        viewport,
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