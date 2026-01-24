//! YAML frontmatter parsing and writing for markdown files.

use anyhow::{bail, Context, Result};
use serde_json::Value;
use std::collections::HashMap;
use std::path::Path;

/// Parse YAML frontmatter from a markdown file.
pub fn parse(path: &Path) -> Result<HashMap<String, Value>> {
    let content = std::fs::read_to_string(path)?;
    let mut lines = content.lines();

    // Check for opening ---
    match lines.next() {
        Some("---") => {}
        _ => bail!("No frontmatter found"),
    }

    // Collect frontmatter lines until closing ---
    let mut yaml_lines = Vec::new();
    for line in lines {
        if line == "---" {
            break;
        }
        yaml_lines.push(line);
    }

    let yaml_content = yaml_lines.join("\n");
    let frontmatter: HashMap<String, Value> =
        serde_yaml::from_str(&yaml_content).context("Failed to parse YAML frontmatter")?;

    Ok(frontmatter)
}

/// Write a markdown file with YAML frontmatter.
pub fn write(path: &Path, metadata: &HashMap<String, Value>, body: Option<&str>) -> Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    let mut content = String::new();
    content.push_str("---\n");

    for (key, value) in metadata {
        let formatted = format_value(value)?;
        content.push_str(&format!("{}: {}\n", key, formatted));
    }

    content.push_str("---\n");
    content.push('\n');

    if let Some(body_content) = body {
        content.push_str(body_content);
        content.push('\n');
    }

    std::fs::write(path, content)?;
    Ok(())
}

/// Format a JSON value as a YAML scalar.
fn format_value(value: &Value) -> Result<String> {
    match value {
        Value::Null => Ok("null".to_string()),
        Value::Bool(b) => Ok(if *b { "true" } else { "false" }.to_string()),
        Value::Number(n) => Ok(n.to_string()),
        Value::String(s) => {
            // Check if string needs quoting
            if s.is_empty()
                || s == "null"
                || s == "true"
                || s == "false"
                || s == "~"
                || s.starts_with('{')
                || s.starts_with('[')
                || s.starts_with('\'')
                || s.starts_with('"')
                || s.starts_with('|')
                || s.starts_with('>')
                || s.starts_with('*')
                || s.starts_with('&')
                || s.starts_with('!')
                || s.contains(':')
                || s.contains('#')
                || s.contains('\n')
            {
                let escaped = s
                    .replace('\\', "\\\\")
                    .replace('"', "\\\"")
                    .replace('\n', "\\n");
                Ok(format!("\"{}\"", escaped))
            } else {
                Ok(s.clone())
            }
        }
        Value::Array(arr) => {
            let items: Result<Vec<String>> = arr.iter().map(format_value).collect();
            Ok(format!("[{}]", items?.join(", ")))
        }
        Value::Object(_) => bail!("Nested objects are not supported in metadata"),
    }
}
