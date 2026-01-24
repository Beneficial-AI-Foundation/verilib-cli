use anyhow::{Context, Result};
use reqwest::Client;
use serde_json::Value;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::constants::{auth_required_msg, init_required_msg};
use crate::commands::status::get_stored_api_key;
use crate::commands::types::Config;
use crate::download::{handle_api_error, wait_for_atomization, download_repo, process_tree};

pub async fn handle_reclone(debug: bool) -> Result<()> {
    if debug {
        println!("Debug: Starting reclone process...");
    } else {
        println!("Starting reclone process...");
    }
    
    // Check if authentication exists
    get_stored_api_key()
        .context(auth_required_msg())?;
    
    // Check if project is initialized (.verilib/config.json exists)
    if !Path::new(".verilib/config.json").exists() {
        anyhow::bail!(init_required_msg());
    }

    // Read and parse config.json to get repo_id and url
    let config_content = fs::read_to_string(".verilib/config.json")
        .context("Failed to read .verilib/config.json")?;

    let config: Config = serde_json::from_str(&config_content)
        .context("Failed to parse config.json")?;

    let repo_id = config.repo.id.clone();
    let url_base = config.repo.url.clone();
    
    println!("Found repository ID: {}", repo_id);
    if debug {
        println!("Debug: Using URL: {}", url_base);
    }
    
    // Check if git is available
    if !is_git_available() {
        anyhow::bail!("Git is not found. Please install Git to use this command");
    }
    
    // Check for uncommitted changes
    if has_uncommitted_changes()? {
        println!("Warning: You have uncommitted changes in your git repository.");
        println!("Please commit or stash your changes before running reclone.");
        anyhow::bail!("Uncommitted changes detected");
    }
    
    // Perform the reclone API call
    let api_key = get_stored_api_key()?;
    let endpoint = format!("{}/v2/repo/reclone/{}", url_base, repo_id);
    
    println!("Calling reclone endpoint: {}", endpoint);
    
    let client = Client::new();
    let response = client
        .post(&endpoint)
        .header("Authorization", format!("ApiKey {}", api_key))
        .header("Accept", "application/json")
        .send()
        .await
        .context("Failed to send reclone request")?;
    
    let status = response.status();
    
    if debug {
        println!("Debug: Response status: {}", status);
    }
    
    if !status.is_success() {
        let error_msg = handle_api_error(response).await?;
        anyhow::bail!(error_msg);
    }
    
    let response_text = response
        .text()
        .await
        .context("Failed to read response body")?;
    
    if debug {
        println!("Debug: Raw response body:");
        println!("{}", response_text);
    }
    
    let json_response: Value = serde_json::from_str(&response_text)
        .context("Failed to parse JSON response")?;
    
    if debug {
        println!("Debug: Parsed JSON response:");
        println!("{}", serde_json::to_string_pretty(&json_response).unwrap_or_else(|_| "Failed to pretty print".to_string()));
    }
    
    // Check for success
    if let Some(status) = json_response.get("status") {
        if status == "success" {
            println!("Reclone completed successfully!");
            
            println!();
            wait_for_atomization(&repo_id, &url_base, &api_key).await?;
            
            println!("Atomization complete! Downloading latest data...");
            
            let download_data = download_repo(&repo_id, &url_base, &api_key, debug).await?;
            
            let verilib_path = PathBuf::from(".verilib");
            if verilib_path.exists() {
                println!("Cleaning existing .verilib directory...");
                fs::remove_dir_all(&verilib_path)
                    .context("Failed to remove existing .verilib directory")?;
            }
            
            fs::create_dir_all(".verilib")
                .context("Failed to create .verilib directory")?;
            
            let mut config = config;
            config.repo.is_admin = download_data.data.is_admin;

            let config_content = serde_json::to_string_pretty(&config)
                .context("Failed to serialize config to JSON")?;

            fs::write(".verilib/config.json", &config_content)
                .context("Failed to write config.json file")?;
            
            println!("Creating files and folders...");
            
            let base_path = PathBuf::from(".verilib");
            process_tree(&download_data.data.tree, &base_path, &download_data.data.layouts)?;
            
            println!("Repository successfully updated!");
            
            return Ok(());
        }
    }
    
    anyhow::bail!("Unexpected response format from reclone API");
}

fn is_git_available() -> bool {
    Command::new("git")
        .arg("--version")
        .output()
        .is_ok()
}

fn has_uncommitted_changes() -> Result<bool> {
    let output = Command::new("git")
        .args(["status", "--porcelain"])
        .output()
        .context("Failed to run git status")?;
    
    if !output.status.success() {
        anyhow::bail!("Git status command failed. Make sure you're in a git repository");
    }
    
    // If git status --porcelain returns any output, there are uncommitted changes
    Ok(!output.stdout.is_empty())
}
