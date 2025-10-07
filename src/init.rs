use anyhow::{Context, Result};
use std::fs;
use std::path::PathBuf;

use crate::constants::{auth_required_msg, DEFAULT_BASE_URL};
use crate::download::{download_repo, process_tree};
use crate::status::get_stored_api_key;

pub async fn handle_init(repo_id: String, url: Option<String>, debug: bool) -> Result<()> {
    println!("Initializing project with repository ID: {}", repo_id);
    
    let api_key = get_stored_api_key()
        .context(auth_required_msg())?;
    
    let url_base = url.unwrap_or_else(|| DEFAULT_BASE_URL.to_string());
    
    println!("Downloading repository structure...");
    
    let download_data = download_repo(&repo_id, &url_base, &api_key, debug).await?;
    
    fs::create_dir_all(".verilib")
        .context("Failed to create .verilib directory")?;
    
    let metadata = serde_json::json!({
        "repo": {
            "id": repo_id,
            "url": url_base
        }
    });
    
    let metadata_json = serde_json::to_string_pretty(&metadata)
        .context("Failed to serialize metadata to JSON")?;
    
    fs::write(".verilib/metadata.json", &metadata_json)
        .context("Failed to write metadata.json file")?;
    
    println!("Creating files and folders...");
    
    let base_path = PathBuf::from(".verilib");
    process_tree(&download_data.data.tree, &base_path, &download_data.data.layouts)?;
    
    println!("Repository successfully initialized!");
    
    Ok(())
}
