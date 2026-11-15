use crate::tree::{CheckState, FileTreeNode};
use egui::{Color32, RichText, Ui};

pub fn render_file_tree_ui(ui: &mut Ui, node: &mut FileTreeNode, search_query: &str) -> bool {
    let mut changed = false;

    let matches_search = search_query.is_empty()
        || node
            .name
            .to_lowercase()
            .contains(&search_query.to_lowercase());

    if !matches_search && !node.is_dir {
        return false;
    }

    ui.horizontal(|ui| {
        // آیکون تیک متناسب با وضعیت گره
        let (icon, color) = match node.check_state {
            CheckState::Checked => ("☑", Color32::from_rgb(166, 227, 161)),
            CheckState::Unchecked => ("☐", Color32::from_rgb(140, 145, 160)),
            CheckState::Indeterminate => ("▣", Color32::from_rgb(250, 179, 135)),
        };

        if ui.button(RichText::new(icon).size(13.0).color(color)).clicked() {
            let next_state = node.check_state != CheckState::Checked;
            node.set_check_recursive(next_state);
            changed = true;
        }

        if node.is_dir {
            let arrow = if node.is_expanded { "▼" } else { "▶" };
            let folder_icon = if node.is_expanded { "📂" } else { "📁" };
            let label = format!("{} {} {}", arrow, folder_icon, node.name);
            if ui.selectable_label(node.is_expanded, label).clicked() {
                node.is_expanded = !node.is_expanded;
            }
        } else {
            let icon = detect_file_icon(&node.name);
            let size_kb = (node.size_bytes as f64) / 1024.0;
            let text = format!("{} {} ({:.1} KB)", icon, node.name, size_kb);
            ui.label(RichText::new(text).color(Color32::from_rgb(205, 214, 244)));
        }
    });

    if node.is_dir && node.is_expanded {
        ui.indent(&node.name, |ui| {
            for child in &mut node.children {
                if render_file_tree_ui(ui, child, search_query) {
                    changed = true;
                }
            }
        });
    }

    node.update_state_from_children();
    changed
}

fn detect_file_icon(name: &str) -> &'static str {
    if name.starts_with(".env") {
        return "🔒";
    }
    match name.split('.').last().unwrap_or("") {
        "rs" => "🦀",
        "ts" | "tsx" | "js" | "jsx" => "⚡",
        "py" => "🐍",
        "go" => "🐹",
        "json" | "toml" | "yaml" | "yml" => "⚙",
        "md" => "📝",
        _ => "📄",
    }
}