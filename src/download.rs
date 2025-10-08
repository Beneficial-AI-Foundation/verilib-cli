use anyhow::{Context, Result};
use reqwest::Client;
use serde::{Deserialize, Deserializer, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Deserialize, Serialize)]
pub struct TreeNode {
    pub id: u64,
    pub parent_id: Option<u64>,
    pub identifier: String,
    pub index: u32,
    pub statement_type: String,
    pub status_id: u32,
    pub specified: u32,
    pub path: String,
    pub snippets: Vec<Snippet>,
    #[serde(default)]
    pub children: Vec<TreeNode>,
    #[serde(default)]
    pub dependencies: Vec<String>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Snippet {
    pub type_id: u32,
    pub text: String,
    pub sortorder: u32,
}

#[derive(Debug, Deserialize)]
pub struct DownloadResponse {
    pub data: DownloadData,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct LayoutNode {
    pub identifier: String,
    pub id: String,
    pub fx: f64,
    pub fy: f64,
    pub path: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Layout {
    pub nodes: Vec<LayoutNode>,
    #[serde(default)]
    pub zoom: Option<serde_json::Value>,
    #[serde(default)]
    pub repositioned: Option<bool>,
}

fn deserialize_layouts<'de, D>(deserializer: D) -> Result<std::collections::HashMap<String, Layout>, D::Error>
where
    D: Deserializer<'de>,
{
    use serde::de::Error;
    use serde_json::Value;
    
    let value = Value::deserialize(deserializer)?;
    
    match value {
        Value::Array(arr) if arr.is_empty() => Ok(std::collections::HashMap::new()),
        Value::Object(_) => serde_json::from_value(value).map_err(Error::custom),
        _ => Err(Error::custom("expected an object or empty array for layouts")),
    }
}

#[derive(Debug, Deserialize)]
pub struct DownloadData {
    pub repo: RepoInfo,
    pub tree: Vec<TreeNode>,
    #[serde(deserialize_with = "deserialize_layouts")]
    pub layouts: std::collections::HashMap<String, Layout>,
}

#[derive(Debug, Deserialize)]
pub struct RepoInfo {
    pub id: String,
}

pub async fn download_repo(
    repo_id: &str,
    base_url: &str,
    api_key: &str,
    debug: bool,
) -> Result<DownloadResponse> {
    let endpoint = format!("{}/v2/repo/download/{}", base_url, repo_id);
    
    let client = Client::new();
    let response = client
        .get(&endpoint)
        .header("Authorization", format!("ApiKey {}", api_key))
        .header("Accept", "application/json")
        .send()
        .await
        .context("Failed to send request to API")?;
    
    if !response.status().is_success() {
        anyhow::bail!(
            "API request failed with status: {} - {}",
            response.status(),
            response.text().await.unwrap_or_else(|_| "Unable to read error response".to_string())
        );
    }
    
    let response_text = response
        .text()
        .await
        .context("Failed to read response body")?;
    
    if debug {
        fs::create_dir_all(".verilib")
            .context("Failed to create .verilib directory for debug output")?;
        fs::write(".verilib/debug_response.json", &response_text)
            .context("Failed to write debug response file")?;
        println!("Debug: API response saved to .verilib/debug_response.json");
    }
    
    let download_data: DownloadResponse = serde_json::from_str(&response_text)
        .context("Failed to parse JSON response")?;
    
    Ok(download_data)
}

pub fn process_tree(nodes: &[TreeNode], base_path: &PathBuf, layouts: &std::collections::HashMap<String, Layout>) -> Result<()> {
    for node in nodes {
        process_node(node, base_path, layouts)?;
    }
    Ok(())
}

fn process_node(node: &TreeNode, current_path: &PathBuf, layouts: &std::collections::HashMap<String, Layout>) -> Result<()> {
    match node.statement_type.as_str() {
        "folder" | "file" => {
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
            
            fs::write(&file_path, content)
                .with_context(|| format!("Failed to write file: {:?}", file_path))?;
            
            if !node.dependencies.is_empty() {
                let meta_file_name = format!("[{}] - {}.meta.verilib", node.index, node.identifier);
                let meta_file_path = current_path.join(&meta_file_name);
                
                let meta_data = serde_json::json!({
                    "dependencies": node.dependencies
                });
                
                let meta_json = serde_json::to_string_pretty(&meta_data)
                    .with_context(|| format!("Failed to serialize dependencies for node: {}", node.id))?;
                
                fs::write(&meta_file_path, meta_json)
                    .with_context(|| format!("Failed to write meta file: {:?}", meta_file_path))?;
            }
        }
    }
    
    Ok(())
}
