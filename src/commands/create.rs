//! Create subcommand implementation.
//!
//! Initialize structure files from source analysis using probe-verus.

use crate::structure::{
    run_command, write_frontmatter, CommandConfig, ExternalTool,
};
use crate::config::ProjectConfig;
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

    let mut config = ProjectConfig::load(&project_root)?;
    config.structure_root = Some(structure_root_relative.clone());
    let config_path = config.save(&project_root)?;
    println!("Wrote config to {}", config_path.display());

    let tracked_output_path = verilib_path.join("tracked_functions.csv");

    let cmd_config = config.command_config();
    run_probe_verus_tracked_csv(&project_root, &tracked_output_path, &cmd_config)?;

    let tracked = read_tracked_csv(&tracked_output_path)?;
    let tracked = disambiguate_names(tracked);
    let structure = tracked_to_structure(&tracked);

    println!("\nGenerating structure files...");
    let structure_root = project_root.join(&structure_root_relative);
    generate_structure_files(&structure, &structure_root)?;

    Ok(())
}

/// Run `probe-verus tracked-csv` to generate the tracked functions CSV.
///
/// Called without `--github-base-url` so the link column contains bare
/// `file_path#Lline` values that `parse_tracked_link` can parse directly.
fn run_probe_verus_tracked_csv(
    project_root: &Path,
    output_path: &Path,
    config: &CommandConfig,
) -> Result<()> {
    if let Some(parent) = output_path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    println!("Running probe-verus tracked-csv...");

    let output_relative = output_path
        .strip_prefix(project_root)
        .unwrap_or(output_path);

    let output_str = output_relative
        .to_str()
        .context("Output path contains non-UTF-8 characters")?;

    let output = run_command(
        &ExternalTool::Probe,
        &["tracked-csv", ".", "--output", output_str],
        Some(project_root),
        config,
    )?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        eprintln!("Error: probe-verus tracked-csv failed.");
        if !stderr.is_empty() {
            eprintln!("{}", stderr);
        }
        bail!("probe-verus tracked-csv failed");
    }

    println!(
        "Generated tracked functions CSV at {}",
        output_path.display()
    );
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
    let mut reader = csv::Reader::from_path(csv_path)
        .with_context(|| format!("Failed to read tracked functions from {}", csv_path.display()))?;

    for result in reader.records() {
        let record = result?;
        let function = record.get(0).unwrap_or("").to_string();
        let module = record.get(1).unwrap_or("").to_string();
        let link = record.get(2).unwrap_or("").to_string();

        let result_key = format!("{}::{}", function, module);
        if results.contains_key(&result_key) {
            eprintln!(
                "Warning: duplicate CSV entry for '{}', later row overwrites earlier",
                result_key
            );
        }
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
///
/// Iterates in sorted key order so that suffix indices (`_0`, `_1`, ...) are
/// assigned deterministically across runs.
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

    let mut sorted_entries: Vec<_> = tracked.into_iter().collect();
    sorted_entries.sort_by(|(a, _), (b, _)| a.cmp(b));

    for (key, mut func) in sorted_entries {
        if duplicates.contains(&func.qualified_name) {
            let idx = name_indices
                .get_mut(&func.qualified_name)
                .expect("duplicate name must have an entry in name_indices");
            func.qualified_name = format!("{}_{}", func.qualified_name, idx);
            *idx += 1;
        }
        new_tracked.insert(key, func);
    }

    new_tracked
}

/// Parse a tracked-csv link into (file_path, line_number).
///
/// probe-verus produces bare links like `src/module.rs#L42` (path with
/// optional `#L<line>` suffix).
fn parse_tracked_link(link: &str) -> Option<(String, u32)> {
    if link.is_empty() {
        return None;
    }

    if let Some((code_path, line_str)) = link.rsplit_once("#L") {
        let line_number: u32 = line_str.parse().ok()?;
        if code_path.is_empty() {
            return None;
        }
        Some((code_path.to_string(), line_number))
    } else {
        Some((link.to_string(), 0))
    }
}

/// Convert tracked functions to a structure dictionary.
fn tracked_to_structure(tracked: &HashMap<String, TrackedFunction>) -> HashMap<String, Value> {
    let mut result = HashMap::new();

    for func in tracked.values() {
        if let Some((code_path, line_start)) = parse_tracked_link(&func.link) {
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

#[cfg(test)]
mod tests {
    use super::*;

    // --- parse_tracked_link ---

    #[test]
    fn test_parse_bare_link_with_line() {
        assert_eq!(
            parse_tracked_link("src/module.rs#L42"),
            Some(("src/module.rs".to_string(), 42))
        );
    }

    #[test]
    fn test_parse_bare_link_nested_path() {
        assert_eq!(
            parse_tracked_link("src/deeply/nested/file.rs#L99"),
            Some(("src/deeply/nested/file.rs".to_string(), 99))
        );
    }

    #[test]
    fn test_parse_bare_link_no_line() {
        assert_eq!(
            parse_tracked_link("src/module.rs"),
            Some(("src/module.rs".to_string(), 0))
        );
    }


    #[test]
    fn test_parse_empty_link() {
        assert_eq!(parse_tracked_link(""), None);
    }

    #[test]
    fn test_parse_hash_l_only() {
        assert_eq!(parse_tracked_link("#L10"), None);
    }

    // --- disambiguate_names ---

    #[test]
    fn test_disambiguate_names_deterministic() {
        let mut tracked = HashMap::new();
        tracked.insert(
            "dup::mod_b".to_string(),
            TrackedFunction {
                link: String::new(),
                qualified_name: "dup".into(),
            },
        );
        tracked.insert(
            "dup::mod_a".to_string(),
            TrackedFunction {
                link: String::new(),
                qualified_name: "dup".into(),
            },
        );

        let result = disambiguate_names(tracked);
        assert_eq!(result["dup::mod_a"].qualified_name, "dup_0");
        assert_eq!(result["dup::mod_b"].qualified_name, "dup_1");
    }

    #[test]
    fn test_disambiguate_names_no_duplicates_unchanged() {
        let mut tracked = HashMap::new();
        tracked.insert(
            "foo::mod_a".to_string(),
            TrackedFunction {
                link: String::new(),
                qualified_name: "foo".into(),
            },
        );
        tracked.insert(
            "bar::mod_b".to_string(),
            TrackedFunction {
                link: String::new(),
                qualified_name: "bar".into(),
            },
        );

        let result = disambiguate_names(tracked);
        assert_eq!(result["foo::mod_a"].qualified_name, "foo");
        assert_eq!(result["bar::mod_b"].qualified_name, "bar");
    }
}
