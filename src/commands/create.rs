//! Create subcommand implementation.
//!
//! Initialize structure files from source analysis.

use crate::structure::{
    parse_github_link, run_command, write_frontmatter, CommandConfig, ConfigPaths, ExecutionMode, StructureConfig,
};
use anyhow::{bail, Context, Result};
use serde_json::{json, Value};
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

/// Run the create subcommand.
pub async fn handle_create(project_root: PathBuf, root: Option<PathBuf>) -> Result<()> {
    let project_root = project_root
        .canonicalize()
        .context("Failed to resolve project root")?;
    let verilib_path = project_root.join(".verilib");
    std::fs::create_dir_all(&verilib_path).context("Failed to create .verilib directory")?;

    let structure_root_relative = root
        .map(|r| r.to_string_lossy().to_string())
        .unwrap_or_else(|| ".verilib/structure".to_string());

    // Write config file with ONLY structure-root field
    let config = StructureConfig::new(&structure_root_relative);
    let config_path = config.save(&project_root, true)?;
    println!("Wrote config to {}", config_path.display());

    // NOTE: .gitignore creation is moved to the 'init' subcommand

    let config = ConfigPaths::load(&project_root)?;

    let tracked_path = project_root.join("functions_to_track.csv");
    if !tracked_path.exists() {
        println!("functions_to_track.csv not found, generating from atomize...");
        crate::commands::atomize::handle_atomize(
            project_root.clone(),
            false,
            false, 
            false,
        )
        .await?;

        let atoms_path = verilib_path.join("atoms.json");
        if atoms_path.exists() {
            generate_functions_to_track_csv(&atoms_path, &tracked_path)?;
        } else {
             bail!("Failed to generate atoms.json for functions_to_track.csv");
        }
    }

    let tracked_output_path = verilib_path.join("tracked_functions.csv");

    run_analyze_verus_specs_proofs(&project_root, &tracked_path, &tracked_output_path, &config.command_config)?;

    let tracked = read_tracked_csv(&tracked_output_path)?;
    let tracked = disambiguate_names(tracked);
    let structure = tracked_to_structure(&tracked);

    // Generate structure files
    println!("\nGenerating structure files...");
    let structure_root = project_root.join(&structure_root_relative);
    generate_structure_files(&structure, &structure_root)?;

    Ok(())
}

/// Run analyze_verus_specs_proofs.py CLI to generate tracked functions CSV.
fn run_analyze_verus_specs_proofs(
    project_root: &Path,
    seed_path: &Path,
    output_path: &Path,
    config: &CommandConfig,
) -> Result<()> {
    let script_name = "analyze_verus_specs_proofs.py";
    let script_path = if matches!(config.execution_mode, ExecutionMode::Docker) {
        let workspace_script = PathBuf::from("/workspace/scripts").join(script_name);
        
        if project_root.join("scripts").join(script_name).exists() {
             workspace_script
        } else {
             PathBuf::from("/usr/local/bin/scripts").join(script_name)
        }
    } else {
        let path = project_root.join("scripts").join(script_name);
        if !path.exists() {
            bail!("Script not found locally: {}", path.display());
        }
        path
    };

    println!("Running {}...", script_name);

    let seed_arg = if matches!(config.execution_mode, ExecutionMode::Docker) {
        seed_path.strip_prefix(project_root).unwrap_or(seed_path).to_string_lossy().to_string()
    } else {
        seed_path.to_string_lossy().to_string()
    };

    let output_arg = if matches!(config.execution_mode, ExecutionMode::Docker) {
        output_path.strip_prefix(project_root).unwrap_or(output_path).to_string_lossy().to_string()
    } else {
        output_path.to_string_lossy().to_string()
    };

    // Ensure parent directory exists (locally)
    if let Some(parent) = output_path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    
    let (seed_flag, output_flag) = if matches!(config.execution_mode, ExecutionMode::Docker) {
        (
             format!("/workspace/{}", seed_arg),
             format!("/workspace/{}", output_arg),
        )
    } else {
        (seed_arg.clone(), output_arg.clone())
    };

    let script_path_str = script_path.to_string_lossy();
    let args = vec![
        "run",
        &script_path_str,
        "--seed",
        &seed_flag,
        "--output",
        &output_flag,
    ];
    
    let output = run_command(
        "uv",
        &args,
        Some(project_root),
        config,
    )?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        eprintln!("Error running {}:\n{}", script_name, stderr);
        bail!("{} failed", script_name);
    }
    
    let stdout = String::from_utf8_lossy(&output.stdout);
    println!("{} output:\n{}", script_name, stdout);

    println!(
        "Generated tracked functions CSV at {}",
        output_path.display()
    );
    Ok(())
}

/// Generate functions_to_track.csv from atoms.json
fn generate_functions_to_track_csv(atoms_path: &Path, output_path: &Path) -> Result<()> {
    let file = std::fs::File::open(atoms_path)?;
    let reader = std::io::BufReader::new(file);
    let atoms: HashMap<String, Value> = serde_json::from_reader(reader)?;

    let mut wtr = csv::Writer::from_path(output_path)?;
    wtr.write_record(&["function", "module", "impl_block"])?;

    for (key, val) in atoms {
        if !key.starts_with("probe:") {
            continue;
        }

        let parts: Vec<&str> = key.split('/').collect();
        if parts.len() < 3 { continue; }
        
        let project_part = parts[0];
        let project_name = project_part.strip_prefix("probe:").unwrap_or(project_part);

        let func_part = parts.last().unwrap();
        
        let function = val.get("display-name")
            .and_then(|v| v.as_str())
            .unwrap_or(func_part)
            .to_string() + "()";

        if parts.len() <= 2 {
            continue; 
        }
        
        let dir_parts = &parts[2..parts.len()-1];
        if dir_parts.is_empty() {
             continue;
        }

        let mut rev_parts: Vec<&str> = dir_parts.to_vec();
        rev_parts.reverse();
        
        let mut module_parts = vec![project_name];
        module_parts.extend(rev_parts);
        let module = module_parts.join("::");

        wtr.write_record(&[&function, &module, ""])?;
    }
    
    wtr.flush()?;
    println!("Generated functions_to_track.csv at {}", output_path.display());
    Ok(())
}


/// Tracked function data from CSV.
#[derive(Debug, Clone)]
struct TrackedFunction {
    link: String,
    qualified_name: String,
}

/// Read tracked functions CSV and return a HashMap.
fn read_tracked_csv(csv_path: &Path) -> Result<HashMap<String, TrackedFunction>> {
    let mut results = HashMap::new();
    let mut reader = csv::Reader::from_path(csv_path)?;

    for result in reader.records() {
        let record = result?;
        let function = record.get(0).unwrap_or("").to_string();
        let module = record.get(1).unwrap_or("").to_string();
        let link = record.get(2).unwrap_or("").to_string();

        let result_key = format!("{}::{}", function, module);
        results.insert(
            result_key,
            TrackedFunction {
                link,
                qualified_name: function,
            },
        );
    }

    Ok(results)
}

/// Disambiguate tracked items that have the same qualified_name.
fn disambiguate_names(
    tracked: HashMap<String, TrackedFunction>,
) -> HashMap<String, TrackedFunction> {
    let mut name_counts: HashMap<String, usize> = HashMap::new();
    for func in tracked.values() {
        *name_counts.entry(func.qualified_name.clone()).or_insert(0) += 1;
    }

    let duplicates: HashSet<_> = name_counts
        .into_iter()
        .filter(|(_, count)| *count > 1)
        .map(|(name, _)| name)
        .collect();

    if duplicates.is_empty() {
        return tracked;
    }

    let mut name_indices: HashMap<String, usize> =
        duplicates.iter().map(|n| (n.clone(), 0)).collect();
    let mut new_tracked = HashMap::new();

    for (key, mut func) in tracked {
        if duplicates.contains(&func.qualified_name) {
            let idx = name_indices.get_mut(&func.qualified_name).unwrap();
            func.qualified_name = format!("{}_{}", func.qualified_name, idx);
            *idx += 1;
        }
        new_tracked.insert(key, func);
    }

    new_tracked
}

/// Convert tracked functions to a structure dictionary.
fn tracked_to_structure(tracked: &HashMap<String, TrackedFunction>) -> HashMap<String, Value> {
    let mut result = HashMap::new();

    for func in tracked.values() {
        if let Some((code_path, line_start)) = parse_github_link(&func.link) {
            if code_path.is_empty() {
                continue;
            }

            let func_name = func.qualified_name.replace("::", ".");
            let file_path = format!("{}/{}.md", code_path, func_name);

            result.insert(
                file_path,
                json!({
                    "code-line": line_start,
                    "code-path": code_path,
                    "code-name": null,
                }),
            );
        }
    }

    result
}

/// Generate structure .md files from a structure dictionary.
fn generate_structure_files(
    structure: &HashMap<String, Value>,
    structure_root: &Path,
) -> Result<()> {
    let mut created_count = 0;

    for (relative_path_str, metadata) in structure {
        let file_path = structure_root.join(relative_path_str);

        if file_path.exists() {
            eprintln!(
                "WARNING: File already exists, overwriting: {}",
                file_path.display()
            );
        }

        let mut metadata_map: HashMap<String, Value> = if let Some(obj) = metadata.as_object() {
            obj.iter().map(|(k, v)| (k.clone(), v.clone())).collect()
        } else {
            HashMap::new()
        };

        let body_content = metadata_map.remove("content");
        let body = body_content.as_ref().and_then(|v| v.as_str());

        write_frontmatter(&file_path, &metadata_map, body)?;
        created_count += 1;
    }

    println!(
        "Created {} structure files in {}",
        created_count,
        structure_root.display()
    );
    Ok(())
}