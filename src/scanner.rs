use crate::config::{AppConfig, Preset};
use crate::formatters::{format_snapshot, SnapshotFile, SnapshotPayload};
use crate::tree::normalize_rel_path;
use sha2::{Digest, Sha256};
use std::collections::{HashMap, HashSet};
use std::fs::File;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use walkdir::WalkDir;

pub struct ScanResult {
    pub raw_output: String,
    pub total_files: usize,
    pub total_bytes: usize,
    pub estimated_tokens: usize,
    pub payload: SnapshotPayload,
    pub elapsed_ms: u128,
}

pub fn execute_scan(
    root: PathBuf,
    config: AppConfig,
    selected_custom_files: Option<HashSet<String>>,
    instruction_paths: Vec<PathBuf>,
    cancel_flag: Arc<AtomicBool>,
) -> Option<ScanResult> {
    let start_time = std::time::Instant::now();

    let mut instructions = Vec::new();
    for doc_path in instruction_paths {
        if let Ok(content) = std::fs::read_to_string(&doc_path) {
            let name = doc_path
                .file_name()
                .map(|s| s.to_string_lossy().to_string())
                .unwrap_or_else(|| "Instruction".to_string());
            instructions.push((name, content));
        }
    }

    let mut files_to_read = Vec::new();
    let mut omitted_files = Vec::new();

    if config.respect_gitignore {
        let walker = ignore::WalkBuilder::new(&root)
            .git_ignore(true)
            .git_global(false)
            .git_exclude(true)
            .build();

        for result in walker.flatten() {
            if cancel_flag.load(Ordering::Relaxed) {
                return None;
            }
            let path = result.path();
            if path.is_file() {
                evaluate_file(
                    path,
                    &root,
                    &config,
                    &selected_custom_files,
                    &mut files_to_read,
                    &mut omitted_files,
                );
            }
        }
    } else {
        let walker = WalkDir::new(&root).into_iter();
        for entry in walker.filter_entry(|e| {
            if e.file_type().is_dir() {
                let name = e.file_name().to_string_lossy();
                if name == ".git" {
                    return false;
                }
                if config.preset != Preset::Custom && config.is_dir_ignored(&name) {
                    return false;
                }
                true
            } else {
                true
            }
        }) {
            if cancel_flag.load(Ordering::Relaxed) {
                return None;
            }
            if let Ok(entry) = entry {
                if entry.file_type().is_file() {
                    evaluate_file(
                        entry.path(),
                        &root,
                        &config,
                        &selected_custom_files,
                        &mut files_to_read,
                        &mut omitted_files,
                    );
                }
            }
        }
    }

    files_to_read.sort_by(|a, b| a.0.cmp(&b.0));

    let mut snapshot_files = Vec::new();
    let mut seen_hashes: HashMap<String, String> = HashMap::new();
    let mut total_bytes = 0;
    let max_output_bytes = config.total_output_limit_mb * 1024 * 1024;

    for (rel_path, abs_path) in files_to_read {
        if cancel_flag.load(Ordering::Relaxed) {
            return None;
        }

        if total_bytes > max_output_bytes {
            omitted_files.push((
                rel_path,
                format!("Exceeded total limit ({} MB)", config.total_output_limit_mb),
            ));
            continue;
        }

        let mut file = match File::open(&abs_path) {
            Ok(f) => f,
            Err(_) => {
                omitted_files.push((rel_path, "Could not open file".to_string()));
                continue;
            }
        };

        let mut buffer = Vec::new();
        if file.read_to_end(&mut buffer).is_err() {
            omitted_files.push((rel_path, "Error reading bytes".to_string()));
            continue;
        }

        if buffer.iter().take(1024).any(|&b| b == 0) {
            omitted_files.push((rel_path, "Binary file detected".to_string()));
            continue;
        }

        let content_str = match String::from_utf8(buffer) {
            Ok(s) => s,
            Err(_) => {
                omitted_files.push((rel_path, "Non-UTF-8 encoding".to_string()));
                continue;
            }
        };

        let clean_content = if config.remove_comments {
            strip_comments_simple(&content_str)
        } else {
            content_str
        };

        total_bytes += clean_content.len();

        if config.deduplication {
            let mut hasher = Sha256::new();
            hasher.update(clean_content.as_bytes());
            let hash_hex = format!("{:x}", hasher.finalize());

            if let Some(orig) = seen_hashes.get(&hash_hex) {
                snapshot_files.push(SnapshotFile {
                    relative_path: rel_path,
                    content: String::new(),
                    is_duplicate_of: Some(orig.clone()),
                });
                continue;
            } else {
                seen_hashes.insert(hash_hex, rel_path.clone());
            }
        }

        snapshot_files.push(SnapshotFile {
            relative_path: rel_path,
            content: clean_content,
            is_duplicate_of: None,
        });
    }

    let tree_structure = build_ascii_tree(&snapshot_files);

    let payload = SnapshotPayload {
        tree_structure,
        instruction_docs: instructions,
        files: snapshot_files,
        omitted_files,
    };

    let raw_output = format_snapshot(&payload, config.output_format);
    let estimated_tokens = raw_output.len() / 4;
    let elapsed_ms = start_time.elapsed().as_millis();

    Some(ScanResult {
        total_files: payload.files.len(),
        total_bytes,
        estimated_tokens,
        raw_output,
        payload,
        elapsed_ms,
    })
}

fn evaluate_file(
    path: &Path,
    root: &Path,
    config: &AppConfig,
    selected_custom_files: &Option<HashSet<String>>,
    files_to_read: &mut Vec<(String, PathBuf)>,
    omitted_files: &mut Vec<(String, String)>,
) {
    let rel = match path.strip_prefix(root) {
        Ok(r) => r.to_path_buf(),
        Err(_) => path.to_path_buf(),
    };

    let rel_str = normalize_rel_path(&rel);

    // ۱. در حالت Custom: فایل دقیقاً باید در لیست تیک‌خورده‌ها باشد
    if config.preset == Preset::Custom {
        if let Some(selected) = selected_custom_files {
            if !selected.contains(&rel_str) {
                return;
            }
        }
    } else {
        // ۲. در سایر پریست‌ها: فیلتر بر اساس اکستنشن مجاز
        let ext = path
            .extension()
            .and_then(|s| s.to_str())
            .unwrap_or("")
            .to_lowercase();

        if !config.is_ext_allowed(&ext) {
            return;
        }
    }

    if let Ok(meta) = path.metadata() {
        if meta.len() > (config.max_file_size_kb as u64 * 1024) {
            omitted_files.push((
                rel_str.clone(),
                format!("Exceeded size limit ({} KB)", meta.len() / 1024),
            ));
            return;
        }
    }

    files_to_read.push((rel_str, path.to_path_buf()));
}

fn build_ascii_tree(files: &[SnapshotFile]) -> String {
    let mut tree = String::new();
    for f in files {
        tree.push_str("├── ");
        tree.push_str(&f.relative_path);
        tree.push('\n');
    }
    tree
}

fn strip_comments_simple(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    for line in input.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("//") || trimmed.starts_with('#') {
            continue;
        }
        out.push_str(line);
        out.push('\n');
    }
    out
}