use super::SnapshotPayload;
use std::collections::BTreeMap;

pub fn render(p: &SnapshotPayload) -> String {
    let mut root = BTreeMap::new();

    if !p.instruction_docs.is_empty() {
        let mut docs = BTreeMap::new();
        for (name, content) in &p.instruction_docs {
            docs.insert(name.as_str(), content.as_str());
        }
        root.insert("instructions", serde_yaml::to_value(docs).unwrap());
    }

    root.insert("file_tree", serde_yaml::Value::String(p.tree_structure.clone()));

    let mut files_list = Vec::new();
    for file in &p.files {
        let mut item = BTreeMap::new();
        item.insert("path", serde_yaml::Value::String(file.relative_path.clone()));

        if let Some(ref dup) = file.is_duplicate_of {
            item.insert("duplicate_of", serde_yaml::Value::String(dup.clone()));
        } else {
            item.insert("content", serde_yaml::Value::String(file.content.clone()));
        }
        files_list.push(item);
    }
    root.insert("files", serde_yaml::to_value(files_list).unwrap());

    if !p.omitted_files.is_empty() {
        let mut omitted = BTreeMap::new();
        for (path, reason) in &p.omitted_files {
            omitted.insert(path.as_str(), reason.as_str());
        }
        root.insert("omitted_files", serde_yaml::to_value(omitted).unwrap());
    }

    serde_yaml::to_string(&root).unwrap_or_else(|_| "# Error rendering YAML".to_string())
}