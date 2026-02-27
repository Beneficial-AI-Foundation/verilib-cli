//! Verify subcommand implementation.
//!
//! Run verification and update stubs.json with verification status.

use crate::config::ProjectConfig;
use crate::structure::{
    cleanup_intermediate_files, get_display_name, run_command, CommandConfig, ExternalTool,
    VERIFY_INTERMEDIATE_FILES,
};
use anyhow::{bail, Context, Result};
use serde_json::Value;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// Run the verify subcommand.
pub async fn handle_verify(
    project_root: PathBuf,
    package: Option<String>,
    verify_only_module: Option<String>,
    no_probe: bool,
    check_only: bool,
) -> Result<()> {
    let project_root = project_root
        .canonicalize()
        .context("Failed to resolve project root")?;
    ProjectConfig::init(&project_root)?;
    let config = ProjectConfig::global().unwrap();
    let stubs_path = config.stubs_path();
    let atoms_path = config.atoms_path();
    let cmd_config = config.command_config();

    // Load existing stubs.json
    if !stubs_path.exists() {
        bail!(
            "{} not found. Run 'verilib-cli atomize' first.",
            stubs_path.display()
        );
    }
    let stubs_content = std::fs::read_to_string(&stubs_path)?;
    let mut stubs: HashMap<String, Value> = serde_json::from_str(&stubs_content)?;

    // If check_only, just check for failures in existing stubs
    if check_only {
        println!("Checking stubs for verification failures...");
        return check_for_failures(&stubs);
    }

    // Run probe-verus verify or load from existing file
    let proofs_path = config.verilib_path().join("proofs.json");
    let proofs_data = if no_probe {
        load_proofs_from_file(&proofs_path)?
    } else {
        run_probe_verify(
            &project_root,
            &proofs_path,
            &atoms_path,
            package.as_deref(),
            verify_only_module.as_deref(),
            &cmd_config,
        )?
    };

    // Update stubs with verification status
    let (newly_verified, newly_unverified) =
        update_stubs_with_verification(&mut stubs, &proofs_data);

    // Save updated stubs.json
    let stubs_content = serde_json::to_string_pretty(&stubs)?;
    std::fs::write(&stubs_path, stubs_content)?;
    println!("\nUpdated {}", stubs_path.display());

    // Print summary
    print_verification_summary(&newly_verified, &newly_unverified);

    Ok(())
}

/// Check if any stub has status "failure".
/// Returns Ok if no failures, error with list of failed stubs otherwise.
fn check_for_failures(stubs: &HashMap<String, Value>) -> Result<()> {
    let mut failed_stubs: Vec<(String, String, String)> = Vec::new();

    for (stub_path, stub_data) in stubs {
        let status = stub_data
            .get("status")
            .and_then(|v| v.as_str())
            .unwrap_or("");

        if status == "failure" {
            let display_name = stub_data
                .get("display-name")
                .and_then(|v| v.as_str())
                .unwrap_or("?")
                .to_string();
            let code_name = stub_data
                .get("code-name")
                .and_then(|v| v.as_str())
                .unwrap_or("?")
                .to_string();
            failed_stubs.push((stub_path.clone(), display_name, code_name));
        }
    }

    if failed_stubs.is_empty() {
        println!("All {} stubs passed verification.", stubs.len());
        return Ok(());
    }

    failed_stubs.sort_by(|a, b| a.0.cmp(&b.0));

    eprintln!(
        "Found {} stubs with status \"failure\":",
        failed_stubs.len()
    );
    for (stub_path, display_name, code_name) in &failed_stubs {
        eprintln!("  {}: {} ({})", stub_path, display_name, code_name);
    }

    bail!(
        "{} stubs failed verification. Run 'verify' to update verification status.",
        failed_stubs.len()
    );
}

/// Update stubs with verification status from proofs data.
/// Returns (newly_verified, newly_unverified) lists.
fn update_stubs_with_verification(
    stubs: &mut HashMap<String, Value>,
    proofs_data: &HashMap<String, Value>,
) -> (Vec<String>, Vec<String>) {
    let mut newly_verified = Vec::new();
    let mut newly_unverified = Vec::new();

    for (stub_name, stub_data) in stubs.iter_mut() {
        let stub_obj = match stub_data.as_object_mut() {
            Some(obj) => obj,
            None => continue,
        };

        // Get the code-name for this stub
        let code_name = match stub_obj.get("code-name").and_then(|v| v.as_str()) {
            Some(name) => name.to_string(),
            None => continue,
        };

        // Get previous verification status
        let was_verified = stub_obj
            .get("verified")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        // Look up current verification status from proofs.json
        let is_verified = proofs_data
            .get(&code_name)
            .and_then(|v| v.get("verified"))
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        // Update the verified field
        stub_obj.insert("verified".to_string(), Value::Bool(is_verified));

        // Track changes
        if is_verified && !was_verified {
            newly_verified.push(stub_name.clone());
        } else if !is_verified && was_verified {
            newly_unverified.push(stub_name.clone());
        }
    }

    newly_verified.sort();
    newly_unverified.sort();

    (newly_verified, newly_unverified)
}

/// Print summary of verification changes.
fn print_verification_summary(newly_verified: &[String], newly_unverified: &[String]) {
    println!();
    println!("{}", "=".repeat(60));
    println!("VERIFICATION STATUS CHANGES");
    println!("{}", "=".repeat(60));

    if !newly_verified.is_empty() {
        println!("\nNewly verified ({}):", newly_verified.len());
        for stub_name in newly_verified {
            let display_name = get_display_name(stub_name);
            println!("  + {}", display_name);
            println!("    {}", stub_name);
        }
    } else {
        println!("\n  No newly verified items");
    }

    if !newly_unverified.is_empty() {
        println!("\nNewly unverified ({}):", newly_unverified.len());
        for stub_name in newly_unverified {
            let display_name = get_display_name(stub_name);
            println!("  - {}", display_name);
            println!("    {}", stub_name);
        }
    } else {
        println!("\n  No newly unverified items");
    }

    println!();
    println!("{}", "=".repeat(60));
    println!("  Newly verified: +{}", newly_verified.len());
    println!("  Newly unverified: -{}", newly_unverified.len());
    println!("{}", "=".repeat(60));
}

/// Load proofs from an existing proofs.json file.
fn load_proofs_from_file(proofs_path: &Path) -> Result<HashMap<String, Value>> {
    if !proofs_path.exists() {
        bail!(
            "proofs.json not found at {}. Run without --no-probe first to generate it.",
            proofs_path.display()
        );
    }

    println!("Loading proofs from {}...", proofs_path.display());
    let content = std::fs::read_to_string(proofs_path)
        .with_context(|| format!("Failed to read {}", proofs_path.display()))?;
    let proofs: HashMap<String, Value> = serde_json::from_str(&content)
        .with_context(|| format!("Failed to parse {}", proofs_path.display()))?;
    println!("Loaded {} proofs", proofs.len());
    Ok(proofs)
}

/// Run probe-verus verify and return the results.
fn run_probe_verify(
    project_root: &Path,
    proofs_path: &Path,
    atoms_path: &Path,
    package: Option<&str>,
    verify_only_module: Option<&str>,
    config: &CommandConfig,
) -> Result<HashMap<String, Value>> {
    if let Some(parent) = proofs_path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    let mut args = vec![
        "verify",
        ".",
        "-o",
        proofs_path
            .strip_prefix(project_root)
            .unwrap_or(proofs_path)
            .to_str()
            .unwrap(),
        "-a",
        atoms_path
            .strip_prefix(project_root)
            .unwrap_or(atoms_path)
            .to_str()
            .unwrap(),
    ];

    if let Some(pkg) = package {
        args.push("-p");
        args.push(pkg);
    }

    if let Some(module) = verify_only_module {
        args.push("--verify-only-module");
        args.push(module);
        println!(
            "Running probe-verus verify on {} (module: {})...",
            project_root.display(),
            module
        );
    } else {
        println!(
            "Running probe-verus verify on {}...",
            project_root.display()
        );
    }

    let output = run_command(&ExternalTool::Probe, &args, Some(project_root), config)?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    if !stdout.is_empty() {
        println!("{}", stdout);
    }
    if !stderr.is_empty() {
        eprintln!("{}", stderr);
    }

    cleanup_intermediate_files(project_root, VERIFY_INTERMEDIATE_FILES);

    // probe-verus exits non-zero when verification has failures, but still
    // produces a valid proofs.json. Only bail if it didn't write the file.
    if !proofs_path.exists() {
        bail!(
            "probe-verus verify failed (exit code: {:?}) and no results were produced",
            output.status.code()
        );
    }

    println!("Verification results saved to {}", proofs_path.display());

    let content = std::fs::read_to_string(proofs_path)?;
    let proofs: HashMap<String, Value> = serde_json::from_str(&content)?;
    Ok(proofs)
}
