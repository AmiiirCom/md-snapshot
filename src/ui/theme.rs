use egui::{Color32, Margin, Rounding, Style, Vec2, Visuals};

pub fn apply_modern_theme(ctx: &egui::Context) {
    let mut visuals = Visuals::dark();

    // رنگ‌بندی عمیق با کنتراست متعادل
    visuals.panel_fill = Color32::from_rgb(20, 21, 26);
    visuals.window_fill = Color32::from_rgb(26, 27, 34);

    // دکمه‌ها و عناصر در حالت عادی
    visuals.widgets.inactive.bg_fill = Color32::from_rgb(34, 36, 46);
    visuals.widgets.inactive.rounding = Rounding::same(6.0);
    visuals.widgets.inactive.fg_stroke.color = Color32::from_rgb(215, 220, 235);

    // حالت Hover
    visuals.widgets.hovered.bg_fill = Color32::from_rgb(46, 50, 66);
    visuals.widgets.hovered.rounding = Rounding::same(6.0);
    visuals.widgets.hovered.fg_stroke.color = Color32::from_rgb(255, 255, 255);

    // حالت Active (کلیک‌شده)
    visuals.widgets.active.bg_fill = Color32::from_rgb(88, 101, 242);
    visuals.widgets.active.rounding = Rounding::same(6.0);

    // استایل تب‌ها و چک‌باکس
    visuals.selection.bg_fill = Color32::from_rgb(88, 101, 242);
    visuals.selection.stroke.color = Color32::WHITE;

    ctx.set_visuals(visuals);

    let mut style: Style = (*ctx.style()).clone();
    style.spacing.item_spacing = Vec2::new(8.0, 8.0);
    style.spacing.button_padding = Vec2::new(10.0, 6.0);
    style.spacing.window_margin = Margin::same(12.0);
    ctx.set_style(style);
}