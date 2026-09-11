use anyhow::{Context, Result};
use std::fs;
use std::path::PathBuf;

use crate::commands::status::get_stored_api_key;
use crate::config::{ProjectConfig, RepoConfig};
use crate::constants::{auth_required_msg, init_required_msg, resolve_base_url};
use crate::download::{download_repo, process_tree};

pub async fn handle_pull(url: Option<String>, debug: bool) -> Result<()> {
    let api_key = get_stored_api_key().context(auth_required_msg())?;

    let project_root = PathBuf::from(".");
    let existing_config = ProjectConfig::load(&project_root)?;

    let repo = existing_config
        .repo
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!(init_required_msg()))?;

    let repo_id = repo.id.clone();
    let config_url = repo.url.as_str();
    let url_base = resolve_base_url(url, Some(config_url));

    println!("Pulling repository ID: {}", repo_id);
    println!("Downloading repository structure from {}...", url_base);

    let download_data = download_repo(&repo_id, &url_base, &api_key, debug).await?;

    let verilib_path = PathBuf::from(".verilib");
    if verilib_path.exists() {
        println!("Cleaning existing .verilib directory...");
        fs::remove_dir_all(&verilib_path)
            .context("Failed to remove existing .verilib directory")?;
    }

    fs::create_dir_all(".verilib").context("Failed to create .verilib directory")?;

    let mut config = existing_config;
    config.repo = Some(RepoConfig {
        id: repo_id,
        url: url_base.clone(),
        is_admin: download_data.data.is_admin,
    });
    config.save(&project_root)?;

    println!("Creating files and folders...");

    let base_path = PathBuf::from(".verilib");
    process_tree(&download_data.data.tree, &base_path, &download_data.data.layouts)?;

    println!("Repository successfully pulled!");

    Ok(())
}
