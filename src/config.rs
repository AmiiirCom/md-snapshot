use std::collections::HashSet;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OutputFormat {
    Markdown,
    Xml,
    Yaml,
}

impl OutputFormat {
    pub fn label(&self) -> &'static str {
        match self {
            OutputFormat::Markdown => "Markdown (.md)",
            OutputFormat::Xml => "XML (.xml)",
            OutputFormat::Yaml => "YAML (.yaml)",
        }
    }

    pub fn extension(&self) -> &'static str {
        match self {
            OutputFormat::Markdown => "md",
            OutputFormat::Xml => "xml",
            OutputFormat::Yaml => "yaml",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Preset {
    Rust,
    WebTypeScript,
    Python,
    Go,
    Fullstack,
    Custom,
}

impl Preset {
    pub fn label(&self) -> &'static str {
        match self {
            Preset::Rust => "🦀 Rust",
            Preset::WebTypeScript => "⚡ Web / TS",
            Preset::Python => "🐍 Python",
            Preset::Go => "🐹 Go",
            Preset::Fullstack => "🌐 Fullstack",
            Preset::Custom => "🛠 Custom (Tree)",
        }
    }

    pub fn default_extensions(&self) -> Vec<String> {
        match self {
            Preset::Rust => vec!["rs", "toml", "md"].into_iter().map(String::from).collect(),
            Preset::WebTypeScript => vec!["ts", "tsx", "js", "jsx", "json", "html", "css", "md"].into_iter().map(String::from).collect(),
            Preset::Python => vec!["py", "toml", "yaml", "yml", "sql", "md"].into_iter().map(String::from).collect(),
            Preset::Go => vec!["go", "mod", "sum", "md"].into_iter().map(String::from).collect(),
            Preset::Fullstack => vec!["rs", "ts", "tsx", "py", "json", "toml", "html", "css", "sql", "md"].into_iter().map(String::from).collect(),
            Preset::Custom => vec!["rs", "ts", "py", "json", "md"].into_iter().map(String::from).collect(),
        }
    }

    pub fn default_ignored_dirs(&self) -> Vec<String> {
        let base = vec![".git", ".idea", ".vscode"];
        let mut list: Vec<String> = base.into_iter().map(String::from).collect();
        match self {
            Preset::Rust => list.extend(vec!["target"].into_iter().map(String::from)),
            Preset::WebTypeScript => list.extend(vec!["node_modules", "dist", ".next", "build"].into_iter().map(String::from)),
            Preset::Python => list.extend(vec!["__pycache__", ".venv", "venv", ".pytest_cache"].into_iter().map(String::from)),
            Preset::Go => list.extend(vec!["vendor", "bin"].into_iter().map(String::from)),
            Preset::Fullstack => list.extend(vec!["target", "node_modules", "dist", "__pycache__", ".venv"].into_iter().map(String::from)),
            Preset::Custom => list.extend(vec!["target", "node_modules"].into_iter().map(String::from)),
        }
        list
    }
}

#[derive(Clone, Debug)]
pub struct AppConfig {
    pub preset: Preset,
    pub output_format: OutputFormat,
    pub allowed_extensions: HashSet<String>,
    pub ignored_directories: HashSet<String>,
    pub respect_gitignore: bool, // اختیاری و پیش‌فرض خاموش (false)
    pub deduplication: bool,
    pub remove_comments: bool,
    pub max_file_size_kb: usize,
    pub total_output_limit_mb: usize,
    pub ext_input_buffer: String,
    pub ignore_input_buffer: String,
    pub tree_search_query: String,
}

impl Default for AppConfig {
    fn default() -> Self {
        let preset = Preset::Rust;
        Self {
            preset,
            output_format: OutputFormat::Markdown,
            allowed_extensions: preset.default_extensions().into_iter().collect(),
            ignored_directories: preset.default_ignored_dirs().into_iter().collect(),
            respect_gitignore: false, // پیش‌فرض قطعاً خاموش
            deduplication: true,
            remove_comments: false,
            max_file_size_kb: 512,
            total_output_limit_mb: 10,
            ext_input_buffer: String::new(),
            ignore_input_buffer: String::new(),
            tree_search_query: String::new(),
        }
    }
}

impl AppConfig {
    pub fn set_preset(&mut self, preset: Preset) {
        self.preset = preset;
        if preset != Preset::Custom {
            self.allowed_extensions = preset.default_extensions().into_iter().collect();
            self.ignored_directories = preset.default_ignored_dirs().into_iter().collect();
        }
    }

    pub fn is_ext_allowed(&self, ext: &str) -> bool {
        self.allowed_extensions.contains(&ext.to_lowercase())
    }

    pub fn is_dir_ignored(&self, name: &str) -> bool {
        self.ignored_directories.contains(name)
    }
}