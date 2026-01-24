use anyhow::{Context, Result};
use std::fs;
use std::path::PathBuf;

use crate::constants::auth_required_msg;
use crate::download::{download_repo, process_tree};
use crate::commands::status::get_stored_api_key;
use crate::commands::types::Config;

pub async fn handle_pull(debug: bool) -> Result<()> {
    let api_key = get_stored_api_key()
        .context(auth_required_msg())?;
    
    let config_path = PathBuf::from(".verilib/config.json");

    if !config_path.exists() {
        anyhow::bail!("No config.json found. Please run 'init' first.");
    }

    let config_content = fs::read_to_string(&config_path)
        .context("Failed to read config.json")?;

    let config: Config = serde_json::from_str(&config_content)
        .context("Failed to parse config.json")?;
    
    let repo_id = config.repo.id;
    let url_base = config.repo.url;
    
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
    
    let config = Config {
        repo: crate::commands::types::RepoConfig {
            id: repo_id.clone(),
            url: url_base.clone(),
            is_admin: download_data.data.is_admin,
        },
    };
    
    let config_json = serde_json::to_string_pretty(&config)
        .context("Failed to serialize config to JSON")?;

    fs::write(".verilib/config.json", &config_json)
        .context("Failed to write config.json file")?;
    
    println!("Creating files and folders...");
    
    let base_path = PathBuf::from(".verilib");
    process_tree(&download_data.data.tree, &base_path, &download_data.data.layouts)?;
    
    println!("Repository successfully pulled!");
    
    Ok(())
}
