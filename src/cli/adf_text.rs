//! Lightweight plain-text / markdown renderer for Atlassian Document Format (ADF) JSON.
//! Completely independent of UI and terminal rendering crates.

use serde_json::Value;

/// Convert an ADF JSON document or node into clean plain text / markdown.
pub fn render_adf_text(val: &Value) -> String {
    let mut out = String::new();
    render_node(val, 0, &mut out);
    out.trim().to_string()
}

fn render_node(node: &Value, indent: usize, out: &mut String) {
    let node_type = node.get("type").and_then(|t| t.as_str()).unwrap_or("");
    match node_type {
        "doc" => {
            if let Some(content) = node.get("content").and_then(|c| c.as_array()) {
                for child in content {
                    render_node(child, indent, out);
                }
            }
        }
        "paragraph" => {
            let prefix = "  ".repeat(indent);
            let inline_text = render_inlines(node);
            if !inline_text.is_empty() {
                out.push_str(&format!("{}{}\n\n", prefix, inline_text));
            }
        }
        "heading" => {
            let level = node
                .get("attrs")
                .and_then(|a| a.get("level"))
                .and_then(|l| l.as_u64())
                .unwrap_or(1) as usize;
            let level = level.clamp(1, 6);
            let hashes = "#".repeat(level);
            let text = render_inlines(node);
            out.push_str(&format!("{} {}\n\n", hashes, text.trim()));
        }
        "bulletList" => {
            if let Some(items) = node.get("content").and_then(|c| c.as_array()) {
                for item in items {
                    render_list_item(item, indent, "- ", out);
                }
                out.push('\n');
            }
        }
        "orderedList" => {
            if let Some(items) = node.get("content").and_then(|c| c.as_array()) {
                for (idx, item) in items.iter().enumerate() {
                    let bullet = format!("{}. ", idx + 1);
                    render_list_item(item, indent, &bullet, out);
                }
                out.push('\n');
            }
        }
        "codeBlock" => {
            let lang = node
                .get("attrs")
                .and_then(|a| a.get("language"))
                .and_then(|l| l.as_str())
                .unwrap_or("");
            let code = extract_plain_text(node);
            out.push_str(&format!("```{}\n{}\n```\n\n", lang, code.trim_end()));
        }
        "blockquote" => {
            let mut block_out = String::new();
            if let Some(content) = node.get("content").and_then(|c| c.as_array()) {
                for child in content {
                    render_node(child, 0, &mut block_out);
                }
            }
            for line in block_out.trim().lines() {
                out.push_str(&format!("> {}\n", line));
            }
            out.push('\n');
        }
        "rule" => {
            out.push_str("---\n\n");
        }
        "panel" => {
            let panel_type = node
                .get("attrs")
                .and_then(|a| a.get("panelType"))
                .and_then(|p| p.as_str())
                .unwrap_or("info");
            let mut inner = String::new();
            if let Some(content) = node.get("content").and_then(|c| c.as_array()) {
                for child in content {
                    render_node(child, 0, &mut inner);
                }
            }
            out.push_str(&format!("[{panel_type}]\n{}\n\n", inner.trim()));
        }
        "table" => {
            render_table(node, out);
        }
        _ => {
            // Fallback for container or unknown types with children
            if let Some(content) = node.get("content").and_then(|c| c.as_array()) {
                for child in content {
                    render_node(child, indent, out);
                }
            }
        }
    }
}

fn render_list_item(item: &Value, indent: usize, prefix: &str, out: &mut String) {
    let padding = "  ".repeat(indent);
    if let Some(content) = item.get("content").and_then(|c| c.as_array()) {
        for (i, child) in content.iter().enumerate() {
            if i == 0 && child.get("type").and_then(|t| t.as_str()) == Some("paragraph") {
                let text = render_inlines(child);
                out.push_str(&format!("{}{}{}\n", padding, prefix, text.trim()));
            } else {
                render_node(child, indent + 1, out);
            }
        }
    }
}

fn render_inlines(node: &Value) -> String {
    let mut text_acc = String::new();
    if let Some(content) = node.get("content").and_then(|c| c.as_array()) {
        for child in content {
            text_acc.push_str(&render_single_inline(child));
        }
    }
    text_acc
}

fn render_single_inline(node: &Value) -> String {
    let node_type = node.get("type").and_then(|t| t.as_str()).unwrap_or("");
    match node_type {
        "text" => {
            let raw_text = node.get("text").and_then(|t| t.as_str()).unwrap_or("");
            apply_marks(raw_text, node.get("marks").and_then(|m| m.as_array()))
        }
        "hardBreak" => "\n".to_string(),
        "mention" => {
            let label = node
                .get("attrs")
                .and_then(|a| a.get("text"))
                .and_then(|t| t.as_str())
                .filter(|s| !s.is_empty())
                .or_else(|| {
                    node.get("attrs")
                        .and_then(|a| a.get("id"))
                        .and_then(|t| t.as_str())
                })
                .unwrap_or("@user");
            label.to_string()
        }
        "emoji" => {
            let short = node
                .get("attrs")
                .and_then(|a| a.get("shortName"))
                .and_then(|s| s.as_str());
            node.get("text")
                .and_then(|t| t.as_str())
                .map(String::from)
                .or_else(|| short.map(|s| format!(":{s}:")))
                .unwrap_or_default()
        }
        "inlineCard" => {
            let url = node
                .get("attrs")
                .and_then(|a| a.get("url"))
                .and_then(|u| u.as_str())
                .unwrap_or("");
            url.to_string()
        }
        _ => String::new(),
    }
}

fn apply_marks(text: &str, marks: Option<&Vec<Value>>) -> String {
    let Some(marks) = marks else {
        return text.to_string();
    };
    let mut styled = text.to_string();
    let mut link_href: Option<&str> = None;

    for mark in marks {
        match mark.get("type").and_then(|t| t.as_str()) {
            Some("code") => styled = format!("`{styled}`"),
            Some("strong") => styled = format!("**{styled}**"),
            Some("em") => styled = format!("*{styled}*"),
            Some("strike") => styled = format!("~~{styled}~~"),
            Some("link") => {
                link_href = mark
                    .get("attrs")
                    .and_then(|a| a.get("href"))
                    .and_then(|h| h.as_str());
            }
            _ => {}
        }
    }

    if let Some(href) = link_href {
        styled = format!("{styled} ({href})");
    }

    styled
}

fn extract_plain_text(node: &Value) -> String {
    if let Some(t) = node.get("text").and_then(|s| s.as_str()) {
        return t.to_string();
    }
    let mut acc = String::new();
    if let Some(content) = node.get("content").and_then(|c| c.as_array()) {
        for child in content {
            acc.push_str(&extract_plain_text(child));
        }
    }
    acc
}

fn render_table(node: &Value, out: &mut String) {
    let Some(rows) = node.get("content").and_then(|c| c.as_array()) else {
        return;
    };
    for row in rows {
        if let Some(cells) = row.get("content").and_then(|c| c.as_array()) {
            let row_str = cells
                .iter()
                .map(|cell| {
                    let mut cell_buf = String::new();
                    if let Some(content) = cell.get("content").and_then(|c| c.as_array()) {
                        for p in content {
                            cell_buf.push_str(&render_inlines(p));
                        }
                    }
                    cell_buf.trim().to_string()
                })
                .collect::<Vec<_>>()
                .join(" | ");
            out.push_str(&format!("| {row_str} |\n"));
        }
    }
    out.push('\n');
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_render_paragraph_and_formatting() {
        let doc = json!({
            "type": "doc",
            "content": [
                {
                    "type": "paragraph",
                    "content": [
                        { "type": "text", "text": "Hello " },
                        { "type": "text", "text": "world", "marks": [{ "type": "strong" }] },
                        { "type": "text", "text": "!" }
                    ]
                }
            ]
        });
        assert_eq!(render_adf_text(&doc), "Hello **world**!");
    }

    #[test]
    fn test_render_headings_and_code() {
        let doc = json!({
            "type": "doc",
            "content": [
                {
                    "type": "heading",
                    "attrs": { "level": 2 },
                    "content": [{ "type": "text", "text": "Subheading" }]
                },
                {
                    "type": "codeBlock",
                    "attrs": { "language": "rust" },
                    "content": [{ "type": "text", "text": "fn main() {}" }]
                }
            ]
        });
        let result = render_adf_text(&doc);
        assert!(result.contains("## Subheading"));
        assert!(result.contains("```rust\nfn main() {}\n```"));
    }

    #[test]
    fn test_render_lists() {
        let doc = json!({
            "type": "doc",
            "content": [
                {
                    "type": "bulletList",
                    "content": [
                        {
                            "type": "listItem",
                            "content": [{
                                "type": "paragraph",
                                "content": [{ "type": "text", "text": "Item 1" }]
                            }]
                        },
                        {
                            "type": "listItem",
                            "content": [{
                                "type": "paragraph",
                                "content": [{ "type": "text", "text": "Item 2" }]
                            }]
                        }
                    ]
                }
            ]
        });
        let result = render_adf_text(&doc);
        assert!(result.contains("- Item 1"));
        assert!(result.contains("- Item 2"));
    }
}
