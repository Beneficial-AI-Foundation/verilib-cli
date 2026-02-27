use anyhow::{Context, Result};
use dialoguer::{Input, Select};
use reqwest::Client;
use serde_json::Value;
use std::fs;
use std::path::PathBuf;
use std::process::Command;

use crate::commands::deploy::collect_deploy_info_with_path;
use crate::commands::status::get_stored_api_key;
use crate::constants::{auth_required_msg, DEFAULT_BASE_URL};
use crate::download::handle_api_error;
use crate::structure::{create_gitignore, ExecutionMode};

#[derive(serde::Deserialize, Debug)]
struct CreateRepoResponse {
    data: CreateRepoData,
}

#[derive(serde::Deserialize, Debug)]
struct CreateRepoData {
    id: u32,
}

pub async fn handle_init(id: Option<String>, url: Option<String>, debug: bool) -> Result<()> {
    let api_key = get_stored_api_key().context(auth_required_msg())?;

    let url_base = url.unwrap_or_else(|| DEFAULT_BASE_URL.to_string());

    let repo_id = if let Some(repo_id) = id {
        println!("Initializing project with repository ID: {}", repo_id);
        repo_id
    } else {
        let git_url = prompt_git_url()?;

        println!("Creating new repository from git URL: {}", git_url);

        let repo_id = create_repo_from_git_url(&git_url, &url_base, &api_key, debug).await?;

        println!("Repository created successfully!");
        println!("Repository ID: {}", repo_id);

        repo_id
    };

    let execution_mode = prompt_execution_mode()?;

    fs::create_dir_all(".verilib").context("Failed to create .verilib directory")?;

    save_config(&repo_id, &url_base, true, execution_mode)?;

    Ok(())
}

fn prompt_execution_mode() -> Result<ExecutionMode> {
    let modes = vec!["Local (Default)", "Docker"];
    let selection = Select::new()
        .with_prompt("Select execution mode")
        .items(&modes)
        .default(0)
        .interact()
        .context("Failed to select execution mode")?;

    match selection {
        0 => Ok(ExecutionMode::Local),
        1 => Ok(ExecutionMode::Docker),
        _ => unreachable!(),
    }
}

fn detect_git_url() -> Option<String> {
    let output = Command::new("git")
        .args(["config", "--get", "remote.origin.url"])
        .output()
        .ok()?;

    if output.status.success() {
        let url = String::from_utf8(output.stdout).ok()?;
        let url = url.trim().to_string();
        if !url.is_empty() {
            let mut normalized = normalize_git_url(&url);

            // Get current branch
            if let Ok(branch_output) = Command::new("git")
                .args(["rev-parse", "--abbrev-ref", "HEAD"])
                .output()
            {
                if branch_output.status.success() {
                    if let Ok(branch) = String::from_utf8(branch_output.stdout) {
                        let branch = branch.trim();
                        if !branch.is_empty() && branch != "HEAD" {
                            normalized.push('@');
                            normalized.push_str(branch);
                        }
                    }
                }
            }

            return Some(normalized);
        }
    }

    None
}

fn normalize_git_url(url: &str) -> String {
    // Convert SSH format (git@github.com:user/repo.git) to HTTPS (https://github.com/user/repo)
    if url.starts_with("git@") {
        // Extract host and path from git@host:path format
        if let Some(at_pos) = url.find('@') {
            if let Some(colon_pos) = url.find(':') {
                if colon_pos > at_pos {
                    let host = &url[at_pos + 1..colon_pos];
                    let path = &url[colon_pos + 1..];
                    // Remove .git suffix if present
                    let path = path.strip_suffix(".git").unwrap_or(path);
                    return format!("https://{}/{}", host, path);
                }
            }
        }
    }

    // If already HTTPS, just remove .git suffix if present
    if url.starts_with("https://") || url.starts_with("http://") {
        return url.strip_suffix(".git").unwrap_or(url).to_string();
    }

    // Return as-is if we can't parse it
    url.to_string()
}

fn prompt_git_url() -> Result<String> {
    println!("\nRepository URL Options:");
    println!("• Full repository: https://github.com/user/repo");
    println!("• Specific branch: https://github.com/user/repo@branch-name");
    println!("• Folder only: https://github.com/user/repo/tree/main/folder-name");
    println!(
        "• Folder from branch: https://github.com/user/repo/tree/main/folder-name@branch-name"
    );
    println!();

    let detected_url = detect_git_url();

    let git_url = if let Some(default_url) = detected_url {
        Input::<String>::new()
            .with_prompt("Enter repository URL")
            .default(default_url)
            .interact_text()
            .context("Failed to get git URL input")?
    } else {
        Input::<String>::new()
            .with_prompt("Enter repository URL")
            .interact_text()
            .context("Failed to get git URL input")?
    };

    let git_url = git_url.trim().to_string();

    if git_url.is_empty() {
        anyhow::bail!("Repository URL cannot be empty");
    }

    Ok(git_url)
}

async fn create_repo_from_git_url(
    git_url: &str,
    base_url: &str,
    api_key: &str,
    debug: bool,
) -> Result<String> {
    println!("\nCollecting repository information...");

    let (language_id, proof_id, verifierversion_id, summary, description, type_id) =
        collect_deploy_info_with_path(base_url, api_key, &PathBuf::from("."), debug).await?;

    let mut payload = serde_json::json!({
        "url": git_url,
        "language_id": language_id,
        "prooflanguage_id": proof_id,
        "summary": summary,
        "type_id": type_id,
    });

    if let Some(desc) = description {
        payload["description"] = Value::String(desc);
    }

    if let Some(version_id) = verifierversion_id {
        payload["verifierversion_id"] = Value::Number(version_id.into());
    }

    let endpoint = format!("{}/v2/repo/create", base_url);

    let client = Client::new();
    let response = client
        .post(&endpoint)
        .header("Authorization", format!("ApiKey {}", api_key))
        .header("Content-Type", "application/json")
        .json(&payload)
        .send()
        .await
        .context("Failed to send create repository request")?;

    let status = response.status();

    if !status.is_success() {
        let error_msg = handle_api_error(response).await?;
        anyhow::bail!(error_msg);
    }

    let response_text = response
        .text()
        .await
        .context("Failed to read response body")?;

    let create_response: CreateRepoResponse = serde_json::from_str(&response_text)
        .context("Failed to parse create repository response")?;

    Ok(create_response.data.id.to_string())
}

fn save_config(
    repo_id: &str,
    base_url: &str,
    is_admin: bool,
    execution_mode: ExecutionMode,
) -> Result<()> {
    let project_root = PathBuf::from(".");
    let mut config = crate::config::ProjectConfig::load(&project_root)?;

    config.repo = Some(crate::config::RepoConfig {
        id: repo_id.to_string(),
        url: base_url.to_string(),
        is_admin,
    });
    config.execution_mode = execution_mode;

    config.save(&project_root)?;

    // Create .gitignore for generated files
    let verilib_path = project_root.join(".verilib");
    create_gitignore(&verilib_path)?;

    Ok(())
}
