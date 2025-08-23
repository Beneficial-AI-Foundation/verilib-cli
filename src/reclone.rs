use anyhow::{Context, Result};
use reqwest::Client;
use serde_json::Value;
use std::fs;
use std::path::Path;
use std::process::Command;

use crate::constants::{auth_required_msg, init_required_msg, DEFAULT_BASE_URL};
use crate::status::get_stored_api_key;

pub async fn handle_reclone(base_url: Option<String>, debug: bool) -> Result<()> {
    if debug {
        println!("Debug: Starting reclone process...");
    } else {
        println!("Starting reclone process...");
    }
    
    // Check if authentication exists
    get_stored_api_key()
        .context(auth_required_msg())?;
    
    // Check if project is initialized (.verilib/tree.json exists)
    if !Path::new(".verilib/tree.json").exists() {
        anyhow::bail!(init_required_msg());
    }
    
    // Read and parse tree.json to get repo_id
    let tree_content = fs::read_to_string(".verilib/tree.json")
        .context("Failed to read .verilib/tree.json")?;
    
    let tree_json: Value = serde_json::from_str(&tree_content)
        .context("Failed to parse tree.json")?;
    
    // Extract repo_id from tree.json (assuming it's in the root or we can find it)
    let repo_id = extract_repo_id(&tree_json)
        .context("Could not find repository ID in tree.json")?;
    
    println!("Found repository ID: {}", repo_id);
    
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
    let url_base = base_url.unwrap_or_else(|| DEFAULT_BASE_URL.to_string());
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
    
    if debug {
        println!("Debug: Response status: {}", response.status());
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
    
    // Check response format
    if let Some(status) = json_response.get("status") {
        if debug {
            println!("Debug: Found top-level status: {}", status);
        }
        if status == "error" {
            if let Some(data) = json_response.get("data") {
                if debug {
                    println!("Debug: Error data: {}", data);
                }
                if let Some(code) = data.get("code") {
                    anyhow::bail!("Reclone failed with error code: {}", code);
                }
            }
            anyhow::bail!("Reclone failed with unknown error");
        }
    } else if debug {
        println!("Debug: No top-level 'status' field found");
    }
    
    // Check for success in data.status
    if let Some(status) = json_response.get("status") {
        if debug {
            println!("Debug: Found data.status: {}", status);
        }
        if status == "success" {
            println!("Reclone completed successfully!");
            return Ok(());
        } else if debug {
            println!("Debug: data.status is not 'success': {}", status);
        }
    } else if debug {
        println!("Debug: No 'status' field found in data object");
    }

    
    anyhow::bail!("Unexpected response format from reclone API. See debug output above for details.");
}

fn extract_repo_id(tree_json: &Value) -> Option<String> {
    // Extract repo.id from the tree.json structure
    tree_json
        .get("repo")?
        .get("id")?
        .as_str()
        .map(|s| s.to_string())
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
