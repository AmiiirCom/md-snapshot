use egui::{Color32, RichText, Rounding, Ui};
use std::collections::HashSet;

pub fn render_chip_input(
    ui: &mut Ui,
    items: &mut HashSet<String>,
    input_buffer: &mut String,
    placeholder: &str,
) {
    let mut sorted_items: Vec<String> = items.iter().cloned().collect();
    sorted_items.sort();

    let mut to_remove = None;

    // ۱. نمایش تگ‌های مرتب‌شده و بدون پرش
    ui.horizontal_wrapped(|ui| {
        for item in &sorted_items {
            let text = format!("{} ✕", item);
            let btn = egui::Button::new(
                RichText::new(text)
                    .size(11.5)
                    .color(Color32::from_rgb(225, 230, 245)),
            )
            .fill(Color32::from_rgb(40, 44, 58))
            .rounding(Rounding::same(4.0));

            if ui.add(btn).clicked() {
                to_remove = Some(item.clone());
            }
        }
    });

    if let Some(rem) = to_remove {
        items.remove(&rem);
    }

    // ۲. فیلد ورود متن در یک خط اختصاصی و تمیز
    ui.horizontal(|ui| {
        let text_edit = egui::TextEdit::singleline(input_buffer)
            .hint_text(placeholder)
            .desired_width(100.0);

        let res = ui.add(text_edit);
        let enter = ui.input(|i| i.key_pressed(egui::Key::Enter));

        if (res.lost_focus() && enter) || ui.button("+ Add").clicked() {
            let clean = input_buffer.trim().trim_start_matches('.').to_string();
            if !clean.is_empty() {
                items.insert(clean);
                input_buffer.clear();
            }
        }
    });
}