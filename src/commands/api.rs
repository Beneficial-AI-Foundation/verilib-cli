use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use std::io::{self, Read, IsTerminal};

#[derive(Debug, Clone)]
pub enum ApiSubcommand {
    Get {
        file: PathBuf,
    },
    List {
        filter: Option<StatusFilter>,
    },
    Set {
        file: PathBuf,
        specified: Option<bool>,
        ignored: Option<bool>,
        verified: Option<bool>,
    },
    Batch {
        input: PathBuf,
    },
    CreateFile {
        path: PathBuf,
        content: Option<String>,
        from_file: Option<PathBuf>,
        disabled: bool,
        specified: bool,
        status_id: u32,
        statement_type: Option<String>,
        code_name: Option<String>,
    },
}

#[derive(Debug, Clone)]
pub enum StatusFilter {
    Specified,
    Ignored,
    Verified,
}

#[derive(Serialize, Deserialize, Debug)]
struct MetaFile {
    #[serde(default)]
    pub specified: bool,
    #[serde(default)]
    pub disabled: bool,
    #[serde(default)]
    pub status_id: u32,
    #[serde(flatten)]
    pub other: serde_json::Value,
}

#[derive(Serialize, Debug)]
struct GetOutput {
    file: String,
    specified: bool,
    ignored: bool,
    verified: bool,
    status_id: u32,
}

#[derive(Serialize, Debug)]
struct ListOutput {
    files: Vec<FileInfo>,
}

#[derive(Serialize, Debug)]
struct FileInfo {
    path: String,
    specified: bool,
    ignored: bool,
    verified: bool,
}

#[derive(Deserialize, Debug)]
struct BatchInput {
    operations: Vec<BatchOperation>,
}

#[derive(Deserialize, Debug)]
struct BatchOperation {
    file: String,
    #[serde(default)]
    specified: Option<bool>,
    #[serde(default)]
    ignored: Option<bool>,
    #[serde(default)]
    verified: Option<bool>,
}

#[derive(Serialize, Debug)]
struct BatchOutput {
    success_count: usize,
    error_count: usize,
    results: Vec<BatchResult>,
}

#[derive(Serialize, Debug)]
struct BatchResult {
    file: String,
    success: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<String>,
}

pub async fn handle_api(
    subcommand: ApiSubcommand,
    json_output: bool,
    dry_run: bool,
) -> Result<()> {
    match subcommand {
        ApiSubcommand::Get { file } => handle_get(file, json_output).await,
        ApiSubcommand::List { filter } => handle_list(filter, json_output).await,
        ApiSubcommand::Set {
            file,
            specified,
            ignored,
            verified,
        } => handle_set(file, specified, ignored, verified, json_output, dry_run).await,
        ApiSubcommand::Batch { input } => handle_batch(input, json_output, dry_run).await,
        ApiSubcommand::CreateFile {
            path,
            content,
            from_file,
            disabled,
            specified,
            status_id,
            statement_type,
            code_name,
        } => handle_create_file(path, content, from_file, disabled, specified, status_id, statement_type, code_name, json_output, dry_run).await,
    }
}

async fn handle_create_file(
    path: PathBuf,
    content: Option<String>,
    from_file: Option<PathBuf>,
    disabled: bool,
    specified: bool,
    status_id: u32,
    statement_type: Option<String>,
    code_name: Option<String>,
    json_output: bool,
    dry_run: bool,
) -> Result<()> {
    let (final_content, source_desc) = if let Some(c) = content {
        (c, "argument string".to_string())
    } else if let Some(p) = from_file {
        let content = fs::read_to_string(&p)
            .with_context(|| format!("Failed to read source file: {:?}", p))?;
        (content, format!("file {:?}", p))
    } else if !io::stdin().is_terminal() {
        let mut content = String::new();
        io::stdin().read_to_string(&mut content)
            .context("Failed to read from stdin")?;
        (content, "stdin".to_string())
    } else {
        anyhow::bail!("No content provided. Use --content, --from-file, or pipe content to stdin.");
    };

    let identifier = path.file_name()
        .ok_or_else(|| anyhow::anyhow!("Invalid path: no filename"))?
        .to_string_lossy()
        .to_string();

    let logical_parent = path.parent().unwrap_or_else(|| std::path::Path::new(""));
    let verilib_root = PathBuf::from(".verilib");
    
    let physical_parent = if logical_parent.starts_with(&verilib_root) {
        logical_parent.to_path_buf()
    } else {
        verilib_root.join(logical_parent)
    };

    if !dry_run {
        fs::create_dir_all(&physical_parent)
            .with_context(|| format!("Failed to create parent directories for {:?}", physical_parent))?;
    }

    let mut next_index = 0;
    if physical_parent.exists() {
        let re = regex::Regex::new(r"^\[(\d+)\]\s*-\s*").unwrap();
        for entry in fs::read_dir(&physical_parent)? {
            let entry = entry?;
            let file_name = entry.file_name().to_string_lossy().to_string();
            if let Some(caps) = re.captures(&file_name) {
                if let Ok(idx) = caps[1].parse::<u32>() {
                    if idx >= next_index {
                        next_index = idx + 1;
                    }
                }
            }
        }
    }

    let atom_filename = format!("[{}] - {}.atom.verilib", next_index, identifier);
    let meta_filename = format!("[{}] - {}.meta.verilib", next_index, identifier);
    let atom_path = physical_parent.join(&atom_filename);
    let meta_path = physical_parent.join(&meta_filename);

    let final_code_name = code_name.unwrap_or_else(|| {
        physical_parent.file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| "unknown".to_string())
    });

    let path_str = path.to_string_lossy();
    let clean_path = if path_str.starts_with(".verilib/") {
        path_str.strip_prefix(".verilib/").unwrap()
    } else if path_str.starts_with(".verilib\\") {
        path_str.strip_prefix(".verilib\\").unwrap()
    } else {
        &path_str
    };
    
    let json_path = if clean_path.starts_with('/') {
        clean_path.to_string()
    } else {
        format!("/{}", clean_path)
    };

    let meta_json = serde_json::json!({
        "code_name": final_code_name,
        "disabled": disabled,
        "identifier": identifier,
        "index": next_index,
        "path": json_path,
        "snippets": [
            {
                "sortorder": 0,
                "text": final_content,
                "type_id": 2
            }
        ],
        "specified": specified,
        "status_id": status_id,
        "statement_type": statement_type
    });

    if dry_run {
        if json_output {
            let output = serde_json::json!({
                "atom_file": atom_path.to_string_lossy(),
                "meta_file": meta_path.to_string_lossy(),
                "status": "dry_run",
                "source": source_desc,
                "meta_content": meta_json
            });
            println!("{}", serde_json::to_string_pretty(&output)?);
        } else {
            println!("Would create atom file: {:?}", atom_path);
            println!("Would create meta file: {:?}", meta_path);
            println!("Source: {}", source_desc);
            println!("Meta content:\n{}", serde_json::to_string_pretty(&meta_json)?);
        }
        return Ok(());
    }

    fs::write(&atom_path, &final_content)
        .with_context(|| format!("Failed to write atom file: {:?}", atom_path))?;
    
    let meta_content_str = serde_json::to_string_pretty(&meta_json)?;
    fs::write(&meta_path, &meta_content_str)
        .with_context(|| format!("Failed to write meta file: {:?}", meta_path))?;

    if json_output {
        let output = serde_json::json!({
            "atom_file": atom_path.to_string_lossy(),
            "meta_file": meta_path.to_string_lossy(),
            "status": "created",
            "source": source_desc
        });
        println!("{}", serde_json::to_string_pretty(&output)?);
    } else {
        println!("Created atom file: {:?}", atom_path);
        println!("Created meta file: {:?}", meta_path);
    }

    Ok(())
}

async fn handle_get(file: PathBuf, json_output: bool) -> Result<()> {
    let resolved_path = resolve_file_path(&file)?;
    validate_meta_file(&resolved_path)?;
    
    let content = fs::read_to_string(&resolved_path)
        .with_context(|| format!("Failed to read file: {:?}", resolved_path))?;
    
    let meta: MetaFile = serde_json::from_str(&content)
        .context("Failed to parse meta file")?;
    
    let output = GetOutput {
        file: resolved_path.to_string_lossy().to_string(),
        specified: meta.specified,
        ignored: meta.disabled,
        verified: meta.status_id == 2,
        status_id: meta.status_id,
    };
    
    if json_output {
        println!("{}", serde_json::to_string_pretty(&output)?);
    } else {
        println!("File: {}", output.file);
        println!("  Specified: {}", output.specified);
        println!("  Ignored:   {}", output.ignored);
        println!("  Verified:  {}", output.verified);
        println!("  Status ID: {}", output.status_id);
    }
    
    Ok(())
}

async fn handle_list(filter: Option<StatusFilter>, json_output: bool) -> Result<()> {
    let verilib_dir = PathBuf::from(".verilib");
    
    if !verilib_dir.exists() {
        anyhow::bail!("No .verilib directory found. Please run 'init' first.");
    }
    
    let mut files = Vec::new();
    
    for entry in walkdir::WalkDir::new(&verilib_dir)
        .into_iter()
        .filter_map(|e| e.ok())
    {
        let path = entry.path();
        if path.is_file() && path.extension().map_or(false, |ext| ext == "verilib") {
            let file_name = path.file_name().unwrap_or_default().to_string_lossy();
            if file_name.contains(".meta.") {
                if let Ok(content) = fs::read_to_string(path) {
                    if let Ok(meta) = serde_json::from_str::<MetaFile>(&content) {
                        let matches_filter = match &filter {
                            None => true,
                            Some(StatusFilter::Specified) => meta.specified,
                            Some(StatusFilter::Ignored) => meta.disabled,
                            Some(StatusFilter::Verified) => meta.status_id == 2,
                        };
                        
                        if matches_filter {
                            files.push(FileInfo {
                                path: path.to_string_lossy().to_string(),
                                specified: meta.specified,
                                ignored: meta.disabled,
                                verified: meta.status_id == 2,
                            });
                        }
                    }
                }
            }
        }
    }
    
    if json_output {
        let output = ListOutput { files };
        println!("{}", serde_json::to_string_pretty(&output)?);
    } else {
        println!("Found {} files", files.len());
        for file in files {
            println!("  {} [Spec: {} | Ign: {} | Ver: {}]", 
                file.path, file.specified, file.ignored, file.verified);
        }
    }
    
    Ok(())
}

async fn handle_set(
    file: PathBuf,
    specified: Option<bool>,
    ignored: Option<bool>,
    verified: Option<bool>,
    json_output: bool,
    dry_run: bool,
) -> Result<()> {
    let resolved_path = resolve_file_path(&file)?;
    validate_meta_file(&resolved_path)?;
    
    if verified.is_some() {
        check_admin_status()?;
    }
    
    let content = fs::read_to_string(&resolved_path)
        .with_context(|| format!("Failed to read file: {:?}", resolved_path))?;
    
    let mut meta: MetaFile = serde_json::from_str(&content)
        .context("Failed to parse meta file")?;
    
    let mut changes = Vec::new();
    
    if let Some(val) = specified {
        if meta.specified != val {
            changes.push(format!("specified: {} -> {}", meta.specified, val));
            meta.specified = val;
        }
    }
    
    if let Some(val) = ignored {
        if meta.disabled != val {
            changes.push(format!("ignored: {} -> {}", meta.disabled, val));
            meta.disabled = val;
        }
    }
    
    if let Some(val) = verified {
        let new_status = if val { 2 } else { 0 };
        if meta.status_id != new_status {
            changes.push(format!("verified: {} -> {}", meta.status_id == 2, val));
            meta.status_id = new_status;
        }
    }
    
    if changes.is_empty() {
        if !json_output {
            println!("No changes needed for: {}", resolved_path.display());
        }
        return Ok(());
    }
    
    if dry_run {
        if json_output {
            println!("{{\"dry_run\": true, \"changes\": {:?}}}", changes);
        } else {
            println!("DRY RUN - Would make the following changes to {}:", resolved_path.display());
            for change in changes {
                println!("  - {}", change);
            }
        }
        return Ok(());
    }
    
    let new_content = serde_json::to_string_pretty(&meta)
        .context("Failed to serialize meta file")?;
    
    fs::write(&resolved_path, new_content)
        .with_context(|| format!("Failed to write file: {:?}", resolved_path))?;
    
    if json_output {
        println!("{{\"success\": true, \"file\": \"{}\", \"changes\": {}}}", 
            resolved_path.display(), changes.len());
    } else {
        println!("Successfully updated: {}", resolved_path.display());
        for change in changes {
            println!("  - {}", change);
        }
    }
    
    Ok(())
}

async fn handle_batch(input: PathBuf, json_output: bool, dry_run: bool) -> Result<()> {
    let content = fs::read_to_string(&input)
        .with_context(|| format!("Failed to read batch input file: {:?}", input))?;
    
    let batch: BatchInput = serde_json::from_str(&content)
        .context("Failed to parse batch input JSON")?;
    
    let mut results = Vec::new();
    let mut success_count = 0;
    let mut error_count = 0;
    
    for op in batch.operations {
        let file_path = PathBuf::from(&op.file);
        let result = handle_set(
            file_path,
            op.specified,
            op.ignored,
            op.verified,
            false, 
            dry_run,
        )
        .await;
        
        match result {
            Ok(_) => {
                success_count += 1;
                results.push(BatchResult {
                    file: op.file,
                    success: true,
                    error: None,
                });
            }
            Err(e) => {
                error_count += 1;
                results.push(BatchResult {
                    file: op.file,
                    success: false,
                    error: Some(e.to_string()),
                });
            }
        }
    }
    
    if json_output {
        let output = BatchOutput {
            success_count,
            error_count,
            results,
        };
        println!("{}", serde_json::to_string_pretty(&output)?);
    } else {
        println!("Batch operation completed:");
        println!("  Success: {}", success_count);
        println!("  Errors:  {}", error_count);
        for result in results {
            if result.success {
                println!("  ✓ {}", result.file);
            } else {
                println!("  ✗ {} - {}", result.file, result.error.unwrap_or_default());
            }
        }
    }
    
    if error_count > 0 {
        std::process::exit(1);
    }
    
    Ok(())
}

fn validate_meta_file(file: &PathBuf) -> Result<()> {
    if !file.exists() {
        anyhow::bail!("File not found: {:?}", file);
    }
    
    if !file.is_file() {
        anyhow::bail!("Path is not a file: {:?}", file);
    }
    
    let file_name = file.file_name().unwrap_or_default().to_string_lossy();
    if !file_name.contains(".meta.verilib") {
        anyhow::bail!("File is not a .meta.verilib file: {:?}", file);
    }
    
    Ok(())
}

fn check_admin_status() -> Result<()> {
    let config_path = PathBuf::from(".verilib/config.json");

    if !config_path.exists() {
        anyhow::bail!("No config.json found. Please run 'init' first.");
    }

    let content = fs::read_to_string(&config_path)
        .context("Failed to read config.json")?;

    #[derive(Deserialize)]
    struct TempMeta {
        repo: TempRepo,
    }
    #[derive(Deserialize)]
    struct TempRepo {
        #[serde(default)]
        is_admin: bool,
    }

    let meta: TempMeta = serde_json::from_str(&content)
        .context("Failed to parse config.json")?;

    if !meta.repo.is_admin {
        anyhow::bail!("Admin access required to modify verified status");
    }

    Ok(())
}


fn resolve_file_path(input: &PathBuf) -> Result<PathBuf> {
    use regex::Regex;
    
    let input_str = input.to_string_lossy().to_string();
    let mut path = input_str.clone();
    
    if path.starts_with(".verilib/") {
        path = path.strip_prefix(".verilib/").unwrap().to_string();
    } else if path.starts_with(".verilib\\") {
        path = path.strip_prefix(".verilib\\").unwrap().to_string();
    }
    
    let path_buf = PathBuf::from(&path);
    let parent = path_buf.parent();
    let filename = path_buf.file_name().unwrap_or_default().to_string_lossy();
    
    let re = Regex::new(r"^\[\d+\]\s*-\s*").unwrap();
    let clean_filename = re.replace(&filename, "").to_string();
    
    let final_filename = if clean_filename.ends_with(".meta.verilib") {
        clean_filename
    } else if clean_filename.ends_with(".verilib") {
        clean_filename.replace(".verilib", ".meta.verilib")
    } else {
        format!("{}.meta.verilib", clean_filename)
    };
    
    let resolved = if let Some(parent_path) = parent {
        PathBuf::from(".verilib")
            .join(parent_path)
            .join(&final_filename)
    } else {
        PathBuf::from(".verilib").join(&final_filename)
    };
    
    if let Some(parent_dir) = resolved.parent() {
        if parent_dir.exists() {
            if let Ok(entries) = fs::read_dir(parent_dir) {
                for entry in entries.flatten() {
                    let entry_name = entry.file_name().to_string_lossy().to_string();
                    let entry_clean = re.replace(&entry_name, "").to_string();
                    if entry_clean == final_filename {
                        return Ok(entry.path());
                    }
                }
            }
        }
    }
    
    Ok(resolved)
}
