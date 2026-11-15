use crate::config::{AppConfig, OutputFormat, Preset};
use crate::formatters::format_snapshot;
use crate::scanner::{execute_scan, ScanResult};
use crate::tree::{build_ui_tree, FileTreeNode};
use crate::ui::chips::render_chip_input;
use crate::ui::file_tree::render_file_tree_ui;
use egui::{Color32, RichText, ScrollArea, Vec2};
use std::collections::HashSet;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{channel, Receiver, Sender};
use std::sync::Arc;
use std::time::Instant;

pub struct MdSnapshotApp {
    root_path: Option<PathBuf>,
    instruction_docs: Vec<PathBuf>,
    config: AppConfig,
    custom_tree: Option<FileTreeNode>,
    scan_rx: Option<Receiver<ScanResult>>,
    scan_tx: Sender<ScanResult>,
    is_scanning: bool,
    cancel_flag: Arc<AtomicBool>,
    result: Option<ScanResult>,
    status_msg: String,
    copied_toast_time: Option<Instant>,
    active_tab: ViewTab,
}

#[derive(PartialEq, Eq)]
enum ViewTab {
    Preview,
    AsciiTree,
    Omitted,
}

impl MdSnapshotApp {
    pub fn new(_cc: &eframe::CreationContext<'_>) -> Self {
        let (tx, rx) = channel();
        Self {
            root_path: None,
            instruction_docs: Vec::new(),
            config: AppConfig::default(),
            custom_tree: None,
            scan_rx: Some(rx),
            scan_tx: tx,
            is_scanning: false,
            cancel_flag: Arc::new(AtomicBool::new(false)),
            result: None,
            status_msg: "Ready. Select a project folder or Drag & Drop here.".to_string(),
            copied_toast_time: None,
            active_tab: ViewTab::Preview,
        }
    }

    fn trigger_scan(&mut self) {
        let root = match &self.root_path {
            Some(r) => r.clone(),
            None => return,
        };

        self.cancel_flag.store(true, Ordering::Relaxed);
        self.cancel_flag = Arc::new(AtomicBool::new(false));
        self.is_scanning = true;
        self.status_msg = "Scanning files and calculating tokens...".to_string();

        let cancel = self.cancel_flag.clone();
        let config = self.config.clone();
        let tx = self.scan_tx.clone();
        let docs = self.instruction_docs.clone();

        // در حالت Custom، فقط و فقط فایل‌های تیک‌خورده ارسال می‌شوند
        let selected_files = if config.preset == Preset::Custom {
            let mut set = HashSet::new();
            if let Some(ref tree) = self.custom_tree {
                tree.collect_selected_file_paths(&mut set);
            }
            Some(set)
        } else {
            None
        };

        std::thread::spawn(move || {
            if let Some(res) = execute_scan(root, config, selected_files, docs, cancel) {
                let _ = tx.send(res);
            }
        });
    }

    fn rebuild_custom_tree(&mut self) {
        if let Some(ref root) = self.root_path {
            self.custom_tree = build_ui_tree(root, &self.config);
        }
    }
}

impl eframe::App for MdSnapshotApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        ctx.input(|i| {
            if let Some(file) = i.raw.dropped_files.first() {
                if let Some(path) = &file.path {
                    if path.is_dir() {
                        self.root_path = Some(path.clone());
                        self.rebuild_custom_tree();
                        self.trigger_scan();
                    } else if path.is_file() {
                        self.instruction_docs.push(path.clone());
                        self.trigger_scan();
                    }
                }
            }
        });

        if let Some(ref rx) = self.scan_rx {
            if let Ok(res) = rx.try_recv() {
                self.is_scanning = false;
                self.status_msg = format!("Completed in {}ms!", res.elapsed_ms);
                self.result = Some(res);
            }
        }

        // Top Panel
        egui::TopBottomPanel::top("top_header").show(ctx, |ui| {
            ui.add_space(4.0);
            ui.horizontal(|ui| {
                ui.heading(RichText::new("📦 md-snapshot").strong().color(Color32::from_rgb(114, 135, 253)));
                ui.separator();

                if ui.button("📁 Open Folder").clicked() {
                    if let Some(folder) = rfd::FileDialog::new().pick_folder() {
                        self.root_path = Some(folder);
                        self.rebuild_custom_tree();
                        self.trigger_scan();
                    }
                }

                if let Some(ref path) = self.root_path {
                    ui.label(RichText::new(path.to_string_lossy()).size(13.0).strong());
                } else {
                    ui.label(RichText::new("No folder selected").italics().color(Color32::GRAY));
                }

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.button("🔄 Refresh").clicked() {
                        self.rebuild_custom_tree();
                        self.trigger_scan();
                    }
                });
            });
            ui.add_space(4.0);
        });

        // Bottom Status Panel
        egui::TopBottomPanel::bottom("bottom_status").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.label(RichText::new(&self.status_msg).size(12.0));

                if let Some(ref res) = self.result {
                    ui.separator();
                    ui.label(format!("📄 Files: {}", res.total_files));
                    ui.label(format!("💾 Size: {:.1} KB", res.total_bytes as f64 / 1024.0));
                    ui.label(
                        RichText::new(format!("🔤 Tokens: ~{}", res.estimated_tokens))
                            .strong()
                            .color(Color32::from_rgb(166, 227, 161)),
                    );

                    let tok = res.estimated_tokens as f32;
                    let gpt_pct = (tok / 128_000.0).min(1.0);
                    let claude_pct = (tok / 200_000.0).min(1.0);

                    ui.separator();
                    ui.label(format!("GPT-4o: {:.1}%", gpt_pct * 100.0));
                    ui.label(format!("Claude 3.5: {:.1}%", claude_pct * 100.0));
                }
            });
        });

        // Left Panel (تنظیمات، پریست‌ها و درخت فایل)
        egui::SidePanel::left("left_filters")
            .default_width(360.0)
            .min_width(300.0)
            .max_width(450.0)
            .resizable(true)
            .show(ctx, |ui| {
                ui.add_space(4.0);
                ui.heading("Presets & Rules");

                // گرید منظم ۲ در ۳ برای دکمه‌های پریست
                egui::Grid::new("presets_grid")
                    .num_columns(3)
                    .spacing([6.0, 6.0])
                    .show(ui, |ui| {
                        let presets = [
                            Preset::Rust,
                            Preset::WebTypeScript,
                            Preset::Python,
                            Preset::Go,
                            Preset::Fullstack,
                            Preset::Custom,
                        ];
                        for (i, p) in presets.iter().enumerate() {
                            let selected = self.config.preset == *p;
                            if ui.add_sized([105.0, 26.0], egui::SelectableLabel::new(selected, p.label())).clicked() {
                                if self.config.preset != *p {
                                    self.config.set_preset(*p);
                                    if *p == Preset::Custom && self.custom_tree.is_none() {
                                        self.rebuild_custom_tree();
                                    }
                                    self.trigger_scan();
                                }
                            }
                            if (i + 1) % 3 == 0 {
                                ui.end_row();
                            }
                        }
                    });

                ui.add_space(4.0);
                ui.separator();

                // گزینه‌های عمومی
                ui.horizontal(|ui| {
                    if ui.checkbox(&mut self.config.deduplication, "SHA-256 Dedup").changed() {
                        self.trigger_scan();
                    }
                    if ui.checkbox(&mut self.config.remove_comments, "Strip Comments").changed() {
                        self.trigger_scan();
                    }
                });

                if ui.checkbox(&mut self.config.respect_gitignore, "Inherit .gitignore rules").changed() {
                    self.trigger_scan();
                }

                ui.separator();

                // حالت Custom: درخت فایل تمام فضای باقیمانده را پر می‌کند
                if self.config.preset == Preset::Custom {
                    let (selected, total) = self
                        .custom_tree
                        .as_ref()
                        .map(|t| t.count_selected_files())
                        .unwrap_or((0, 0));

                    ui.horizontal(|ui| {
                        ui.label(RichText::new(format!("🌿 File Tree ({}/{} selected)", selected, total)).strong());
                    });

                    ui.horizontal(|ui| {
                        if ui.small_button("☑ All").clicked() {
                            if let Some(ref mut tree) = self.custom_tree {
                                tree.set_check_recursive(true);
                                self.trigger_scan();
                            }
                        }
                        if ui.small_button("☐ None").clicked() {
                            if let Some(ref mut tree) = self.custom_tree {
                                tree.set_check_recursive(false);
                                self.trigger_scan();
                            }
                        }
                        if ui.small_button("▼ Expand").clicked() {
                            if let Some(ref mut tree) = self.custom_tree {
                                tree.set_all_expanded(true);
                            }
                        }
                        if ui.small_button("▶ Collapse").clicked() {
                            if let Some(ref mut tree) = self.custom_tree {
                                tree.set_all_expanded(false);
                            }
                        }
                    });

                    ui.add_space(2.0);
                    ui.text_edit_singleline(&mut self.config.tree_search_query);
                    ui.add_space(2.0);

                    // اسکرول اختصاصی و بدون تداخل برای درخت فایل
                    ScrollArea::vertical()
                        .auto_shrink([false, false])
                        .show(ui, |ui| {
                            if let Some(ref mut tree) = self.custom_tree {
                                let query = self.config.tree_search_query.clone();
                                if render_file_tree_ui(ui, tree, &query) {
                                    self.trigger_scan(); // اعمال آنی و زنده تغییرات تیک‌ها
                                }
                            } else {
                                ui.label("Open a folder to see file tree.");
                            }
                        });
                } else {
                    // در سایر پریست‌ها: چیپ‌های مرتب و اسناد کمکی
                    ScrollArea::vertical().show(ui, |ui| {
                        ui.label(RichText::new("Allowed Extensions").strong());
                        render_chip_input(
                            ui,
                            &mut self.config.allowed_extensions,
                            &mut self.config.ext_input_buffer,
                            "+ ext",
                        );

                        ui.add_space(6.0);
                        ui.label(RichText::new("Ignored Folders").strong());
                        render_chip_input(
                            ui,
                            &mut self.config.ignored_directories,
                            &mut self.config.ignore_input_buffer,
                            "+ dir",
                        );

                        ui.separator();
                        ui.horizontal(|ui| {
                            ui.label(RichText::new("Context Docs").strong());
                            if ui.small_button("+ Add Doc").clicked() {
                                if let Some(file) = rfd::FileDialog::new()
                                    .add_filter("Markdown", &["md", "txt"])
                                    .pick_file()
                                {
                                    self.instruction_docs.push(file);
                                    self.trigger_scan();
                                }
                            }
                        });

                        let mut remove_idx = None;
                        for (idx, doc) in self.instruction_docs.iter().enumerate() {
                            ui.horizontal(|ui| {
                                ui.label(format!("📄 {}", doc.file_name().unwrap().to_string_lossy()));
                                if ui.small_button("✕").clicked() {
                                    remove_idx = Some(idx);
                                }
                            });
                        }
                        if let Some(idx) = remove_idx {
                            self.instruction_docs.remove(idx);
                            self.trigger_scan();
                        }
                    });
                }
            });

        // Central Panel (پیش‌نمایش و دکمه‌های اکشن)
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.horizontal(|ui| {
                let prev_format = self.config.output_format;
                ui.radio_value(&mut self.config.output_format, OutputFormat::Markdown, OutputFormat::Markdown.label());
                ui.radio_value(&mut self.config.output_format, OutputFormat::Xml, OutputFormat::Xml.label());
                ui.radio_value(&mut self.config.output_format, OutputFormat::Yaml, OutputFormat::Yaml.label());

                if self.config.output_format != prev_format {
                    if let Some(ref mut res) = self.result {
                        res.raw_output = format_snapshot(&res.payload, self.config.output_format);
                        res.estimated_tokens = res.raw_output.len() / 4;
                    }
                }

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    let copy_btn = egui::Button::new(
                        RichText::new("📋 Copy Output").strong().size(13.0).color(Color32::WHITE),
                    )
                    .fill(Color32::from_rgb(88, 101, 242));

                    if ui.add_sized(Vec2::new(115.0, 30.0), copy_btn).clicked() {
                        if let Some(ref res) = self.result {
                            ctx.output_mut(|o| o.copied_text = res.raw_output.clone());
                            self.copied_toast_time = Some(Instant::now());
                        }
                    }

                    let save_btn = egui::Button::new(
                        RichText::new("💾 Save As...").strong().size(13.0).color(Color32::WHITE),
                    )
                    .fill(Color32::from_rgb(52, 130, 80));

                    if ui.add_sized(Vec2::new(100.0, 30.0), save_btn).clicked() {
                        if let Some(ref res) = self.result {
                            let ext = self.config.output_format.extension();
                            let default_filename = format!("codebase_snapshot.{}", ext);
                            if let Some(save_path) = rfd::FileDialog::new()
                                .set_file_name(&default_filename)
                                .add_filter(self.config.output_format.label(), &[ext])
                                .save_file()
                            {
                                if let Err(e) = std::fs::write(&save_path, &res.raw_output) {
                                    self.status_msg = format!("Failed to save: {}", e);
                                } else {
                                    self.status_msg = format!("File saved: {:?}", save_path.file_name().unwrap_or_default());
                                }
                            }
                        }
                    }

                    if let Some(time) = self.copied_toast_time {
                        if time.elapsed().as_secs_f32() < 2.5 {
                            ui.label(RichText::new("✔ Copied!").color(Color32::GREEN).strong());
                        }
                    }
                });
            });

            ui.separator();

            ui.horizontal(|ui| {
                ui.selectable_value(&mut self.active_tab, ViewTab::Preview, "📄 Output Preview");
                ui.selectable_value(&mut self.active_tab, ViewTab::AsciiTree, "🌲 File Tree");
                ui.selectable_value(&mut self.active_tab, ViewTab::Omitted, "⚠️ Skipped Files");
            });

            ui.separator();

            ScrollArea::vertical().show(ui, |ui| {
                if let Some(ref res) = self.result {
                    match self.active_tab {
                        ViewTab::Preview => {
                            // جلوگیری از لگ در فایل‌های فوق حجیم
                            let preview_limit = 60_000;
                            if res.raw_output.len() > preview_limit {
                                ui.label(RichText::new("ℹ Preview is limited to first 60 KB for smooth UI performance. The full content will be copied or saved.").italics().color(Color32::from_rgb(250, 179, 135)));
                                let preview_slice = &res.raw_output[..preview_limit];
                                ui.add(
                                    egui::TextEdit::multiline(&mut preview_slice.to_string().as_str())
                                        .font(egui::TextStyle::Monospace)
                                        .desired_width(f32::INFINITY),
                                );
                            } else {
                                ui.add(
                                    egui::TextEdit::multiline(&mut res.raw_output.as_str())
                                        .font(egui::TextStyle::Monospace)
                                        .desired_width(f32::INFINITY),
                                );
                            }
                        }
                        ViewTab::AsciiTree => {
                            ui.add(
                                egui::TextEdit::multiline(&mut res.payload.tree_structure.as_str())
                                    .font(egui::TextStyle::Monospace)
                                    .desired_width(f32::INFINITY),
                            );
                        }
                        ViewTab::Omitted => {
                            if res.payload.omitted_files.is_empty() {
                                ui.label("No files were skipped.");
                            } else {
                                for (path, reason) in &res.payload.omitted_files {
                                    ui.label(format!("• {}: {}", path, reason));
                                }
                            }
                        }
                    }
                } else {
                    ui.centered_and_justified(|ui| {
                        ui.label(RichText::new("Drag & Drop your project folder here to generate context.").size(16.0).color(Color32::GRAY));
                    });
                }
            });
        });
    }
}