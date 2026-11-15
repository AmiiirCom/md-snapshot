pub mod markdown;
pub mod xml;
pub mod yaml;

use crate::config::OutputFormat;

#[derive(Clone, Debug)]
pub struct SnapshotFile {
    pub relative_path: String,
    pub content: String,
    pub is_duplicate_of: Option<String>,
}

#[derive(Clone, Debug)]
pub struct SnapshotPayload {
    pub tree_structure: String,
    pub instruction_docs: Vec<(String, String)>,
    pub files: Vec<SnapshotFile>,
    pub omitted_files: Vec<(String, String)>,
}

pub fn format_snapshot(payload: &SnapshotPayload, format: OutputFormat) -> String {
    match format {
        OutputFormat::Markdown => markdown::render(payload),
        OutputFormat::Xml => xml::render(payload),
        OutputFormat::Yaml => yaml::render(payload),
    }
}