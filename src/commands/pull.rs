use anyhow::{Context, Result};
use std::fs;
use std::path::PathBuf;

use crate::constants::auth_required_msg;
use crate::download::{download_repo, process_tree};
use crate::commands::status::get_stored_api_key;
use crate::commands::types::Metadata;

pub async fn handle_pull(debug: bool) -> Result<()> {
    let api_key = get_stored_api_key()
        .context(auth_required_msg())?;
    
    let metadata_path = PathBuf::from(".verilib/metadata.json");
    
    if !metadata_path.exists() {
        anyhow::bail!("No metadata.json found. Please run 'init' first.");
    }
    
    let metadata_content = fs::read_to_string(&metadata_path)
        .context("Failed to read metadata.json")?;
    
    let metadata: Metadata = serde_json::from_str(&metadata_content)
        .context("Failed to parse metadata.json")?;
    
    let repo_id = metadata.repo.id;
    let url_base = metadata.repo.url;
    
    println!("Pulling repository ID: {}", repo_id);
    println!("Downloading repository structure...");
    
    let download_data = download_repo(&repo_id, &url_base, &api_key, debug).await?;
    
    let verilib_path = PathBuf::from(".verilib");
    if verilib_path.exists() {
        println!("Cleaning existing .verilib directory...");
        fs::remove_dir_all(&verilib_path)
            .context("Failed to remove existing .verilib directory")?;
    }
    
    fs::create_dir_all(".verilib")
        .context("Failed to create .verilib directory")?;
    
    let metadata = Metadata {
        repo: crate::commands::types::RepoMetadata {
            id: repo_id.clone(),
            url: url_base.clone(),
            is_admin: download_data.data.is_admin,
        },
    };
    
    let metadata_json = serde_json::to_string_pretty(&metadata)
        .context("Failed to serialize metadata to JSON")?;
    
    fs::write(".verilib/metadata.json", &metadata_json)
        .context("Failed to write metadata.json file")?;
    
    println!("Creating files and folders...");
    
    let base_path = PathBuf::from(".verilib");
    process_tree(&download_data.data.tree, &base_path, &download_data.data.layouts)?;
    
    println!("Repository successfully pulled!");
    
    Ok(())
}
