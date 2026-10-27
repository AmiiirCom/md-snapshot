#![windows_subsystem = "windows"]

use eframe::egui;
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, HashMap, HashSet};
use std::fs::File;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{channel, Receiver, Sender};
use std::sync::Arc;
use std::thread;
use walkdir::WalkDir;

const APP_NAME: &str = "MD Snapshot";

const DEFAULT_EXTENSIONS: &[&str] = &[
    "rs", "toml", "md", "json", "yaml", "yml", "py", "js", "ts", "tsx", "jsx",
    "html", "css", "go", "java", "c", "h", "cpp", "hpp", "sh", "sql",
];

const DEFAULT_IGNORED_DIRS: &[&str] = &[
    ".git", "target", "node_modules", "vendor", "dist", "build", ".idea",
    ".vscode", "__pycache__", ".next",
];

#[derive(Clone)]
struct AppConfig {
    project_path: Option<PathBuf>,
    extra_docs: Vec<PathBuf>,
    extensions: HashSet<String>,
    ignored_dirs: HashSet<String>,
    include_tree: bool,
    include_contents: bool,
    deduplicate: bool,
    max_file_size_kb: usize,
    max_total_size_mb: usize,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            project_path: None,
            extra_docs: Vec::new(),
            extensions: DEFAULT_EXTENSIONS.iter().map(|&s| s.to_string()).collect(),
            ignored_dirs: DEFAULT_IGNORED_DIRS.iter().map(|&s| s.to_string()).collect(),
            include_tree: true,
            include_contents: true,
            deduplicate: true,
            max_file_size_kb: 512,
            max_total_size_mb: 10,
        }
    }
}

#[derive(Clone, Debug)]
struct SkippedFile {
    relative_path: String,
    reason: String,
}

#[derive(Clone, Debug)]
struct GenerationResult {
    markdown: String,
    included_count: usize,
    extra_docs_count: usize,
    skipped_files: Vec<SkippedFile>,
    total_chars: usize,
    estimated_tokens: usize,
}

enum WorkerMessage {
    Progress { current_file: String, processed: usize, total: usize },
    Finished(GenerationResult),
    Error(String),
}

fn generate_app_icon() -> egui::IconData {
    const W: usize = 48;
    const H: usize = 48;
    let mut rgba = vec![0u8; W * H * 4];

    let font_m = [
        [1,0,0,0,1],
        [1,1,0,1,1],
        [1,0,1,0,1],
        [1,0,0,0,1],
        [1,0,0,0,1],
        [1,0,0,0,1],
        [1,0,0,0,1],
    ];
    let font_d = [
        [1,1,1,1,0],
        [1,0,0,0,1],
        [1,0,0,0,1],
        [1,0,0,0,1],
        [1,0,0,0,1],
        [1,0,0,0,1],
        [1,1,1,1,0],
    ];
    let font_s = [
        [0,1,1,1,1],
        [1,0,0,0,0],
        [1,0,0,0,0],
        [0,1,1,1,0],
        [0,0,0,0,1],
        [0,0,0,0,1],
        [1,1,1,1,0],
    ];

    for y in 0..H {
        for x in 0..W {
            let idx = (y * W + x) * 4;
            let dx = (x as i32 - 24).abs();
            let dy = (y as i32 - 24).abs();
            if dx > 22 && dy > 22 {
                continue;
            }
            rgba[idx] = 30;
            rgba[idx + 1] = 41;
            rgba[idx + 2] = 59;
            rgba[idx + 3] = 255;
        }
    }

    let mut draw_glyph = |glyph: &[[u8; 5]; 7], offset_x: usize, offset_y: usize, r: u8, g: u8, b: u8| {
        for gy in 0..7 {
            for gx in 0..5 {
                if glyph[gy][gx] == 1 {
                    for sy in 0..2 {
                        for sx in 0..2 {
                            let px = offset_x + gx * 2 + sx;
                            let py = offset_y + gy * 2 + sy;
                            if px < W && py < H {
                                let idx = (py * W + px) * 4;
                                rgba[idx] = r;
                                rgba[idx + 1] = g;
                                rgba[idx + 2] = b;
                                rgba[idx + 3] = 255;
                            }
                        }
                    }
                }
            }
        }
    };

    let top_y = 17;
    draw_glyph(&font_m, 6, top_y, 56, 189, 248);
    draw_glyph(&font_d, 19, top_y, 255, 255, 255);
    draw_glyph(&font_s, 32, top_y, 56, 189, 248);

    egui::IconData {
        rgba,
        width: W as u32,
        height: H as u32,
    }
}

fn run_packer(
    config: AppConfig,
    cancel_flag: Arc<AtomicBool>,
    tx: Sender<WorkerMessage>,
) {
    let root = match &config.project_path {
        Some(p) => p.clone(),
        None => {
            let _ = tx.send(WorkerMessage::Error("No project directory selected.".into()));
            return;
        }
    };

    let project_name = root
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| "Project".to_string());

    let mut extra_docs_rendered: Vec<(String, String)> = Vec::new();
    for doc_path in &config.extra_docs {
        if let Ok(mut file) = File::open(doc_path) {
            let mut content = String::new();
            if file.read_to_string(&mut content).is_ok() {
                let name = doc_path.file_name().unwrap_or_default().to_string_lossy().to_string();
                extra_docs_rendered.push((name, content.trim_end_matches(['\r', '\n']).to_string()));
            }
        }
    }

    let mut candidate_files: Vec<PathBuf> = Vec::new();

    for entry in WalkDir::new(&root).follow_links(false).into_iter() {
        if cancel_flag.load(Ordering::Relaxed) {
            return;
        }

        let entry = match entry {
            Ok(e) => e,
            Err(_) => continue,
        };

        let path = entry.path();

        if entry.file_type().is_dir() {
            if let Some(dir_name) = path.file_name().and_then(|n| n.to_str()) {
                if config.ignored_dirs.contains(dir_name) && path != root {
                    continue;
                }
            }
        }

        if entry.file_type().is_file() {
            let in_ignored = path
                .strip_prefix(&root)
                .unwrap_or(path)
                .components()
                .any(|c| config.ignored_dirs.contains(c.as_os_str().to_string_lossy().as_ref()));

            if in_ignored {
                continue;
            }

            if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
                if config.extensions.contains(&ext.to_lowercase()) {
                    candidate_files.push(path.to_path_buf());
                }
            }
        }
    }

    candidate_files.sort_by(|a, b| {
        let rel_a = a.strip_prefix(&root).unwrap_or(a);
        let rel_b = b.strip_prefix(&root).unwrap_or(b);
        rel_a.cmp(rel_b)
    });

    let mut included_relative_paths: Vec<String> = Vec::new();
    let mut file_contents: Vec<(String, String, Option<String>)> = Vec::new();
    let mut skipped_files: Vec<SkippedFile> = Vec::new();
    let mut content_hashes: HashMap<String, String> = HashMap::new();
    let mut current_total_bytes: usize = 0;
    let max_total_bytes = config.max_total_size_mb * 1024 * 1024;
    let max_file_bytes = config.max_file_size_kb * 1024;

    let total_candidates = candidate_files.len();

    for (idx, file_path) in candidate_files.into_iter().enumerate() {
        if cancel_flag.load(Ordering::Relaxed) {
            return;
        }

        let rel_path = file_path
            .strip_prefix(&root)
            .unwrap_or(&file_path)
            .to_string_lossy()
            .replace('\\', "/");

        let _ = tx.send(WorkerMessage::Progress {
            current_file: rel_path.clone(),
            processed: idx + 1,
            total: total_candidates,
        });

        let metadata = match std::fs::metadata(&file_path) {
            Ok(m) => m,
            Err(e) => {
                skipped_files.push(SkippedFile {
                    relative_path: rel_path,
                    reason: format!("Metadata error: {}", e),
                });
                continue;
            }
        };

        if metadata.len() as usize > max_file_bytes {
            skipped_files.push(SkippedFile {
                relative_path: rel_path,
                reason: format!("File exceeds size limit ({} KB)", metadata.len() / 1024),
            });
            continue;
        }

        let mut buffer = Vec::new();
        match File::open(&file_path).and_then(|mut f| f.read_to_end(&mut buffer)) {
            Ok(_) => {},
            Err(e) => {
                skipped_files.push(SkippedFile {
                    relative_path: rel_path,
                    reason: format!("Read error: {}", e),
                });
                continue;
            }
        }

        let probe_len = buffer.len().min(8192);
        if buffer[..probe_len].contains(&0) {
            skipped_files.push(SkippedFile {
                relative_path: rel_path,
                reason: "Binary file detected".into(),
            });
            continue;
        }

        let text_content = match String::from_utf8(buffer) {
            Ok(s) => s,
            Err(_) => {
                skipped_files.push(SkippedFile {
                    relative_path: rel_path,
                    reason: "Invalid UTF-8 encoding".into(),
                });
                continue;
            }
        };

        let trimmed_content = text_content.trim_end_matches(['\r', '\n']).to_string();

        if current_total_bytes + trimmed_content.len() > max_total_bytes {
            skipped_files.push(SkippedFile {
                relative_path: rel_path,
                reason: "Total output limit exceeded".into(),
            });
            continue;
        }

        included_relative_paths.push(rel_path.clone());

        if config.include_contents {
            if config.deduplicate {
                let mut hasher = Sha256::new();
                hasher.update(trimmed_content.as_bytes());
                let hash_str = format!("{:x}", hasher.finalize());

                if let Some(first_path) = content_hashes.get(&hash_str) {
                    file_contents.push((rel_path, String::new(), Some(first_path.clone())));
                    continue;
                } else {
                    content_hashes.insert(hash_str, rel_path.clone());
                }
            }

            current_total_bytes += trimmed_content.len();
            file_contents.push((rel_path, trimmed_content, None));
        }
    }

    let mut md = String::new();
    md.push_str(&format!("# Project: {}\n\n", project_name));

    if !extra_docs_rendered.is_empty() {
        md.push_str("## Context & Instructions\n\n");
        for (doc_name, content) in &extra_docs_rendered {
            md.push_str(&format!("### {}\n", doc_name));
            let fence = make_safe_fence(content);
            md.push_str(&format!("{}markdown\n{}\n{}\n\n", fence, content, fence));
        }
    }

    if config.include_tree {
        md.push_str("## Project Tree\n```text\n");
        let tree_repr = build_compact_tree(&included_relative_paths);
        md.push_str(&tree_repr);
        md.push_str("```\n\n");
    }

    if config.include_contents && !file_contents.is_empty() {
        md.push_str("## Source Files\n\n");
        for (rel_path, content, duplicate_of) in file_contents {
            md.push_str(&format!("### {}\n", rel_path));

            if let Some(original) = duplicate_of {
                md.push_str(&format!("*(Identical content to `{}`)*\n\n", original));
            } else {
                let lang = detect_language(&rel_path);
                let fence = make_safe_fence(&content);
                md.push_str(&format!("{}{}\n{}\n{}\n\n", fence, lang, content, fence));
            }
        }
    }

    if !skipped_files.is_empty() {
        md.push_str("## Omitted / Skipped Files\n");
        for skipped in &skipped_files {
            md.push_str(&format!("- `{}`: {}\n", skipped.relative_path, skipped.reason));
        }
        md.push('\n');
    }

    let total_chars = md.chars().count();
    let estimated_tokens = (total_chars + 3) / 4;

    let res = GenerationResult {
        markdown: md,
        included_count: included_relative_paths.len(),
        extra_docs_count: extra_docs_rendered.len(),
        skipped_files,
        total_chars,
        estimated_tokens,
    };

    let _ = tx.send(WorkerMessage::Finished(res));
}

fn make_safe_fence(content: &str) -> String {
    let mut max_ticks = 0;
    let mut current_ticks = 0;

    for ch in content.chars() {
        if ch == '`' {
            current_ticks += 1;
            if current_ticks > max_ticks {
                max_ticks = current_ticks;
            }
        } else {
            current_ticks = 0;
        }
    }

    let fence_len = (max_ticks + 1).max(3);
    "`".repeat(fence_len)
}

fn detect_language(path: &str) -> &'static str {
    let ext = Path::new(path)
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase();

    match ext.as_str() {
        "rs" => "rust",
        "py" => "python",
        "js" => "javascript",
        "ts" => "typescript",
        "tsx" => "tsx",
        "jsx" => "jsx",
        "toml" => "toml",
        "json" => "json",
        "yaml" | "yml" => "yaml",
        "md" => "markdown",
        "html" => "html",
        "css" => "css",
        "go" => "go",
        "java" => "java",
        "c" | "h" => "c",
        "cpp" | "hpp" | "cc" => "cpp",
        "sh" | "bash" => "bash",
        "sql" => "sql",
        _ => "text",
    }
}

fn build_compact_tree(paths: &[String]) -> String {
    #[derive(Default)]
    struct Node {
        children: BTreeMap<String, Node>,
        is_file: bool,
    }

    let mut root = Node::default();
    for path in paths {
        let parts: Vec<&str> = path.split('/').collect();
        let mut curr = &mut root;
        for (i, part) in parts.iter().enumerate() {
            let is_last = i == parts.len() - 1;
            curr = curr.children.entry(part.to_string()).or_default();
            if is_last {
                curr.is_file = true;
            }
        }
    }

    fn render(node: &Node, prefix: &str, output: &mut String) {
        let total = node.children.len();
        for (i, (name, child)) in node.children.iter().enumerate() {
            let is_last = i == total - 1;
            let pointer = if is_last { "└── " } else { "├── " };
            output.push_str(&format!("{}{}{}\n", prefix, pointer, name));
            let next_prefix = format!("{}{}", prefix, if is_last { "    " } else { "│   " });
            render(child, &next_prefix, output);
        }
    }

    let mut out = String::new();
    render(&root, "", &mut out);
    if out.is_empty() {
        out.push_str("(empty)\n");
    }
    out
}

struct App {
    config: AppConfig,
    new_ext_input: String,
    new_dir_input: String,

    worker_rx: Option<Receiver<WorkerMessage>>,
    cancel_flag: Option<Arc<AtomicBool>>,
    is_working: bool,
    progress_status: String,
    processed_count: usize,
    total_count: usize,

    result: Option<GenerationResult>,
    notification_msg: Option<(String, std::time::Instant)>,
}

impl Default for App {
    fn default() -> Self {
        Self {
            config: AppConfig::default(),
            new_ext_input: String::new(),
            new_dir_input: String::new(),
            worker_rx: None,
            cancel_flag: None,
            is_working: false,
            progress_status: String::new(),
            processed_count: 0,
            total_count: 0,
            result: None,
            notification_msg: None,
        }
    }
}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        let mut style = (*ctx.style()).clone();
        style.interaction.selectable_labels = false;
        style.spacing.item_spacing = egui::vec2(8.0, 6.0);
        style.spacing.button_padding = egui::vec2(10.0, 6.0);
        style.visuals.window_rounding = egui::Rounding::same(8.0);
        ctx.set_style(style);

        let mut finished_res = None;
        let mut error_msg = None;

        if let Some(ref rx) = self.worker_rx {
            while let Ok(msg) = rx.try_recv() {
                match msg {
                    WorkerMessage::Progress { current_file, processed, total } => {
                        self.progress_status = current_file;
                        self.processed_count = processed;
                        self.total_count = total;
                    }
                    WorkerMessage::Finished(res) => {
                        finished_res = Some(res);
                        break;
                    }
                    WorkerMessage::Error(err) => {
                        error_msg = Some(err);
                        break;
                    }
                }
            }
        }

        if let Some(res) = finished_res {
            self.result = Some(res);
            self.is_working = false;
            self.worker_rx = None;
            self.cancel_flag = None;
        } else if let Some(err) = error_msg {
            self.notification_msg = Some((format!("Error: {}", err), std::time::Instant::now()));
            self.is_working = false;
            self.worker_rx = None;
            self.cancel_flag = None;
        }

        // نوار بالایی کاملاً ساده و بدون اموجی
        egui::TopBottomPanel::top("top_bar").show(ctx, |ui| {
            ui.add_space(4.0);
            ui.horizontal(|ui| {
                ui.label(
                    egui::RichText::new("MD Snapshot")
                        .strong()
                        .size(17.0)
                        .color(egui::Color32::from_rgb(56, 189, 248)),
                );
                ui.label(
                    egui::RichText::new("| LLM Context Packer")
                        .weak()
                        .size(13.0),
                );
            });
            ui.add_space(4.0);
        });

        // پنل تنظیمات سمت چپ
        egui::SidePanel::left("control_panel")
            .resizable(true)
            .default_width(340.0)
            .show(ctx, |ui| {
                egui::ScrollArea::vertical().show(ui, |ui| {
                    ui.add_space(4.0);

                    ui.label(egui::RichText::new("Project Root Directory").strong());
                    ui.horizontal(|ui| {
                        if ui.button("Select Folder...").clicked() {
                            if let Some(folder) = rfd::FileDialog::new().pick_folder() {
                                self.config.project_path = Some(folder);
                            }
                        }
                        if let Some(ref p) = self.config.project_path {
                            ui.label(
                                egui::RichText::new(p.file_name().unwrap_or_default().to_string_lossy())
                                    .color(egui::Color32::from_rgb(52, 211, 153))
                                    .strong(),
                            );
                        } else {
                            ui.weak("No folder selected");
                        }
                    });

                    ui.separator();

                    ui.label(egui::RichText::new("Extra Context / Docs (skill.md, agent.md)").strong());
                    ui.horizontal(|ui| {
                        if ui.button("Add Docs...").clicked() {
                            if let Some(files) = rfd::FileDialog::new()
                                .add_filter("Markdown & Text", &["md", "txt", "json", "yaml", "yml"])
                                .pick_files()
                            {
                                for file in files {
                                    if !self.config.extra_docs.contains(&file) {
                                        self.config.extra_docs.push(file);
                                    }
                                }
                            }
                        }
                        if !self.config.extra_docs.is_empty() {
                            if ui.button("Clear All").clicked() {
                                self.config.extra_docs.clear();
                            }
                        }
                    });

                    if !self.config.extra_docs.is_empty() {
                        let mut to_remove_doc = None;
                        for (idx, doc) in self.config.extra_docs.iter().enumerate() {
                            ui.horizontal(|ui| {
                                let name = doc.file_name().unwrap_or_default().to_string_lossy();
                                ui.label(format!("- {}", name));
                                if ui.small_button("X").clicked() {
                                    to_remove_doc = Some(idx);
                                }
                            });
                        }
                        if let Some(idx) = to_remove_doc {
                            self.config.extra_docs.remove(idx);
                        }
                    } else {
                        ui.weak("No extra docs attached (Optional)");
                    }

                    ui.separator();

                    ui.label(egui::RichText::new("Output Rules").strong());
                    ui.checkbox(&mut self.config.include_tree, "Include Visual File Tree");
                    ui.checkbox(&mut self.config.include_contents, "Include Source File Contents");
                    ui.checkbox(&mut self.config.deduplicate, "Deduplicate by Content Hash");

                    ui.separator();
                    ui.label(egui::RichText::new("Limits").strong());
                    ui.horizontal(|ui| {
                        ui.label("Per File (KB):");
                        ui.add(egui::DragValue::new(&mut self.config.max_file_size_kb).clamp_range(16..=20480));
                    });
                    ui.horizontal(|ui| {
                        ui.label("Total Output (MB):");
                        ui.add(egui::DragValue::new(&mut self.config.max_total_size_mb).clamp_range(1..=200));
                    });

                    ui.separator();
                    ui.collapsing("Allowed File Extensions", |ui| {
                        ui.horizontal(|ui| {
                            ui.text_edit_singleline(&mut self.new_ext_input);
                            if ui.button("Add").clicked() && !self.new_ext_input.trim().is_empty() {
                                let ext = self.new_ext_input.trim().trim_start_matches('.').to_lowercase();
                                self.config.extensions.insert(ext);
                                self.new_ext_input.clear();
                            }
                        });

                        let mut to_remove_ext = None;
                        for ext in &self.config.extensions {
                            ui.horizontal(|ui| {
                                ui.label(format!(".{}", ext));
                                if ui.small_button("X").clicked() {
                                    to_remove_ext = Some(ext.clone());
                                }
                            });
                        }
                        if let Some(ext) = to_remove_ext {
                            self.config.extensions.remove(&ext);
                        }
                    });

                    ui.collapsing("Ignored Directories", |ui| {
                        ui.horizontal(|ui| {
                            ui.text_edit_singleline(&mut self.new_dir_input);
                            if ui.button("Add").clicked() && !self.new_dir_input.trim().is_empty() {
                                let dir = self.new_dir_input.trim().to_string();
                                self.config.ignored_dirs.insert(dir);
                                self.new_dir_input.clear();
                            }
                        });

                        let mut to_remove_dir = None;
                        for dir in &self.config.ignored_dirs {
                            ui.horizontal(|ui| {
                                ui.label(dir);
                                if ui.small_button("X").clicked() {
                                    to_remove_dir = Some(dir.clone());
                                }
                            });
                        }
                        if let Some(dir) = to_remove_dir {
                            self.config.ignored_dirs.remove(&dir);
                        }
                    });

                    ui.separator();

                    if !self.is_working {
                        let can_run = self.config.project_path.is_some();
                        let btn = egui::Button::new(
                            egui::RichText::new("Generate Snapshot")
                                .strong()
                                .size(14.0),
                        )
                        .min_size(egui::vec2(ui.available_width(), 36.0));

                        if ui.add_enabled(can_run, btn).clicked() {
                            let (tx, rx) = channel();
                            let cancel_flag = Arc::new(AtomicBool::new(false));
                            let cancel_clone = cancel_flag.clone();
                            let conf = self.config.clone();

                            self.worker_rx = Some(rx);
                            self.cancel_flag = Some(cancel_flag);
                            self.is_working = true;
                            self.processed_count = 0;
                            self.total_count = 0;
                            self.progress_status = "Scanning files...".to_string();

                            thread::spawn(move || {
                                run_packer(conf, cancel_clone, tx);
                            });
                        }
                    } else {
                        ui.horizontal(|ui| {
                            ui.spinner();
                            if ui.button("Cancel").clicked() {
                                if let Some(ref flag) = self.cancel_flag {
                                    flag.store(true, Ordering::Relaxed);
                                }
                                self.is_working = false;
                            }
                        });
                        ui.label(format!("Processed: {} of {}", self.processed_count, self.total_count));
                        ui.weak(&self.progress_status);
                        ctx.request_repaint();
                    }
                });
            });

        // پنل میانی خروجی و پیش‌نمایش
        egui::CentralPanel::default().show(ctx, |ui| {
            if let Some((ref msg, time)) = self.notification_msg {
                if time.elapsed().as_secs() < 4 {
                    ui.label(
                        egui::RichText::new(msg)
                            .strong()
                            .color(egui::Color32::from_rgb(52, 211, 153)),
                    );
                } else {
                    self.notification_msg = None;
                }
            }

            if let Some(ref res) = self.result {
                ui.horizontal(|ui| {
                    if ui.button(egui::RichText::new("Copy All").strong()).clicked() {
                        ctx.output_mut(|o| o.copied_text = res.markdown.clone());
                        self.notification_msg = Some(("Copied to clipboard successfully.".into(), std::time::Instant::now()));
                    }

                    if ui.button(egui::RichText::new("Save File...").strong()).clicked() {
                        if let Some(save_path) = rfd::FileDialog::new()
                            .set_file_name("md_snapshot.md")
                            .save_file()
                        {
                            if let Err(e) = std::fs::write(&save_path, &res.markdown) {
                                self.notification_msg = Some((format!("Save error: {}", e), std::time::Instant::now()));
                            } else {
                                self.notification_msg = Some(("File saved successfully.".into(), std::time::Instant::now()));
                            }
                        }
                    }

                    ui.separator();
                    ui.label(egui::RichText::new(format!("Files: {}", res.included_count)).strong());
                    if res.extra_docs_count > 0 {
                        ui.label(
                            egui::RichText::new(format!("Docs: {}", res.extra_docs_count))
                                .color(egui::Color32::from_rgb(251, 191, 36)),
                        );
                    }
                    ui.label(format!("Skipped: {}", res.skipped_files.len()));
                    ui.label(format!("Size: {} KB", res.total_chars / 1024));

                    ui.label(
                        egui::RichText::new(format!("~{} Tokens", res.estimated_tokens))
                            .strong()
                            .color(egui::Color32::from_rgb(56, 189, 248)),
                    );
                });

                ui.add_space(2.0);
                ui.weak("Note: Token count is an approximation (~4 chars/token). Output is ready for LLMs.");
                ui.separator();

                egui::ScrollArea::vertical()
                    .auto_shrink([false; 2])
                    .show(ui, |ui| {
                        let mut text = res.markdown.as_str();
                        ui.add(
                            egui::TextEdit::multiline(&mut text)
                                .font(egui::TextStyle::Monospace)
                                .desired_width(f32::INFINITY)
                                .lock_focus(true),
                        );
                    });
            } else {
                ui.centered_and_justified(|ui| {
                    ui.label(
                        egui::RichText::new("Select a project folder and click \"Generate Snapshot\" to preview.")
                            .size(14.0)
                            .weak(),
                    );
                });
            }
        });
    }
}

fn main() -> eframe::Result<()> {
    let icon = generate_app_icon();

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1040.0, 720.0])
            .with_min_inner_size([780.0, 520.0])
            .with_title(APP_NAME)
            .with_icon(icon),
        ..Default::default()
    };

    eframe::run_native(
        APP_NAME,
        options,
        Box::new(|_cc| Box::new(App::default())),
    )
}