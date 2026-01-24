use anyhow::{Context, Result};
use dialoguer::Select;
use regex::Regex;
use reqwest::Client;
use serde_json::Value;
use sha2::{Sha256, Digest};
use std::collections::HashMap;
use std::ffi::OsStr;
use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};

use crate::constants::{auth_required_msg, DEFAULT_BASE_URL};
use crate::commands::status::get_stored_api_key;
use crate::download::handle_api_error;
use super::types::{Config, DeployNode, DeployResponse, RepoConfig, VerifierVersionsResponse, LANGUAGES, TYPES};

#[derive(Debug, Clone, Copy)]
enum ChangeDecision {
    Ask,
    YesToAll,
    NoToAll,
}

pub async fn handle_deploy(url: Option<String>, debug: bool) -> Result<()> {
    println!("Preparing deployment...");
    if debug {
        println!("Debug mode: {}", debug);
    }

    let api_key = get_stored_api_key()
        .context(auth_required_msg())?;

    let url_base = url.unwrap_or_else(|| DEFAULT_BASE_URL.to_string());

    let repo_id = read_repo_id_from_config()?;
    
    let deploy_info = if repo_id.is_none() {
        println!("New repository - collecting deployment information...");
        Some(collect_deploy_info(&url_base, &api_key, debug).await?)
    } else {
        println!("Updating existing repository (ID: {})...", repo_id.as_ref().unwrap());
        None
    };

    println!("\nScanning .verilib directory...");
    
    let verilib_path = PathBuf::from(".verilib");
    if !verilib_path.exists() {
        anyhow::bail!("No .verilib directory found. Please run 'init' first.");
    }

    let mut decision = ChangeDecision::Ask;
    let mut has_changes = false;
    let tree = build_tree(&verilib_path, &verilib_path, &mut decision, &mut has_changes)?;
    let layouts = build_layouts(&verilib_path, &verilib_path)?;
    
    if debug {
        let tree_json = serde_json::to_string_pretty(&tree)
            .context("Failed to serialize tree for debugging")?;
        fs::write(".verilib/debug_deploy_tree.json", &tree_json)
            .context("Failed to write debug tree file")?;
        println!("Debug: Tree saved to .verilib/debug_deploy_tree.json");
        
        let layouts_json = serde_json::to_string_pretty(&layouts)
            .context("Failed to serialize layouts for debugging")?;
        fs::write(".verilib/debug_deploy_layouts.json", &layouts_json)
            .context("Failed to write debug layouts file")?;
        println!("Debug: Layouts saved to .verilib/debug_deploy_layouts.json");
    }

    let mut payload = serde_json::json!({
        "tree": tree,
        "layouts": layouts,
    });
    
    if has_changes {
        payload["has_changes"] = Value::Bool(true);
    }

    if let Some((language_id, proof_id, verifierversion_id, summary, description, type_id)) = deploy_info {
        payload["language_id"] = Value::Number(language_id.into());
        payload["proof_id"] = Value::Number(proof_id.into());
        payload["summary"] = Value::String(summary);
        payload["type_id"] = Value::Number(type_id.into());
        
        if let Some(desc) = description {
            payload["description"] = Value::String(desc);
        }
        
        if let Some(version_id) = verifierversion_id {
            payload["verifierversion_id"] = Value::Number(version_id.into());
        }
    }

    let endpoint = if let Some(ref id) = repo_id {
        payload["repo_id"] = Value::String(id.clone());
        format!("{}/v2/repo/deploy/{}", url_base, id)
    } else {
        format!("{}/v2/repo/deploy", url_base)
    };

    println!("\nDeploying to {}...", endpoint);

    let client = Client::new();
    let response = client
        .post(&endpoint)
        .header("Authorization", format!("ApiKey {}", api_key))
        .header("Content-Type", "application/json")
        .json(&payload)
        .send()
        .await
        .context("Failed to send deploy request")?;

    let status = response.status();
    
    if !status.is_success() {
        let error_msg = handle_api_error(response).await?;
        anyhow::bail!(error_msg);
    }
    
    let response_text = response
        .text()
        .await
        .context("Failed to read response body")?;

    if debug {
        println!("Debug: API response: {}", response_text);
    }
    
    let deploy_response: DeployResponse = serde_json::from_str(&response_text)
        .context("Failed to parse deploy response")?;
    
    save_config_from_response(&deploy_response, &url_base)
        .context("Failed to save config file")?;

    println!("Deployment successful!");
    
    Ok(())
}

fn read_repo_id_from_config() -> Result<Option<String>> {
    let config_path = PathBuf::from(".verilib/config.json");

    if !config_path.exists() {
        return Ok(None);
    }

    let config_content = fs::read_to_string(&config_path)
        .context("Failed to read config.json")?;

    let config: Config = serde_json::from_str(&config_content)
        .context("Failed to parse config.json")?;

    Ok(Some(config.repo.id))
}

fn save_config_from_response(response_data: &DeployResponse, base_url: &str) -> Result<()> {
    let repo_id_str = response_data.data.id.to_string();

    let mut is_admin = false;
    let config_path = PathBuf::from(".verilib/config.json");
    if config_path.exists() {
         if let Ok(content) = fs::read_to_string(&config_path) {
             if let Ok(cfg) = serde_json::from_str::<Config>(&content) {
                 is_admin = cfg.repo.is_admin;
             }
         }
    }

    let config = Config {
        repo: RepoConfig {
            id: repo_id_str.clone(),
            url: base_url.to_string(),
            is_admin,
        },
    };

    let config_json = serde_json::to_string_pretty(&config)
        .context("Failed to serialize config")?;

    fs::write(&config_path, config_json)
        .context("Failed to write config.json")?;

    println!("Config saved to .verilib/config.json");
    println!("Repository ID: {}", response_data.data.id);
    println!("Repository URL: {}", base_url);
    Ok(())
}

fn detect_language_in_path(search_path: &PathBuf, debug: bool) -> Option<u32> {
    let full_path = std::fs::canonicalize(search_path)
        .unwrap_or_else(|_| search_path.clone());
    if debug {
        println!("Debug: Scanning for language detection in directory: {}", full_path.display());
    }
    
    for language in LANGUAGES {
        if debug {
            println!("Debug: Checking for {} with extensions: {:?}", language.name, language.extensions);
        }
        for ext in language.extensions {
            if find_files_with_extension(search_path, ext, debug) {
                if debug {
                    println!("Debug: Found {} file with extension {}", language.name, ext);
                }
                return Some(language.id);
            }
        }
    }
    
    if debug {
        println!("Debug: No matching language detected");
    }
    None
}

fn find_files_with_extension(dir: &Path, extension: &str, debug: bool) -> bool {
    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            let file_name = path.file_name().unwrap_or_default().to_string_lossy();
            
            if file_name == "config.json" || file_name == "debug_response.json" {
                continue;
            }
            
            if path.is_dir() {
                let dir_name = file_name.to_string();
                let ext_without_dot = extension.trim_start_matches('.');
                
                if dir_name == format!("mod{}", extension) || 
                   dir_name.ends_with(&format!(".{}", ext_without_dot)) {
                    if debug {
                        println!("Debug: Found matching directory: {} with extension {}", dir_name, extension);
                    }
                    return true;
                }
                
                if find_files_with_extension(&path, extension, debug) {
                    return true;
                }
            } else if path.is_file() {
                if let Some(file_ext) = path.extension() {
                    let file_ext_str = file_ext.to_string_lossy();
                    let ext_without_dot = extension.trim_start_matches('.');
                    if file_ext_str == ext_without_dot {
                        if debug {
                            println!("Debug: Found matching file: {} with extension {}", file_name, extension);
                        }
                        return true;
                    }
                }
            }
        }
    }
    
    false
}

fn prompt_language(default_id: Option<u32>, prompt_text: &str) -> Result<u32> {
    let items: Vec<String> = LANGUAGES.iter()
        .map(|l| {
            if Some(l.id) == default_id {
                format!("{} (detected)", l.name)
            } else {
                l.name.to_string()
            }
        })
        .collect();
    
    let default_idx = if let Some(id) = default_id {
        LANGUAGES.iter().position(|l| l.id == id).unwrap_or(0)
    } else {
        0
    };
    
    let selection = Select::new()
        .with_prompt(prompt_text)
        .items(&items)
        .default(default_idx)
        .interact()
        .context("Failed to get language selection")?;
    
    Ok(LANGUAGES[selection].id)
}

async fn fetch_verifier_versions(proof_id: u32, base_url: &str, api_key: &str, debug: bool) -> Result<Option<u32>> {
    let endpoint = format!("{}/v2/verifier/versions/{}", base_url, proof_id);
    
    if debug {
        println!("Debug: Fetching verifier versions from: {}", endpoint);
    }
    
    let client = Client::new();
    let response = client
        .get(&endpoint)
        .header("Authorization", format!("ApiKey {}", api_key))
        .header("Accept", "application/json")
        .send()
        .await
        .context("Failed to fetch verifier versions")?;
    
    if debug {
        println!("Debug: Response status: {}", response.status());
    }
    
    if !response.status().is_success() {
        let error_msg = handle_api_error(response).await?;
        if debug {
            println!("Debug: Request failed - {}", error_msg);
        }
        return Ok(None);
    }
    
    let response_text = response
        .text()
        .await
        .context("Failed to read response body")?;
    
    if debug {
        println!("Debug: Response body: {}", response_text);
    }
    
    let versions_response: VerifierVersionsResponse = serde_json::from_str(&response_text)
        .context("Failed to parse verifier versions response")?;
    
    if debug {
        println!("Debug: Found {} versions", versions_response.data.len());
    }
    
    if versions_response.data.is_empty() {
        if debug {
            println!("Debug: No versions available");
        }
        return Ok(None);
    }
    
    let items: Vec<String> = versions_response.data.iter()
        .map(|v| v.version.clone())
        .collect();
    
    let selection = Select::new()
        .with_prompt("Select Verifier Version")
        .items(&items)
        .default(0)
        .interact()
        .context("Failed to get version selection")?;
    
    Ok(Some(versions_response.data[selection].id))
}

fn prompt_type() -> Result<u32> {
    let items: Vec<&str> = TYPES.iter().map(|(_, name)| *name).collect();
    
    let selection = Select::new()
        .with_prompt("Select Type")
        .items(&items)
        .default(0)
        .interact()
        .context("Failed to get type selection")?;
    
    Ok(TYPES[selection].0)
}

fn prompt_summary() -> Result<String> {
    loop {
        println!("\nEnter summary (max 128 characters, required):");
        print!("> ");
        io::stdout().flush()?;
        
        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        let input = input.trim().to_string();
        
        if input.is_empty() {
            println!("Summary cannot be empty. Please try again.");
            continue;
        }
        
        if input.chars().all(|c| c.is_whitespace()) {
            println!("Summary cannot contain only whitespace. Please try again.");
            continue;
        }
        
        if input.len() > 128 {
            println!("Summary must be 128 characters or less (current: {}). Please try again.", input.len());
            continue;
        }
        
        return Ok(input);
    }
}

fn prompt_description() -> Result<Option<String>> {
    println!("\nEnter description (optional, press Enter to skip):");
    print!("> ");
    io::stdout().flush()?;
    
    let mut input = String::new();
    io::stdin().read_line(&mut input)?;
    let input = input.trim().to_string();
    
    if input.is_empty() {
        Ok(None)
    } else {
        Ok(Some(input))
    }
}

pub async fn collect_deploy_info(base_url: &str, api_key: &str, debug: bool) -> Result<(u32, u32, Option<u32>, String, Option<String>, u32)> {
    collect_deploy_info_with_path(base_url, api_key, &PathBuf::from(".verilib"), debug).await
}

pub async fn collect_deploy_info_with_path(base_url: &str, api_key: &str, search_path: &PathBuf, debug: bool) -> Result<(u32, u32, Option<u32>, String, Option<String>, u32)> {
    let detected_language = detect_language_in_path(search_path, debug);
    
    let language_id = prompt_language(detected_language, "Select Language:")?;
    let proof_id = prompt_language(Some(language_id), "Select Proof Language:")?;
    
    let verifierversion_id = fetch_verifier_versions(proof_id, base_url, api_key, debug).await?;
    
    let summary = prompt_summary()?;
    let description = prompt_description()?;
    let type_id = prompt_type()?;
    
    Ok((language_id, proof_id, verifierversion_id, summary, description, type_id))
}

fn build_tree(base_path: &Path, current_path: &Path, decision: &mut ChangeDecision, has_changes: &mut bool) -> Result<Vec<DeployNode>> {
    let mut nodes = Vec::new();
    
    let entries = fs::read_dir(current_path)
        .with_context(|| format!("Failed to read directory: {:?}", current_path))?;
    
    for entry in entries {
        let entry = entry?;
        let path = entry.path();
        let extension = path.extension();
        let file_name = entry.file_name();
        let file_name_str = file_name.to_string_lossy();

        if extension == Some(OsStr::new("json")) {
            continue;
        }
        
        if path.is_dir() {
            let relative_path = path.strip_prefix(base_path)
                .unwrap()
                .to_string_lossy()
                .to_string();
            
            let children = build_tree(base_path, &path, decision, has_changes)?;
            
            nodes.push(DeployNode {
                identifier: relative_path,
                content: String::new(),
                dependencies: Vec::new(),
                code_name: String::new(),
                file_type: "folder".to_string(),
                children,
                status_id: None,
                snippets: None,
                specified: false,
                disabled: false,
            });
        } else if file_name_str.ends_with(".atom.verilib") {
            let content = fs::read_to_string(&path)
                .with_context(|| format!("Failed to read file: {:?}", path))?;
            let regex_pattern = r"\[\d*\]\s-\s";
            let re = Regex::new(regex_pattern).unwrap();
            let identifier_base = path.strip_prefix(base_path)
                .unwrap()
                .to_string_lossy()
                .to_string()
                .trim_end_matches(".atom.verilib")
                .to_string();
            let identifier = re.replace(&identifier_base, "").to_string();
            
            let meta_file_name = file_name_str.trim_end_matches(".atom.verilib").to_string() + ".meta.verilib";
            let meta_path = path.parent().unwrap().join(meta_file_name);
            
            let (dependencies, code_name, status_id, stored_fingerprint, snippets_value, specified, disabled) = if meta_path.exists() {
                let meta_content = fs::read_to_string(&meta_path)?;
                let meta_value: Value = serde_json::from_str(&meta_content)?;
                
                let deps = if let Some(deps) = meta_value.get("dependencies") {
                    serde_json::from_value(deps.clone()).unwrap_or_default()
                } else {
                    Vec::new()
                };
                
                let name = if let Some(name) = meta_value.get("code_name") {
                    name.as_str().unwrap_or_default().to_string()
                } else {
                    String::new()
                };
                
                let status = meta_value.get("status_id").and_then(|v| v.as_u64()).map(|v| v as u32);
                let fingerprint = meta_value.get("fingerprint").and_then(|v| v.as_str()).map(|s| s.to_string());
                let snippets = meta_value.get("snippets").cloned();
                let specified = meta_value.get("specified").and_then(|v| v.as_bool()).unwrap_or_default();
                let disabled = meta_value.get("disabled").and_then(|v| v.as_bool()).unwrap_or_default();
                
                (deps, name, status, fingerprint, snippets, specified, disabled)
            } else {
                (Vec::new(), String::new(), None, None, None, false, false)
            };
            
            let mut hasher = Sha256::new();
            hasher.update(&content);
            let hash_result = hasher.finalize();
            let current_fingerprint = format!("{:x}", hash_result);
            
            let (final_content, snippets) = if let Some(stored_fp) = stored_fingerprint {
                if stored_fp != current_fingerprint {
                    let use_new_content = match *decision {
                        ChangeDecision::YesToAll => true,
                        ChangeDecision::NoToAll => false,
                        ChangeDecision::Ask => {
                            println!("\nFile has been modified: {}", identifier);
                            println!("   Current file differs from the stored version.");
                            
                            let options = vec![
                                "Yes - Deploy edited content (triggers re-snippetization for entire repository)",
                                "No - Keep original snippets for this file",
                                "No to all - Skip all edited files"
                            ];
                            
                            let selection = Select::new()
                                .with_prompt("Would you like to deploy the edited content?")
                                .items(&options)
                                .default(0)
                                .interact()?;
                            
                            match selection {
                                0 => {
                                    *decision = ChangeDecision::YesToAll;
                                    true
                                }
                                1 => false,
                                2 => {
                                    *decision = ChangeDecision::NoToAll;
                                    false
                                }
                                _ => false,
                            }
                        }
                    };
                    if use_new_content {
                        *has_changes = true;
                    }
                } else {
                   
                }
                 (content.clone(), snippets_value)
            } else {
                *has_changes = true;
                (content.clone(), None)
            };
            
            nodes.push(DeployNode {
                identifier,
                content: final_content,
                dependencies,
                code_name,
                file_type: "file".to_string(),
                children: Vec::new(),
                status_id,
                snippets,  
                specified,
                disabled
            });
        }
    }
    
    Ok(nodes)
}

fn build_layouts(base_path: &Path, current_path: &Path) -> Result<HashMap<String, Value>> {
    let mut layouts = HashMap::new();
    
    let entries = fs::read_dir(current_path)
        .with_context(|| format!("Failed to read directory: {:?}", current_path))?;
    
    for entry in entries {
        let entry = entry?;
        let path = entry.path();
        
        if path.is_dir() {
            let layout_file = path.join("layout.verilib");
            
            if layout_file.exists() {
                let layout_content = fs::read_to_string(&layout_file)?;
                let layout_value: Value = serde_json::from_str(&layout_content)?;
                
                let relative_path = path.strip_prefix(base_path)
                    .unwrap()
                    .to_string_lossy()
                    .to_string();
                
                layouts.insert(relative_path, layout_value);
            }
            
            let child_layouts = build_layouts(base_path, &path)?;
            layouts.extend(child_layouts);
        }
    }
    
    Ok(layouts)
}
