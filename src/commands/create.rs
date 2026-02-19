//! Create subcommand implementation.
//!
//! Initialize structure files from source analysis.

use crate::structure::{
    parse_github_link, run_command, write_frontmatter, CommandConfig, ExecutionMode, StructureConfig,
};
use anyhow::{Context, Result};
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
    let config_path = config.save(&project_root)?;
    println!("Wrote config to {}", config_path.display());

    // NOTE: .gitignore creation is moved to the 'init' subcommand

    let tracked_path = project_root.join("functions_to_track.csv");
    let seed_path = if tracked_path.exists() {
        Some(tracked_path)
    } else {
        // Default: track all functions when functions_to_track.csv is absent
        None
    };

    let tracked_output_path = verilib_path.join("tracked_functions.csv");

    // Script is optional: if absent or fails, create proceeds with no structure files
    let local_config = CommandConfig {
        execution_mode: ExecutionMode::Local,
        ..Default::default()
    };
    let tracked = match try_run_analyze_verus_specs_proofs(
        &project_root,
        seed_path.as_deref(),
        &tracked_output_path,
        &local_config,
    ) {
        Some(path) => read_tracked_csv(&path)?,
        None => {
            println!("Skipping structure generation (analyze_verus_specs_proofs.py not run)");
            HashMap::new()
        }
    };
    let tracked = disambiguate_names(tracked);
    let structure = tracked_to_structure(&tracked);

    // Generate structure files
    println!("\nGenerating structure files...");
    let structure_root = project_root.join(&structure_root_relative);
    generate_structure_files(&structure, &structure_root)?;

    Ok(())
}

/// Path to the script bundled with verilib-cli (scripts/ relative to the executable).
fn cli_bundled_script_path() -> Option<PathBuf> {
    let exe = std::env::current_exe().ok()?;
    let cli_root = exe.parent()?.parent()?; // target/release/verilib-cli -> verilib-cli root
    let script = cli_root.join("scripts").join("analyze_verus_specs_proofs.py");
    script.exists().then_some(script)
}

/// Optionally run analyze_verus_specs_proofs.py. Returns Some(output_path) on success, None if
/// script is absent or fails (create continues without structure files).
fn try_run_analyze_verus_specs_proofs(
    project_root: &Path,
    seed_path: Option<&Path>,
    output_path: &Path,
    config: &CommandConfig,
) -> Option<PathBuf> {
    let project_script = project_root
        .join("scripts")
        .join("analyze_verus_specs_proofs.py");
    let script_path: PathBuf = project_script
        .exists()
        .then_some(project_script)
        .or_else(cli_bundled_script_path)?;

    println!("Running analyze_verus_specs_proofs.py...");

    let output_relative = output_path
        .strip_prefix(project_root)
        .unwrap_or(output_path);

    if let Some(parent) = output_path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }

    let mut args: Vec<String> = vec![
        "run".into(),
        script_path.to_str()?.into(),
        "--output".into(),
        output_relative.to_str()?.into(),
    ];
    if let Some(seed) = seed_path {
        let seed_relative = seed.strip_prefix(project_root).unwrap_or(seed);
        args.extend(["--seed".into(), seed_relative.to_str()?.into()]);
    }

    let args_ref: Vec<&str> = args.iter().map(String::as_str).collect();
    let output = run_command("uv", &args_ref, Some(project_root), config).ok()?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        eprintln!(
            "Warning: analyze_verus_specs_proofs.py failed (skipping structure generation):\n{}",
            stderr
        );
        return None;
    }

    println!(
        "Generated tracked functions CSV at {}",
        output_path.display()
    );
    Some(output_path.to_path_buf())
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
    std::fs::create_dir_all(structure_root)
        .context("Failed to create structure root directory")?;
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
