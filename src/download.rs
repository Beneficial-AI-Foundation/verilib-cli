use anyhow::{Context, Result};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Deserialize, Serialize)]
pub struct TreeNode {
    pub id: u64,
    pub parent_id: Option<u64>,
    pub identifier: String,
    pub statement_type: String,
    pub status_id: u32,
    pub specified: u32,
    pub snippets: Vec<Snippet>,
    #[serde(default)]
    pub children: Vec<TreeNode>,
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

#[derive(Debug, Deserialize)]
pub struct DownloadData {
    pub tree: Vec<TreeNode>,
}



pub async fn download_repo(
    repo_id: &str,
    base_url: &str,
    api_key: &str,
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
    
    let download_data: DownloadResponse = response
        .json()
        .await
        .context("Failed to parse JSON response")?;
    
    Ok(download_data)
}

pub fn process_tree(nodes: &[TreeNode], base_path: &PathBuf) -> Result<()> {
    for node in nodes {
        process_node(node, base_path)?;
    }
    Ok(())
}

fn process_node(node: &TreeNode, current_path: &PathBuf) -> Result<()> {
    match node.statement_type.as_str() {
        "folder" | "file" => {
            let dir_path = current_path.join(&node.identifier);
            fs::create_dir_all(&dir_path)
                .with_context(|| format!("Failed to create directory: {:?}", dir_path))?;
            
            process_tree(&node.children, &dir_path)?;
        }
        _ => {
            let file_name = format!("{}.atom.verilib", node.identifier);
            let file_path = current_path.join(&file_name);
            
            let content = node.snippets
                .iter()
                .map(|s| s.text.as_str())
                .collect::<Vec<&str>>()
                .join("");
            
            if content.is_empty() {
                eprintln!("Warning: Empty content for {}", node.identifier);
            }
            
            fs::write(&file_path, content)
                .with_context(|| format!("Failed to write file: {:?}", file_path))?;
        }
    }
    
    Ok(())
}
