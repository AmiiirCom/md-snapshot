use super::SnapshotPayload;

pub fn render(p: &SnapshotPayload) -> String {
    let mut out = String::with_capacity(128 * 1024);

    if !p.instruction_docs.is_empty() {
        out.push_str("# Context & Custom Instructions\n\n");
        for (name, content) in &p.instruction_docs {
            out.push_str(&format!("## Document: `{}`\n\n{}\n\n---\n\n", name, content));
        }
    }

    out.push_str("# Project File Structure\n\n```text\n");
    out.push_str(&p.tree_structure);
    out.push_str("\n```\n\n---\n\n# Codebase Files\n\n");

    for file in &p.files {
        out.push_str(&format!("## File: `{}`\n\n", file.relative_path));

        if let Some(ref original) = file.is_duplicate_of {
            out.push_str(&format!(
                "> *Note: Identical content detected via SHA-256 deduplication. See `{}`.*\n\n",
                original
            ));
            continue;
        }

        let fence = calculate_adaptive_fence(&file.content);
        let lang = detect_lang(&file.relative_path);

        out.push_str(&fence);
        out.push_str(lang);
        out.push('\n');
        out.push_str(&file.content);
        if !file.content.ends_with('\n') {
            out.push('\n');
        }
        out.push_str(&fence);
        out.push_str("\n\n");
    }

    if !p.omitted_files.is_empty() {
        out.push_str("# Omitted / Skipped Files\n\n| File Path | Reason |\n|---|---|\n");
        for (path, reason) in &p.omitted_files {
            out.push_str(&format!("| `{}` | {} |\n", path, reason));
        }
        out.push('\n');
    }

    out
}

fn calculate_adaptive_fence(content: &str) -> String {
    let mut max_backticks = 2;
    let mut current = 0;
    for ch in content.chars() {
        if ch == '`' {
            current += 1;
            if current > max_backticks {
                max_backticks = current;
            }
        } else {
            current = 0;
        }
    }
    "`".repeat(max_backticks + 1)
}

fn detect_lang(path: &str) -> &'static str {
    match path.split('.').last().unwrap_or("").to_lowercase().as_str() {
        "rs" => "rust",
        "ts" | "tsx" => "typescript",
        "js" | "jsx" => "javascript",
        "py" => "python",
        "go" => "go",
        "toml" => "toml",
        "json" => "json",
        "yaml" | "yml" => "yaml",
        "html" => "html",
        "css" => "css",
        "sql" => "sql",
        "sh" | "bash" => "bash",
        "c" | "h" => "c",
        "cpp" | "hpp" => "cpp",
        "md" => "markdown",
        _ => "text",
    }
}