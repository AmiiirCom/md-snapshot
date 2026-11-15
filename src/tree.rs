use std::collections::HashSet;
use std::path::{Path, PathBuf};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CheckState {
    Unchecked,
    Checked,
    Indeterminate,
}

#[derive(Clone, Debug)]
pub struct FileTreeNode {
    pub name: String,
    pub relative_path: PathBuf,
    pub is_dir: bool,
    pub check_state: CheckState,
    pub children: Vec<FileTreeNode>,
    pub is_expanded: bool,
    pub size_bytes: u64,
}

impl FileTreeNode {
    pub fn new_dir(name: String, relative_path: PathBuf) -> Self {
        Self {
            name,
            relative_path,
            is_dir: true,
            check_state: CheckState::Checked,
            children: Vec::new(),
            is_expanded: true,
            size_bytes: 0,
        }
    }

    pub fn new_file(name: String, relative_path: PathBuf, size_bytes: u64) -> Self {
        Self {
            name,
            relative_path,
            is_dir: false,
            check_state: CheckState::Checked,
            children: Vec::new(),
            is_expanded: false,
            size_bytes,
        }
    }

    pub fn set_check_recursive(&mut self, checked: bool) {
        self.check_state = if checked {
            CheckState::Checked
        } else {
            CheckState::Unchecked
        };
        for child in &mut self.children {
            child.set_check_recursive(checked);
        }
    }

    pub fn update_state_from_children(&mut self) {
        if !self.is_dir || self.children.is_empty() {
            return;
        }

        for child in &mut self.children {
            child.update_state_from_children();
        }

        let total = self.children.len();
        let checked_count = self
            .children
            .iter()
            .filter(|c| c.check_state == CheckState::Checked)
            .count();
        let unchecked_count = self
            .children
            .iter()
            .filter(|c| c.check_state == CheckState::Unchecked)
            .count();

        if checked_count == total {
            self.check_state = CheckState::Checked;
        } else if unchecked_count == total {
            self.check_state = CheckState::Unchecked;
        } else {
            self.check_state = CheckState::Indeterminate;
        }
    }

    /// جمع‌آوری دقیق فایل‌های تیک‌خورده به صورت مسیر نرمالایز شده
    pub fn collect_selected_file_paths(&self, acc: &mut HashSet<String>) {
        if self.check_state == CheckState::Unchecked {
            return;
        }
        if !self.is_dir {
            if self.check_state == CheckState::Checked {
                acc.insert(normalize_rel_path(&self.relative_path));
            }
            return;
        }
        for child in &self.children {
            child.collect_selected_file_paths(acc);
        }
    }

    /// شمارش (فایل‌های انتخاب‌شده، کل فایل‌ها)
    pub fn count_selected_files(&self) -> (usize, usize) {
        if !self.is_dir {
            let is_checked = if self.check_state == CheckState::Checked { 1 } else { 0 };
            return (is_checked, 1);
        }
        let mut selected = 0;
        let mut total = 0;
        for child in &self.children {
            let (s, t) = child.count_selected_files();
            selected += s;
            total += t;
        }
        (selected, total)
    }

    pub fn set_all_expanded(&mut self, expand: bool) {
        if self.is_dir {
            self.is_expanded = expand;
            for child in &mut self.children {
                child.set_all_expanded(expand);
            }
        }
    }
}

pub fn normalize_rel_path(path: &Path) -> String {
    path.to_string_lossy().replace('\\', "/")
}

pub fn build_ui_tree(root_path: &Path, config: &crate::config::AppConfig) -> Option<FileTreeNode> {
    let root_name = root_path
        .file_name()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_else(|| "root".to_string());

    let mut root_node = FileTreeNode::new_dir(root_name, PathBuf::from("."));
    root_node.is_expanded = true;

    build_tree_recursive(root_path, root_path, &mut root_node, config);
    Some(root_node)
}

fn build_tree_recursive(
    base: &Path,
    current: &Path,
    node: &mut FileTreeNode,
    config: &crate::config::AppConfig,
) {
    let read_dir = match std::fs::read_dir(current) {
        Ok(rd) => rd,
        Err(_) => return,
    };

    let mut entries = Vec::new();
    for entry in read_dir.flatten() {
        entries.push(entry);
    }

    entries.sort_by(|a, b| {
        let a_is_dir = a.file_type().map(|t| t.is_dir()).unwrap_or(false);
        let b_is_dir = b.file_type().map(|t| t.is_dir()).unwrap_or(false);
        b_is_dir.cmp(&a_is_dir).then_with(|| a.file_name().cmp(&b.file_name()))
    });

    for entry in entries {
        let path = entry.path();
        let name = entry.file_name().to_string_lossy().to_string();

        if let Ok(file_type) = entry.file_type() {
            if file_type.is_symlink() {
                continue;
            }

            let rel_path = path.strip_prefix(base).unwrap_or(&path).to_path_buf();

            if file_type.is_dir() {
                if name == ".git" {
                    continue;
                }
                if config.preset != crate::config::Preset::Custom && config.is_dir_ignored(&name) {
                    continue;
                }
                let mut child_node = FileTreeNode::new_dir(name, rel_path);
                build_tree_recursive(base, &path, &mut child_node, config);
                if !child_node.children.is_empty() {
                    node.children.push(child_node);
                }
            } else if file_type.is_file() {
                let is_allowed = if config.preset == crate::config::Preset::Custom {
                    true // در حالت Custom تمام فایل‌ها حتی .env و بدون پسوندها مجازند
                } else {
                    let ext = path
                        .extension()
                        .and_then(|s| s.to_str())
                        .unwrap_or("")
                        .to_lowercase();
                    config.is_ext_allowed(&ext)
                };

                if is_allowed {
                    let size = entry.metadata().map(|m| m.len()).unwrap_or(0);
                    node.children.push(FileTreeNode::new_file(name, rel_path, size));
                }
            }
        }
    }
}