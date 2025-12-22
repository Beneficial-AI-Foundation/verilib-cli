use anyhow::{Context, Result};
use std::fs;
use std::path::PathBuf;
use sha2::{Sha256, Digest};

use super::types::{TreeNode, Layout};

pub fn process_tree(nodes: &[TreeNode], base_path: &PathBuf, layouts: &std::collections::HashMap<String, Layout>) -> Result<()> {
    for node in nodes {
        process_node(node, base_path, layouts)?;
    }
    Ok(())
}

fn process_node(node: &TreeNode, current_path: &PathBuf, layouts: &std::collections::HashMap<String, Layout>) -> Result<()> {
    match node.statement_type.as_str() {
        "folder" | "file" | "molecule" => {
            let dir_path = current_path.join(&node.identifier);
            fs::create_dir_all(&dir_path)
                .with_context(|| format!("Failed to create directory: {:?}", dir_path))?;
            
            if let Some(layout) = layouts.get(&node.path) {
                let layout_path = dir_path.join("layout.verilib");
                let layout_json = serde_json::to_string_pretty(&layout.nodes)
                    .with_context(|| format!("Failed to serialize layout for node: {}", node.id))?;
                fs::write(&layout_path, layout_json)
                    .with_context(|| format!("Failed to write layout file: {:?}", layout_path))?;
            }
            
            process_tree(&node.children, &dir_path, layouts)?;
        }
        _ => {
            let file_name = format!("[{}] - {}.atom.verilib", node.index, node.identifier);
            let file_path = current_path.join(&file_name);
            
            let content = node.snippets
                .iter()
                .map(|s| s.text.as_str())
                .collect::<Vec<&str>>()
                .join("");
            
            if content.is_empty() {
                eprintln!("Warning: Empty content for {} (snippets count: {})", node.identifier, node.snippets.len());
            }
            
            let mut hasher = Sha256::new();
            hasher.update(&content);
            let hash_result = hasher.finalize();
            let fingerprint = format!("{:x}", hash_result);
            
            fs::write(&file_path, content)
                .with_context(|| format!("Failed to write file: {:?}", file_path))?;
            
            let meta_file_name = format!("[{}] - {}.meta.verilib", node.index, node.identifier);
            let meta_file_path = current_path.join(&meta_file_name);
            
            let meta_data = serde_json::json!({
                "id": node.id,
                "parent_id": node.parent_id,
                "identifier": node.identifier,
                "index": node.index,
                "statement_type": node.statement_type,
                "status_id": node.status_id,
                "specified": node.specified,
                "path": node.path,
                "dependencies": node.dependencies,
                "code_name": node.code_name,
                "disabled": node.disabled,
                "fingerprint": fingerprint,
                "snippets": node.snippets
            });
            
            let meta_json = serde_json::to_string_pretty(&meta_data)
                .with_context(|| format!("Failed to serialize metadata for node: {}", node.id))?;
            
            fs::write(&meta_file_path, meta_json)
                .with_context(|| format!("Failed to write meta file: {:?}", meta_file_path))?;
        }
    }
    
    Ok(())
}
