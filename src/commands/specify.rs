//! Specify subcommand implementation.
//!
//! Check specification status and manage spec certs.

use crate::structure::{
    create_cert, display_menu, get_existing_certs, require_probe_installed, run_command,
    ConfigPaths,
};
use anyhow::{bail, Context, Result};
use serde_json::Value;
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

/// Run the specify subcommand.
///
/// Flow:
/// 1. Load stubs from stubs.json
/// 2. Run probe-verus specify to get spec info for each function
/// 3. Enrich stubs with spec-text from specs data (only for specified functions)
/// 4. Find stubs with spec-text that need certification
/// 5. Display menu and create certs for selected functions
/// 6. Update specified status in stubs based on certification
/// 7. Write updated stubs back to stubs.json
pub async fn handle_specify(project_root: PathBuf, no_probe: bool, check_only: bool) -> Result<()> {
    let project_root = project_root
        .canonicalize()
        .context("Failed to resolve project root")?;
    let config = ConfigPaths::load(&project_root)?;

    // Load stubs from stubs.json
    let mut stubs_data = read_stubs_json(&config.structure_json_path)?;
    println!("Loaded {} stubs from stubs.json", stubs_data.len());

    // Run probe-verus specify or load from existing file
    let specs_path = config.verilib_path.join("specs.json");
    let specs_data = if no_probe {
        load_specs_from_file(&specs_path)?
    } else {
        run_probe_specify(&project_root, &specs_path, &config.atoms_path)?
    };

    // Enrich stubs with spec-text (only for functions where specified=true)
    incorporate_spec_text(&mut stubs_data, &specs_data);

    // Find stubs with spec-text that are not yet certified
    let existing_certs = get_existing_certs(&config.certs_specify_dir)?;
    println!("Found {} existing certs", existing_certs.len());
    let uncertified = find_uncertified_functions(&stubs_data, &existing_certs);

    // If check_only, verify all stubs with specs have certs
    if check_only {
        return check_all_certified(&uncertified);
    }

    // Display menu and create certs for selected functions
    let newly_certified = collect_certifications(&uncertified, &config.certs_specify_dir)?;

    // Update specified status based on all certified functions
    let all_certified: HashSet<String> = existing_certs.union(&newly_certified).cloned().collect();
    update_stubs_specification_status(&mut stubs_data, &all_certified);

    // Write updated stubs back to stubs.json
    write_stubs_json(&config.structure_json_path, &stubs_data)?;

    println!("Done.");
    Ok(())
}

/// Check if all stubs with specs have certs.
/// Returns Ok if all are certified, error with list of uncertified stubs otherwise.
fn check_all_certified(uncertified: &HashMap<String, Value>) -> Result<()> {
    if uncertified.is_empty() {
        println!("All stubs with specs have certs.");
        return Ok(());
    }

    eprintln!(
        "Found {} stubs with specs missing certs:",
        uncertified.len()
    );

    let mut uncertified_list: Vec<_> = uncertified.iter().collect();
    uncertified_list.sort_by(|a, b| a.0.cmp(b.0));

    for (stub_path, stub) in &uncertified_list {
        let code_name = stub
            .get("code-name")
            .and_then(|v| v.as_str())
            .unwrap_or("?");
        let display_name = stub
            .get("display-name")
            .and_then(|v| v.as_str())
            .unwrap_or("?");
        eprintln!("  {}: {} ({})", stub_path, display_name, code_name);
    }

    bail!(
        "{} stubs with specs are missing certs. Run 'specify' to certify them.",
        uncertified.len()
    );
}

/// Find stubs with spec-text that are not yet certified.
fn find_uncertified_functions(
    stubs_data: &HashMap<String, Value>,
    existing_certs: &HashSet<String>,
) -> HashMap<String, Value> {
    // Find stubs which have "spec-text" field
    let stubs_with_specs: HashMap<String, Value> = stubs_data
        .iter()
        .filter(|(_, stub)| stub.get("spec-text").is_some())
        .map(|(k, v)| (k.clone(), v.clone()))
        .collect();
    println!("\nFound {} stubs with spec-text", stubs_with_specs.len());

    // Filter out existing certs (by code-name)
    let uncertified: HashMap<String, Value> = stubs_with_specs
        .into_iter()
        .filter(|(_, stub)| {
            let code_name = stub
                .get("code-name")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            !existing_certs.contains(code_name)
        })
        .collect();

    println!("Found {} stubs needing certification", uncertified.len());

    uncertified
}

/// Display menu for uncertified functions and create certs for selected ones.
/// Returns the set of newly certified code-names.
fn collect_certifications(
    uncertified: &HashMap<String, Value>,
    certs_dir: &Path,
) -> Result<HashSet<String>> {
    let mut newly_certified = HashSet::new();

    if uncertified.is_empty() {
        println!("\nAll functions with specs in structure are already validated!");
        return Ok(newly_certified);
    }

    println!(
        "\n{} functions with specs need certification",
        uncertified.len()
    );

    let mut uncertified_list: Vec<(String, Value)> = uncertified
        .iter()
        .map(|(k, v)| (k.clone(), v.clone()))
        .collect();
    uncertified_list.sort_by(|a, b| a.0.cmp(&b.0));

    let selected_indices = display_menu(&uncertified_list, |i, _stub_path, stub| {
        let display_name = stub
            .get("display-name")
            .and_then(|v| v.as_str())
            .unwrap_or("?");
        let code_path = stub
            .get("code-path")
            .and_then(|v| v.as_str())
            .unwrap_or("?");
        let spec_text = stub.get("spec-text");
        let lines_start = spec_text
            .and_then(|v| v.get("lines-start"))
            .and_then(|v| v.as_u64())
            .map(|l| l.to_string())
            .unwrap_or_else(|| "?".to_string());
        let lines_end = spec_text
            .and_then(|v| v.get("lines-end"))
            .and_then(|v| v.as_u64())
            .map(|l| l.to_string())
            .unwrap_or_else(|| "?".to_string());

        format!(
            "  [{}] {} ({}#L{}-L{})",
            i, display_name, code_path, lines_start, lines_end
        )
    })?;

    if selected_indices.is_empty() {
        println!("\nNo functions selected.");
        return Ok(newly_certified);
    }

    println!(
        "\nCreating certs for {} functions...",
        selected_indices.len()
    );

    for idx in &selected_indices {
        let (_stub_path, stub) = &uncertified_list[*idx];
        let code_name = stub
            .get("code-name")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        newly_certified.insert(code_name.to_string());
        let cert_path = create_cert(certs_dir, code_name)?;
        println!(
            "  Created: {}",
            cert_path.file_name().unwrap_or_default().to_string_lossy()
        );
    }

    println!(
        "\nCreated {} cert files in {}",
        selected_indices.len(),
        certs_dir.display()
    );

    Ok(newly_certified)
}

/// Load specs from an existing specs.json file.
fn load_specs_from_file(specs_path: &Path) -> Result<HashMap<String, Value>> {
    if !specs_path.exists() {
        bail!(
            "specs.json not found at {}. Run without --no-probe first to generate it.",
            specs_path.display()
        );
    }

    println!("Loading specs from {}...", specs_path.display());
    let content = std::fs::read_to_string(specs_path)
        .with_context(|| format!("Failed to read {}", specs_path.display()))?;
    let specs: HashMap<String, Value> = serde_json::from_str(&content)
        .with_context(|| format!("Failed to parse {}", specs_path.display()))?;
    println!("Loaded {} specs", specs.len());
    Ok(specs)
}

/// Run probe-verus specify and return the results.
fn run_probe_specify(
    project_root: &Path,
    specs_path: &Path,
    atoms_path: &Path,
) -> Result<HashMap<String, Value>> {
    require_probe_installed()?;

    if let Some(parent) = specs_path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    println!(
        "Running probe-verus specify on {}...",
        project_root.display()
    );

    let output = run_command(
        "probe-verus",
        &[
            "specify",
            project_root.to_str().unwrap(),
            "-o",
            specs_path.to_str().unwrap(),
            "-a",
            atoms_path.to_str().unwrap(),
        ],
        Some(project_root),
    )?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        eprintln!("Error: probe-verus specify failed.");
        if !stderr.is_empty() {
            eprintln!("{}", stderr);
        }
        bail!("probe-verus specify failed");
    }

    println!("Specs saved to {}", specs_path.display());

    let content = std::fs::read_to_string(specs_path)?;
    let specs: HashMap<String, Value> = serde_json::from_str(&content)?;
    Ok(specs)
}

/// Update stubs_data with specification statuses based on certified names.
fn update_stubs_specification_status(
    stubs_data: &mut HashMap<String, Value>,
    certified_names: &HashSet<String>,
) {
    for entry in stubs_data.values_mut() {
        if let Some(obj) = entry.as_object_mut() {
            let specified = obj
                .get("code-name")
                .and_then(|v| v.as_str())
                .map(|name| certified_names.contains(name))
                .unwrap_or(false);

            obj.insert("specified".to_string(), Value::Bool(specified));
        }
    }

    println!(
        "Updated specification status for {} stubs",
        stubs_data.len()
    );
}

/// Read stubs.json into a HashMap.
fn read_stubs_json(stubs_path: &Path) -> Result<HashMap<String, Value>> {
    if !stubs_path.exists() {
        return Ok(HashMap::new());
    }

    let content = std::fs::read_to_string(stubs_path)?;
    let stubs: HashMap<String, Value> = serde_json::from_str(&content)?;
    Ok(stubs)
}

/// Write stubs_data to stubs.json.
fn write_stubs_json(stubs_path: &Path, stubs_data: &HashMap<String, Value>) -> Result<()> {
    let content = serde_json::to_string_pretty(stubs_data)?;
    std::fs::write(stubs_path, content)?;
    println!("Wrote stubs to {}", stubs_path.display());
    Ok(())
}

/// Incorporate spec-text from specs_data into stubs_data.
/// For each stub with a code-name, look up code-name in specs_data
/// and add "spec-text" field if specified is true.
fn incorporate_spec_text(
    stubs_data: &mut HashMap<String, Value>,
    specs_data: &HashMap<String, Value>,
) {
    let mut count = 0;
    for stub in stubs_data.values_mut() {
        if let Some(obj) = stub.as_object_mut() {
            let code_name = obj
                .get("code-name")
                .and_then(|v| v.as_str())
                .unwrap_or("");

            if let Some(spec_info) = specs_data.get(code_name) {
                // Only add spec-text if specified is true
                let is_specified = spec_info
                    .get("specified")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(false);

                if is_specified {
                    if let Some(spec_text) = spec_info.get("spec-text") {
                        obj.insert("spec-text".to_string(), spec_text.clone());
                        count += 1;
                    }
                }
            }
        }
    }
    println!("Incorporated spec-text for {} stubs", count);
}
