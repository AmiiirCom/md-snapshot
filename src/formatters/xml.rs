use super::SnapshotPayload;

pub fn render(p: &SnapshotPayload) -> String {
    let mut out = String::with_capacity(128 * 1024);
    out.push_str("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<codebase_snapshot>\n");

    if !p.instruction_docs.is_empty() {
        out.push_str("  <instructions>\n");
        for (name, content) in &p.instruction_docs {
            out.push_str(&format!("    <doc name=\"{}\">\n", escape_xml_attr(name)));
            out.push_str(&format_cdata(content));
            out.push_str("\n    </doc>\n");
        }
        out.push_str("  </instructions>\n");
    }

    out.push_str("  <directory_tree>\n");
    out.push_str(&escape_xml_text(&p.tree_structure));
    out.push_str("\n  </directory_tree>\n");

    if !p.omitted_files.is_empty() {
        out.push_str("  <omitted_files>\n");
        for (path, reason) in &p.omitted_files {
            out.push_str(&format!(
                "    <omitted path=\"{}\" reason=\"{}\" />\n",
                escape_xml_attr(path),
                escape_xml_attr(reason)
            ));
        }
        out.push_str("  </omitted_files>\n");
    }

    out.push_str("  <files>\n");
    for file in &p.files {
        if let Some(ref original) = file.is_duplicate_of {
            out.push_str(&format!(
                "    <file path=\"{}\" duplicate_of=\"{}\" />\n",
                escape_xml_attr(&file.relative_path),
                escape_xml_attr(original)
            ));
        } else {
            out.push_str(&format!("    <file path=\"{}\">\n", escape_xml_attr(&file.relative_path)));
            out.push_str(&format_cdata(&file.content));
            out.push_str("\n    </file>\n");
        }
    }
    out.push_str("  </files>\n</codebase_snapshot>");
    out
}

fn format_cdata(text: &str) -> String {
    let safe_text = text.replace("]]>", "]]]]><![CDATA[>");
    format!("<![CDATA[\n{}\n]]>", safe_text)
}

fn escape_xml_attr(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('"', "&quot;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

fn escape_xml_text(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}