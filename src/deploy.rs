use anyhow::{Context, Result};
use dialoguer::Select;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};

use crate::constants::{auth_required_msg, DEFAULT_BASE_URL};
use crate::status::get_stored_api_key;

#[derive(Debug)]
struct Language {
    id: u32,
    name: &'static str,
    extensions: &'static [&'static str],
}

const LANGUAGES: &[Language] = &[
    Language { id: 1, name: "Dafny", extensions: &[".dfy"] },
    Language { id: 2, name: "Lean", extensions: &[".lean"] },
    Language { id: 3, name: "Rocq", extensions: &[".v"] },
    Language { id: 4, name: "Isabelle", extensions: &[".thy"] },
    Language { id: 5, name: "Metamath", extensions: &[".mm"] },
    Language { id: 6, name: "Rust", extensions: &[".rs"] },
    Language { id: 7, name: "RefinedC", extensions: &[".c"] },
    Language { id: 8, name: "Python", extensions: &[".py"] },
    Language { id: 9, name: "Kani", extensions: &[".rs"] },
    Language { id: 10, name: "Verus", extensions: &[".rs"] },
];

const TYPES: &[(u32, &str)] = &[
    (1, "Algorithms"),
    (5, "Blockchain"),
    (6, "Privacy"),
    (7, "Security"),
    (8, "Math"),
];

#[derive(Debug, Serialize)]
struct DeployNode {
    identifier: String,
    content: String,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    dependencies: Vec<String>,
    children: Vec<DeployNode>,
}

#[derive(Debug, Deserialize)]
struct VerifierVersionsResponse {
    data: Vec<VerifierVersion>,
}

#[derive(Debug, Deserialize)]
struct VerifierVersion {
    id: u32,
    version: String,
}

#[derive(Debug, Deserialize, Serialize)]
struct Metadata {
    repo: RepoMetadata,
}

#[derive(Debug, Deserialize, Serialize)]
struct RepoMetadata {
    id: String,
    url: String,
}

#[derive(Debug, Deserialize)]
struct DeployResponse {
    status: String,
    data: DeployData,
}

#[derive(Debug, Deserialize)]
struct DeployData {
    id: u64,
}

pub async fn handle_deploy(url: Option<String>, debug: bool) -> Result<()> {
    println!("Preparing deployment...");
    println!("Debug mode: {}", debug);

    let api_key = get_stored_api_key()
        .context(auth_required_msg())?;

    let url_base = url.unwrap_or_else(|| DEFAULT_BASE_URL.to_string());

    let repo_id = read_repo_id_from_metadata()?;
    
    let deploy_info = if repo_id.is_none() {
        println!("New repository - collecting deployment information...");
        Some(collect_deploy_info(&url_base, &api_key).await?)
    } else {
        println!("Updating existing repository (ID: {})...", repo_id.as_ref().unwrap());
        None
    };

    println!("\nScanning .verilib directory...");
    
    let verilib_path = PathBuf::from(".verilib");
    if !verilib_path.exists() {
        anyhow::bail!("No .verilib directory found. Please run 'init' first.");
    }

    let tree = build_tree(&verilib_path, &verilib_path)?;
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
    let response_text = response
        .text()
        .await
        .unwrap_or_else(|_| "Unable to read response".to_string());

    if !status.is_success() {
        anyhow::bail!(
            "Deploy failed with status: {} - {}",
            status,
            response_text
        );
    }

    if debug {
        println!("Debug: API response: {}", response_text);
    }
    
    let deploy_response: DeployResponse = serde_json::from_str(&response_text)
        .context("Failed to parse deploy response")?;
    
    save_metadata_from_response(&deploy_response, &url_base)
        .context("Failed to save metadata file")?;

    println!("Deployment successful!");
    
    Ok(())
}

fn read_repo_id_from_metadata() -> Result<Option<String>> {
    let metadata_path = PathBuf::from(".verilib/metadata.json");
    
    if !metadata_path.exists() {
        return Ok(None);
    }

    let metadata_content = fs::read_to_string(&metadata_path)
        .context("Failed to read metadata.json")?;
    
    let metadata: Metadata = serde_json::from_str(&metadata_content)
        .context("Failed to parse metadata.json")?;
    
    Ok(Some(metadata.repo.id))
}

fn save_metadata_from_response(response_data: &DeployResponse, base_url: &str) -> Result<()> {
    let repo_id_str = response_data.data.id.to_string();
    
    let metadata = Metadata {
        repo: RepoMetadata {
            id: repo_id_str.clone(),
            url: base_url.to_string(),
        },
    };
    
    let metadata_path = PathBuf::from(".verilib/metadata.json");
    let metadata_json = serde_json::to_string_pretty(&metadata)
        .context("Failed to serialize metadata")?;
    
    fs::write(&metadata_path, metadata_json)
        .context("Failed to write metadata.json")?;
    
    println!("Metadata saved to .verilib/metadata.json");
    println!("Repository ID: {}", response_data.data.id);
    println!("Repository URL: {}", base_url);
    Ok(())
}

fn detect_language() -> Option<u32> {
    let verilib_path = PathBuf::from(".verilib");
    
    println!("Debug: Scanning for language detection in .verilib directory...");
    
    for language in LANGUAGES {
        println!("Debug: Checking for {} with extensions: {:?}", language.name, language.extensions);
        for ext in language.extensions {
            if find_files_with_extension(&verilib_path, ext) {
                println!("Debug: Found {} file with extension {}", language.name, ext);
                return Some(language.id);
            }
        }
    }
    
    println!("Debug: No matching language detected");
    None
}

fn find_files_with_extension(dir: &Path, extension: &str) -> bool {
    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            let file_name = path.file_name().unwrap_or_default().to_string_lossy();
            
            if file_name == "metadata.json" || file_name == "debug_response.json" {
                continue;
            }
            
            if path.is_dir() {
                let dir_name = file_name.to_string();
                let ext_without_dot = extension.trim_start_matches('.');
                
                if dir_name == format!("mod{}", extension) || 
                   dir_name.ends_with(&format!(".{}", ext_without_dot)) {
                    println!("Debug: Found matching directory: {} with extension {}", dir_name, extension);
                    return true;
                }
                
                if find_files_with_extension(&path, extension) {
                    return true;
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

async fn fetch_verifier_versions(proof_id: u32, base_url: &str, api_key: &str) -> Result<Option<u32>> {
    let endpoint = format!("{}/v2/verifier/versions/{}", base_url, proof_id);
    
    println!("Debug: Fetching verifier versions from: {}", endpoint);
    
    let client = Client::new();
    let response = client
        .get(&endpoint)
        .header("Authorization", format!("ApiKey {}", api_key))
        .header("Accept", "application/json")
        .send()
        .await
        .context("Failed to fetch verifier versions")?;
    
    println!("Debug: Response status: {}", response.status());
    
    if !response.status().is_success() {
        println!("Debug: Request failed, no verifier versions available");
        return Ok(None);
    }
    
    let response_text = response
        .text()
        .await
        .context("Failed to read response body")?;
    
    println!("Debug: Response body: {}", response_text);
    
    let versions_response: VerifierVersionsResponse = serde_json::from_str(&response_text)
        .context("Failed to parse verifier versions response")?;
    
    println!("Debug: Found {} versions", versions_response.data.len());
    
    if versions_response.data.is_empty() {
        println!("Debug: No versions available");
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

async fn collect_deploy_info(base_url: &str, api_key: &str) -> Result<(u32, u32, Option<u32>, String, Option<String>, u32)> {
    let detected_language = detect_language();
    
    let language_id = prompt_language(detected_language, "Select Language:")?;
    let proof_id = prompt_language(Some(language_id), "Select Proof Language:")?;
    
    let verifierversion_id = fetch_verifier_versions(proof_id, base_url, api_key).await?;
    
    let summary = prompt_summary()?;
    let description = prompt_description()?;
    let type_id = prompt_type()?;
    
    Ok((language_id, proof_id, verifierversion_id, summary, description, type_id))
}

fn build_tree(base_path: &Path, current_path: &Path) -> Result<Vec<DeployNode>> {
    let mut nodes = Vec::new();
    
    let entries = fs::read_dir(current_path)
        .with_context(|| format!("Failed to read directory: {:?}", current_path))?;
    
    for entry in entries {
        let entry = entry?;
        let path = entry.path();
        let file_name = entry.file_name();
        let file_name_str = file_name.to_string_lossy();
        
        if file_name_str == "metadata.json" || file_name_str == "debug_response.json" {
            continue;
        }
        
        if path.is_dir() {
            let relative_path = path.strip_prefix(base_path)
                .unwrap()
                .to_string_lossy()
                .to_string();
            
            let children = build_tree(base_path, &path)?;
            
            nodes.push(DeployNode {
                identifier: relative_path,
                content: String::new(),
                dependencies: Vec::new(),
                children,
            });
        } else if file_name_str.ends_with(".atom.verilib") {
            let content = fs::read_to_string(&path)
                .with_context(|| format!("Failed to read file: {:?}", path))?;
            
            let identifier = path.strip_prefix(base_path)
                .unwrap()
                .to_string_lossy()
                .to_string()
                .trim_end_matches(".atom.verilib")
                .to_string();
            
            let meta_file_name = file_name_str.trim_end_matches(".atom.verilib").to_string() + ".meta.verilib";
            let meta_path = path.parent().unwrap().join(meta_file_name);
            
            let dependencies = if meta_path.exists() {
                let meta_content = fs::read_to_string(&meta_path)?;
                let meta_value: Value = serde_json::from_str(&meta_content)?;
                if let Some(deps) = meta_value.get("dependencies") {
                    serde_json::from_value(deps.clone()).unwrap_or_default()
                } else {
                    Vec::new()
                }
            } else {
                Vec::new()
            };
            
            nodes.push(DeployNode {
                identifier,
                content,
                dependencies,
                children: Vec::new(),
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
