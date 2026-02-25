use anyhow::{Context, Result};
use reqwest::Client;
use serde_json::Value;
use std::path::PathBuf;
use std::process::Command;

use crate::constants::{auth_required_msg, init_required_msg};
use crate::commands::status::get_stored_api_key;
use crate::config::ProjectConfig;
use crate::download::handle_api_error;

pub async fn handle_reclone(debug: bool) -> Result<()> {
    if debug {
        println!("Debug: Starting reclone process...");
    } else {
        println!("Starting reclone process...");
    }
    
    // Check if authentication exists
    get_stored_api_key()
        .context(auth_required_msg())?;
    
    let project_root = PathBuf::from(".");
    let config = ProjectConfig::load(&project_root)?;

    let repo = config.repo.ok_or_else(|| anyhow::anyhow!(init_required_msg()))?;

    let repo_id = repo.id;
    let url_base = repo.url;
    
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

    // Check for unpushed commits
    if has_unpushed_commits()? {
        println!("Warning: You have unpushed commits in your git repository.");
        println!("Please push your changes before running reclone.");
        anyhow::bail!("Unpushed commits detected");
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
    
    if let Some(status) = json_response.get("status") {
        if status == "success" {
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

fn has_unpushed_commits() -> Result<bool> {
    let output = Command::new("git")
        .args(["rev-list", "--count", "@{u}..HEAD"])
        .output()
        .context("Failed to run git rev-list to check pushed status")?;
    
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        if stderr.contains("no upstream configured") {
             // Fallback: Check if the current HEAD commit exists on any remote branch
             let branch_output = Command::new("git")
                 .args(["branch", "-r", "--contains", "HEAD"])
                 .output()
                 .context("Failed to check remote branches")?;
             
             // If output has content, commit is on some remote (safe)
             return Ok(branch_output.stdout.is_empty());
        }
        anyhow::bail!("Git command failed: {}", stderr.trim());
    }
    
    let count = String::from_utf8_lossy(&output.stdout)
        .trim()
        .parse::<usize>()
        .unwrap_or(0);
        
    Ok(count > 0)
}
