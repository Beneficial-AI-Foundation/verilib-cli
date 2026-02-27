//! Atomize subcommand implementation.
//!
//! Enrich structure files with metadata from SCIP atoms.

use crate::config::ProjectConfig;
use crate::structure::{
    cleanup_intermediate_files, parse_frontmatter, run_command,
    write_frontmatter, CommandConfig, ExternalTool, ATOMIZE_INTERMEDIATE_FILES,
};
use anyhow::{bail, Context, Result};
use intervaltree::IntervalTree;
use serde_json::{json, Value};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

/// Run the atomize subcommand.
pub async fn handle_atomize(
    project_root: PathBuf,
    update_stubs: bool,
    no_probe: bool,
    check_only: bool,
    atoms_only: bool,
    rust_analyzer: bool,
) -> Result<()> {
    let project_root = project_root
        .canonicalize()
        .context("Failed to resolve project root")?;

    // Decide whether to use atoms-only mode:
    //   1. Explicit --atoms-only flag always wins
    //   2. Cargo.toml has no Verus deps -> pure Rust -> atoms-only + rust-analyzer
    //   3. Verus project with config.json -> full pipeline
    //   4. Verus project without config.json -> error (need create first)
    let is_pure_rust = !is_verus_project(&project_root);
    let use_atoms_only = if atoms_only {
        true
    } else if is_pure_rust {
        println!("No Verus dependencies detected in Cargo.toml.");
        println!("Auto-enabling atoms-only mode for pure Rust project.\n");
        true
    } else {
        ProjectConfig::init(&project_root)?;
        if ProjectConfig::global().unwrap().structure_root_path().is_err() {
            bail!(
                "Verus project detected but no .verilib/config.json found. \
                 Run 'verilib-cli create' first."
            );
        }
        false
    };

    let use_rust_analyzer = rust_analyzer || is_pure_rust;

    if use_atoms_only {
        return handle_atoms_only(&project_root, no_probe, use_rust_analyzer);
    }

    // init already called when checking structure_root above
    let config = ProjectConfig::global().unwrap();
    let structure_root = config.structure_root_path()?;
    let stubs_path = config.stubs_path();
    let atoms_path = config.atoms_path();
    let cmd_config = config.command_config();

    // Step 1: Generate stubs from .md files
    let stubs = if no_probe {
        load_stubs_from_md_files(&structure_root)?
    } else {
        generate_stubs(
            &project_root,
            &structure_root,
            &stubs_path,
            &cmd_config,
        )?
    };
    println!("Loaded {} stubs", stubs.len());

    // Step 2: Generate or load atoms.json
    let probe_atoms = if no_probe {
        load_atoms_from_file(&atoms_path)?
    } else {
        generate_probe_atoms(
            &project_root,
            &atoms_path,
            &cmd_config,
            use_rust_analyzer,
        )?
    };
    println!("Loaded {} atoms", probe_atoms.len());

    // Step 3: Build probe index for fast lookups
    let probe_index = ProbeIndex::build(&probe_atoms, project_root);

    // Step 4: Enrich stubs with code-name and all atom metadata
    println!("Enriching stubs with atom metadata...");
    let enriched = probe_index.enrich_stubs(&stubs, &probe_atoms)?;

    // If check_only, compare .md stubs against enriched and report mismatches
    if check_only {
        println!("Checking .md stub files against enriched stubs...");
        return check_stubs_match(&stubs, &enriched);
    }

    // Step 5: Save enriched stubs.json
    println!(
        "Saving enriched stubs to {}...",
        stubs_path.display()
    );
    let content = serde_json::to_string_pretty(&enriched)?;
    std::fs::write(&stubs_path, content)?;

    // Optionally update .md files with code-name
    if update_stubs {
        println!("Updating structure files with code-names...");
        update_structure_files(&enriched, &structure_root)?;
    }

    println!("Done.");
    Ok(())
}

/// Atoms-only mode: just produce atoms.json without stubs enrichment.
fn handle_atoms_only(project_root: &Path, no_probe: bool, rust_analyzer: bool) -> Result<()> {
    let verilib_path = project_root.join(".verilib");
    std::fs::create_dir_all(&verilib_path).context("Failed to create .verilib directory")?;

    let atoms_path = verilib_path.join("atoms.json");
    let config = CommandConfig::default();

    let atoms = if no_probe {
        load_atoms_from_file(&atoms_path)?
    } else {
        generate_probe_atoms(project_root, &atoms_path, &config, rust_analyzer)?
    };

    println!("Atoms-only mode: generated {} atoms.", atoms.len());
    println!("Output: {}", atoms_path.display());
    Ok(())
}

/// Check whether a parsed Cargo.toml contains Verus indicators.
///
/// Returns true if any of these are found:
/// - `[package.metadata.verus]` section
/// - `vstd`, `verus_builtin`, or `verus_builtin_macros` in `[dependencies]`,
///   `[dev-dependencies]`, or `[build-dependencies]`
/// - Same crates in `[workspace.dependencies]`
fn has_verus_indicators(parsed: &toml::Value) -> bool {
    if parsed
        .get("package")
        .and_then(|p| p.get("metadata"))
        .and_then(|m| m.get("verus"))
        .is_some()
    {
        return true;
    }

    let dep_sections = ["dependencies", "dev-dependencies", "build-dependencies"];
    let verus_crates = ["vstd", "verus_builtin", "verus_builtin_macros"];

    for section in &dep_sections {
        if let Some(deps) = parsed.get(section).and_then(|v| v.as_table()) {
            for crate_name in &verus_crates {
                if deps.contains_key(*crate_name) {
                    return true;
                }
            }
        }
    }

    if let Some(workspace) = parsed.get("workspace").and_then(|v| v.as_table()) {
        if let Some(deps) = workspace.get("dependencies").and_then(|v| v.as_table()) {
            for crate_name in &verus_crates {
                if deps.contains_key(*crate_name) {
                    return true;
                }
            }
        }
    }

    false
}

const SKIP_DIRS: &[&str] = &["target", ".git", "node_modules"];

/// Check if a project uses Verus by scanning all Cargo.toml files under the
/// project root. Skips `target/`, `.git/`, and `node_modules/` directories.
fn is_verus_project(project_root: &Path) -> bool {
    for entry in WalkDir::new(project_root).into_iter().filter_entry(|e| {
        !e.file_type().is_dir() || !SKIP_DIRS.contains(&e.file_name().to_str().unwrap_or(""))
    }) {
        let entry = match entry {
            Ok(e) => e,
            Err(_) => continue,
        };
        if entry.file_name() != "Cargo.toml" || !entry.file_type().is_file() {
            continue;
        }
        let content = match std::fs::read_to_string(entry.path()) {
            Ok(c) => c,
            Err(_) => continue,
        };
        let parsed: toml::Value = match content.parse() {
            Ok(v) => v,
            Err(_) => continue,
        };
        if has_verus_indicators(&parsed) {
            return true;
        }
    }
    false
}

/// Run probe-verus stubify to generate stubs.json from .md files.
fn generate_stubs(
    project_root: &Path,
    structure_root: &Path,
    stubs_path: &Path,
    config: &CommandConfig,
) -> Result<HashMap<String, Value>> {
    if let Some(parent) = stubs_path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    println!(
        "Running probe-verus stubify on {}...",
        structure_root.display()
    );

    let output = run_command(
        &ExternalTool::Probe,
        &[
            "stubify",
            structure_root
                .strip_prefix(project_root)
                .unwrap_or(structure_root)
                .to_str()
                .unwrap(),
            "-o",
            stubs_path
                .strip_prefix(project_root)
                .unwrap_or(stubs_path)
                .to_str()
                .unwrap(),
        ],
        Some(project_root),
        config,
    )?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        eprintln!("Error: probe-verus stubify failed.");
        if !stderr.is_empty() {
            eprintln!("{}", stderr);
        }
        cleanup_intermediate_files(project_root, ATOMIZE_INTERMEDIATE_FILES);
        bail!("probe-verus stubify failed");
    }

    println!("Stubs saved to {}", stubs_path.display());

    let content = std::fs::read_to_string(stubs_path)?;
    let stubs: HashMap<String, Value> = serde_json::from_str(&content)?;
    Ok(stubs)
}

/// Walk the structure directory and parse .md frontmatter to build stubs
/// without requiring probe-verus. This mirrors what `probe-verus stubify` does.
fn load_stubs_from_md_files(structure_root: &Path) -> Result<HashMap<String, Value>> {
    if !structure_root.exists() {
        bail!(
            "Structure directory not found at {}. Run 'verilib-cli create' first.",
            structure_root.display()
        );
    }

    println!(
        "Loading stubs from .md files in {}...",
        structure_root.display()
    );

    let mut stubs: HashMap<String, Value> = HashMap::new();
    for entry in WalkDir::new(structure_root)
        .into_iter()
        .filter_map(|e| e.ok())
    {
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("md") {
            continue;
        }
        let rel_path = path
            .strip_prefix(structure_root)
            .unwrap_or(path)
            .to_string_lossy()
            .to_string();
        match parse_frontmatter(path) {
            Ok(fm) => {
                stubs.insert(rel_path, serde_json::to_value(fm)?);
            }
            Err(e) => {
                eprintln!("Warning: skipping {}: {}", rel_path, e);
            }
        }
    }

    Ok(stubs)
}

/// Load atoms from an existing atoms.json file.
fn load_atoms_from_file(atoms_path: &Path) -> Result<HashMap<String, Value>> {
    if !atoms_path.exists() {
        bail!(
            "atoms.json not found at {}. Run without --no-probe first to generate it.",
            atoms_path.display()
        );
    }

    println!("Loading atoms from {}...", atoms_path.display());
    let content = std::fs::read_to_string(atoms_path)
        .with_context(|| format!("Failed to read {}", atoms_path.display()))?;
    let atoms: HashMap<String, Value> = serde_json::from_str(&content)
        .with_context(|| format!("Failed to parse {}", atoms_path.display()))?;
    Ok(atoms)
}

/// Run probe-verus atomize on the project and save results to atoms.json.
fn generate_probe_atoms(
    project_root: &Path,
    atoms_path: &Path,
    config: &CommandConfig,
    use_rust_analyzer: bool,
) -> Result<HashMap<String, Value>> {
    if let Some(parent) = atoms_path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    let analyzer_label = if use_rust_analyzer {
        "rust-analyzer"
    } else {
        "verus-analyzer"
    };
    println!(
        "Running probe-verus atomize ({}) on {}...",
        analyzer_label,
        project_root.display()
    );

    let atoms_path_str = atoms_path
        .strip_prefix(project_root)
        .unwrap_or(atoms_path)
        .to_str()
        .unwrap();

    let mut args = vec!["atomize", ".", "-o", atoms_path_str, "-r"];
    if use_rust_analyzer {
        args.push("--rust-analyzer");
    }

    let output = run_command(&ExternalTool::Probe, &args, Some(project_root), config)?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        eprintln!("Error: probe-verus atomize failed.");
        if !stderr.is_empty() {
            eprintln!("{}", stderr);
        }
        cleanup_intermediate_files(project_root, ATOMIZE_INTERMEDIATE_FILES);
        bail!("probe-verus atomize failed");
    }

    cleanup_intermediate_files(project_root, ATOMIZE_INTERMEDIATE_FILES);

    println!("Atoms saved to {}", atoms_path.display());

    let content = std::fs::read_to_string(atoms_path)?;
    let atoms: HashMap<String, Value> = serde_json::from_str(&content)?;
    Ok(atoms)
}

/// Interval-tree index for fast line-based atom lookups, bundled with the
/// project root used to canonicalize code-paths (resolving symlinks).
struct ProbeIndex {
    trees: HashMap<String, IntervalTree<u32, String>>,
    project_root: PathBuf,
}

impl ProbeIndex {
    /// Build the index from parsed atoms, canonicalizing every code-path
    /// relative to `project_root` so that symlinks are transparent.
    fn build(atoms: &HashMap<String, Value>, project_root: PathBuf) -> Self {
        let mut trees: HashMap<String, Vec<(std::ops::Range<u32>, String)>> = HashMap::new();

        for (probe_name, atom_data) in atoms {
            let code_path = match atom_data.get("code-path").and_then(|v| v.as_str()) {
                Some(p) => canonicalize_code_path(&project_root, p),
                None => continue,
            };

            let code_text = match atom_data.get("code-text") {
                Some(ct) => ct,
                None => continue,
            };

            let lines_start = match code_text.get("lines-start").and_then(|v| v.as_u64()) {
                Some(l) => l as u32,
                None => continue,
            };

            let lines_end = match code_text.get("lines-end").and_then(|v| v.as_u64()) {
                Some(l) => l as u32,
                None => continue,
            };

            trees
                .entry(code_path)
                .or_default()
                .push((lines_start..lines_end + 1, probe_name.clone()));
        }

        Self {
            trees: trees
                .into_iter()
                .map(|(k, v)| (k, v.into_iter().collect()))
                .collect(),
            project_root,
        }
    }

    /// Look up code-name from code-path and code-line.
    /// Canonicalizes the code-path to resolve symlinks before lookup.
    fn lookup_code_name(&self, code_path: &str, code_line: u32) -> Option<String> {
        let canonical = canonicalize_code_path(&self.project_root, code_path);
        let tree = self.trees.get(&canonical)?;

        let matching: Vec<_> = tree.query(code_line..code_line + 1).collect();

        if matching.is_empty() {
            return None;
        }

        let exact: Vec<_> = matching
            .iter()
            .filter(|iv| iv.range.start == code_line)
            .collect();

        if !exact.is_empty() {
            return Some(exact[0].value.clone());
        }

        Some(matching[0].value.clone())
    }

    /// Resolve code-name and atom for an entry.
    /// First tries existing code-name, then falls back to inference from code-path/code-line.
    fn resolve_code_name_and_atom<'a>(
        &self,
        entry: &Value,
        file_path: &str,
        atoms: &'a HashMap<String, Value>,
    ) -> Option<(String, &'a Value)> {
        if let Some(name) = entry.get("code-name").and_then(|v| v.as_str()) {
            if let Some(atom) = atoms.get(name) {
                return Some((name.to_string(), atom));
            }
        }

        let code_path = entry.get("code-path").and_then(|v| v.as_str());
        let code_line = entry
            .get("code-line")
            .and_then(|v| v.as_u64())
            .map(|l| l as u32);

        let (code_path, code_line) = match (code_path, code_line) {
            (Some(p), Some(l)) => (p, l),
            _ => {
                eprintln!("WARNING: Missing code-path or code-line for {}", file_path);
                return None;
            }
        };

        let code_name = self.lookup_code_name(code_path, code_line)?;
        let atom = atoms.get(&code_name)?;

        Some((code_name, atom))
    }

    /// Enrich stubs with code-name and all metadata from atoms.
    fn enrich_stubs(
        &self,
        stubs: &HashMap<String, Value>,
        atoms: &HashMap<String, Value>,
    ) -> Result<HashMap<String, Value>> {
        let mut result = HashMap::new();
        let mut enriched_count = 0;
        let mut skipped_count = 0;

        for (file_path, entry) in stubs {
            let (code_name, atom) =
                match self.resolve_code_name_and_atom(entry, file_path, atoms) {
                    Some(r) => r,
                    None => {
                        skipped_count += 1;
                        result.insert(file_path.clone(), entry.clone());
                        continue;
                    }
                };

            let enriched_entry = build_enriched_entry(&code_name, atom);
            result.insert(file_path.clone(), enriched_entry);
            enriched_count += 1;
        }

        println!("Entries enriched: {}", enriched_count);
        println!("Skipped: {}", skipped_count);

        Ok(result)
    }
}

/// Canonicalize a code-path relative to the project root, resolving symlinks.
/// Falls back to the original path if the file doesn't exist or canonicalization fails.
fn canonicalize_code_path(project_root: &Path, code_path: &str) -> String {
    project_root
        .join(code_path)
        .canonicalize()
        .ok()
        .and_then(|p| p.strip_prefix(project_root).ok().map(|r| r.to_path_buf()))
        .map(|p| p.to_string_lossy().to_string())
        .unwrap_or_else(|| code_path.to_string())
}

/// Build an enriched entry from atom data.
fn build_enriched_entry(code_name: &str, atom: &Value) -> Value {
    let code_path = atom.get("code-path").and_then(|v| v.as_str()).unwrap_or("");

    let code_text = atom.get("code-text");

    let lines_start = code_text
        .and_then(|ct| ct.get("lines-start"))
        .and_then(|v| v.as_u64())
        .unwrap_or(0);

    let lines_end = code_text
        .and_then(|ct| ct.get("lines-end"))
        .and_then(|v| v.as_u64())
        .unwrap_or(0);

    let code_module = atom
        .get("code-module")
        .and_then(|v| v.as_str())
        .unwrap_or("");

    let dependencies = atom
        .get("dependencies")
        .cloned()
        .unwrap_or_else(|| json!([]));

    let display_name = atom
        .get("display-name")
        .and_then(|v| v.as_str())
        .unwrap_or("");

    json!({
        "code-path": code_path,
        "code-text": {
            "lines-start": lines_start,
            "lines-end": lines_end,
        },
        "code-name": code_name,
        "code-module": code_module,
        "dependencies": dependencies,
        "display-name": display_name,
    })
}

/// Check if .md stub files match the enriched stubs.
/// Compares code-name, code-path, and code-line fields.
fn check_stubs_match(
    stubs: &HashMap<String, Value>,
    enriched: &HashMap<String, Value>,
) -> Result<()> {
    use std::collections::HashSet;

    let mut mismatches: Vec<String> = Vec::new();
    let mut mismatched_files: HashSet<String> = HashSet::new();

    for (file_path, stub_entry) in stubs {
        let enriched_entry = match enriched.get(file_path) {
            Some(e) => e,
            None => {
                mismatches.push(format!("{}: missing from enriched stubs", file_path));
                mismatched_files.insert(file_path.clone());
                continue;
            }
        };

        // Compare code-name
        let stub_code_name = stub_entry.get("code-name").and_then(|v| v.as_str());
        let enriched_code_name = enriched_entry.get("code-name").and_then(|v| v.as_str());
        if stub_code_name != enriched_code_name {
            mismatches.push(format!(
                "{}: code-name mismatch: .md has {:?}, enriched has {:?}",
                file_path, stub_code_name, enriched_code_name
            ));
            mismatched_files.insert(file_path.clone());
        }

        // Compare code-path
        let stub_code_path = stub_entry.get("code-path").and_then(|v| v.as_str());
        let enriched_code_path = enriched_entry.get("code-path").and_then(|v| v.as_str());
        if stub_code_path != enriched_code_path {
            mismatches.push(format!(
                "{}: code-path mismatch: .md has {:?}, enriched has {:?}",
                file_path, stub_code_path, enriched_code_path
            ));
            mismatched_files.insert(file_path.clone());
        }

        // Compare code-line (from stub) vs lines-start (from enriched code-text)
        let stub_code_line = stub_entry.get("code-line").and_then(|v| v.as_u64());
        let enriched_code_line = enriched_entry
            .get("code-text")
            .and_then(|ct| ct.get("lines-start"))
            .and_then(|v| v.as_u64());
        if stub_code_line != enriched_code_line {
            mismatches.push(format!(
                "{}: code-line mismatch: .md has {:?}, enriched has {:?}",
                file_path, stub_code_line, enriched_code_line
            ));
            mismatched_files.insert(file_path.clone());
        }
    }

    if mismatches.is_empty() {
        println!("All {} stub files match enriched stubs.", stubs.len());
        Ok(())
    } else {
        eprintln!(
            "Found {} mismatches in {} stub files:",
            mismatches.len(),
            mismatched_files.len()
        );
        for mismatch in &mismatches {
            eprintln!("  {}", mismatch);
        }
        eprintln!("\nStub files needing update:");
        let mut files: Vec<_> = mismatched_files.iter().collect();
        files.sort();
        for file in files {
            eprintln!("  {}", file);
        }
        bail!(
            "{} stub files do not match enriched stubs. Run 'atomize --update-stubs' to update them.",
            mismatched_files.len()
        );
    }
}

/// Update structure .md files with code-name field from enriched data.
fn update_structure_files(enriched: &HashMap<String, Value>, structure_root: &Path) -> Result<()> {
    let mut updated_count = 0;
    let mut skipped_count = 0;

    for (file_path, entry) in enriched {
        let path = structure_root.join(file_path);
        if !path.exists() {
            skipped_count += 1;
            continue;
        }

        let code_name = match entry.get("code-name").and_then(|v| v.as_str()) {
            Some(name) => name,
            None => {
                skipped_count += 1;
                continue;
            }
        };

        let fm = match parse_frontmatter(&path) {
            Ok(fm) => fm,
            Err(_) => {
                skipped_count += 1;
                continue;
            }
        };

        // Read original file content to preserve body
        let original_content = std::fs::read_to_string(&path)?;
        let body_start = original_content
            .find("\n---\n")
            .map(|pos| pos + 5)
            .and_then(|start| {
                original_content[start..]
                    .find("\n---\n")
                    .map(|p| start + p + 5)
            });

        let body = body_start.map(|start| original_content[start..].to_string());

        // Build updated frontmatter
        let mut metadata: HashMap<String, Value> =
            fm.iter().map(|(k, v)| (k.clone(), v.clone())).collect();
        metadata.insert("code-name".to_string(), json!(code_name));

        // Update code-path and code-line to be consistent with enriched data
        if let Some(code_path) = entry.get("code-path").and_then(|v| v.as_str()) {
            metadata.insert("code-path".to_string(), json!(code_path));
        }
        if let Some(code_line) = entry
            .get("code-text")
            .and_then(|ct| ct.get("lines-start"))
            .and_then(|v| v.as_u64())
        {
            metadata.insert("code-line".to_string(), json!(code_line));
        }

        write_frontmatter(&path, &metadata, body.as_deref())?;
        updated_count += 1;
    }

    println!("Structure files updated: {}", updated_count);
    println!("Skipped: {}", skipped_count);

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[cfg(unix)]
    #[test]
    fn test_canonicalize_code_path_resolves_symlinks() {
        let dir = TempDir::new().unwrap();
        let project_root = dir.path().canonicalize().unwrap();

        let real_dir = project_root.join("deps").join("my-crate").join("my-crate");
        std::fs::create_dir_all(real_dir.join("src")).unwrap();
        std::fs::write(real_dir.join("src").join("lib.rs"), "fn main() {}").unwrap();

        std::os::unix::fs::symlink(real_dir.to_str().unwrap(), project_root.join("my-crate"))
            .unwrap();

        let via_symlink = canonicalize_code_path(&project_root, "my-crate/src/lib.rs");
        let via_real = canonicalize_code_path(&project_root, "deps/my-crate/my-crate/src/lib.rs");

        assert_eq!(via_symlink, via_real);
        assert_eq!(via_real, "deps/my-crate/my-crate/src/lib.rs");
    }

    #[test]
    fn test_canonicalize_code_path_nonexistent_falls_back() {
        let dir = TempDir::new().unwrap();
        let project_root = dir.path().canonicalize().unwrap();

        let result = canonicalize_code_path(&project_root, "nonexistent/src/lib.rs");
        assert_eq!(result, "nonexistent/src/lib.rs");
    }

    #[cfg(unix)]
    #[test]
    fn test_enrich_stubs_matches_through_symlinks() {
        let dir = TempDir::new().unwrap();
        let project_root = dir.path().canonicalize().unwrap();

        let real_dir = project_root.join("deps").join("my-crate").join("my-crate");
        std::fs::create_dir_all(real_dir.join("src")).unwrap();
        std::fs::write(real_dir.join("src").join("lib.rs"), "fn main() {}").unwrap();

        std::os::unix::fs::symlink(real_dir.to_str().unwrap(), project_root.join("my-crate"))
            .unwrap();

        let mut atoms = HashMap::new();
        atoms.insert(
            "probe:my-crate/0.1.0/func_a()".to_string(),
            json!({
                "code-path": "my-crate/src/lib.rs",
                "code-text": { "lines-start": 10, "lines-end": 20 },
                "code-module": "my_crate",
                "dependencies": [],
                "display-name": "func_a",
            }),
        );

        let mut stubs = HashMap::new();
        stubs.insert(
            "deps/my-crate/my-crate/src/lib.rs/func_a.md".to_string(),
            json!({
                "code-path": "deps/my-crate/my-crate/src/lib.rs",
                "code-line": 10,
            }),
        );

        let index = ProbeIndex::build(&atoms, project_root);
        let enriched = index.enrich_stubs(&stubs, &atoms).unwrap();

        let entry = &enriched["deps/my-crate/my-crate/src/lib.rs/func_a.md"];
        assert_eq!(
            entry.get("code-name").and_then(|v| v.as_str()).unwrap(),
            "probe:my-crate/0.1.0/func_a()"
        );
    }

    #[test]
    fn test_enrich_stubs_direct_path_match() {
        let dir = TempDir::new().unwrap();
        let project_root = dir.path().canonicalize().unwrap();

        std::fs::create_dir_all(project_root.join("src")).unwrap();
        std::fs::write(project_root.join("src").join("lib.rs"), "").unwrap();

        let mut atoms = HashMap::new();
        atoms.insert(
            "probe:test/0.1.0/func_a()".to_string(),
            json!({
                "code-path": "src/lib.rs",
                "code-text": { "lines-start": 5, "lines-end": 15 },
                "code-module": "test",
                "dependencies": [],
                "display-name": "func_a",
            }),
        );

        let mut stubs = HashMap::new();
        stubs.insert(
            "src/lib.rs/func_a.md".to_string(),
            json!({
                "code-path": "src/lib.rs",
                "code-line": 5,
            }),
        );

        let index = ProbeIndex::build(&atoms, project_root);
        let enriched = index.enrich_stubs(&stubs, &atoms).unwrap();

        let entry = &enriched["src/lib.rs/func_a.md"];
        assert_eq!(
            entry.get("code-name").and_then(|v| v.as_str()).unwrap(),
            "probe:test/0.1.0/func_a()"
        );
    }

    #[test]
    fn test_is_verus_project_with_vstd_dep() {
        let dir = TempDir::new().unwrap();
        std::fs::write(
            dir.path().join("Cargo.toml"),
            r#"[package]
name = "test"
version = "0.1.0"

[dependencies]
vstd = { git = "https://github.com/verus-lang/verus" }
"#,
        )
        .unwrap();
        assert!(is_verus_project(dir.path()));
    }

    #[test]
    fn test_is_verus_project_with_builtin_dep() {
        let dir = TempDir::new().unwrap();
        std::fs::write(
            dir.path().join("Cargo.toml"),
            r#"[package]
name = "test"
version = "0.1.0"

[dependencies]
verus_builtin = { git = "https://github.com/verus-lang/verus" }
"#,
        )
        .unwrap();
        assert!(is_verus_project(dir.path()));
    }

    #[test]
    fn test_is_verus_project_with_metadata_section() {
        let dir = TempDir::new().unwrap();
        std::fs::write(
            dir.path().join("Cargo.toml"),
            r#"[package]
name = "test"
version = "0.1.0"

[package.metadata.verus]
verify = true
"#,
        )
        .unwrap();
        assert!(is_verus_project(dir.path()));
    }

    #[test]
    fn test_is_verus_project_with_workspace_deps() {
        let dir = TempDir::new().unwrap();
        std::fs::write(
            dir.path().join("Cargo.toml"),
            r#"[workspace]
members = ["crate-a"]

[workspace.dependencies]
vstd = { git = "https://github.com/verus-lang/verus" }
"#,
        )
        .unwrap();
        assert!(is_verus_project(dir.path()));
    }

    #[test]
    fn test_is_not_verus_project_plain_rust() {
        let dir = TempDir::new().unwrap();
        std::fs::write(
            dir.path().join("Cargo.toml"),
            r#"[package]
name = "test"
version = "0.1.0"

[dependencies]
serde = "1.0"
tokio = "1"
"#,
        )
        .unwrap();
        assert!(!is_verus_project(dir.path()));
    }

    #[test]
    fn test_is_not_verus_project_no_cargo_toml() {
        let dir = TempDir::new().unwrap();
        assert!(!is_verus_project(dir.path()));
    }

    #[test]
    fn test_is_verus_project_nested_cargo_toml() {
        let dir = TempDir::new().unwrap();
        std::fs::write(
            dir.path().join("Cargo.toml"),
            r#"[workspace]
members = ["sub-crate"]
"#,
        )
        .unwrap();
        let sub = dir.path().join("sub-crate");
        std::fs::create_dir_all(&sub).unwrap();
        std::fs::write(
            sub.join("Cargo.toml"),
            r#"[package]
name = "sub-crate"
version = "0.1.0"

[dependencies]
vstd = { git = "https://github.com/verus-lang/verus" }
"#,
        )
        .unwrap();
        assert!(is_verus_project(dir.path()));
    }

    #[test]
    fn test_is_not_verus_project_nested_plain_rust() {
        let dir = TempDir::new().unwrap();
        std::fs::write(
            dir.path().join("Cargo.toml"),
            r#"[workspace]
members = ["sub-crate"]
"#,
        )
        .unwrap();
        let sub = dir.path().join("sub-crate");
        std::fs::create_dir_all(&sub).unwrap();
        std::fs::write(
            sub.join("Cargo.toml"),
            r#"[package]
name = "sub-crate"
version = "0.1.0"

[dependencies]
serde = "1.0"
"#,
        )
        .unwrap();
        assert!(!is_verus_project(dir.path()));
    }

    #[test]
    fn test_is_verus_project_skips_target_dir() {
        let dir = TempDir::new().unwrap();
        std::fs::write(
            dir.path().join("Cargo.toml"),
            r#"[package]
name = "test"
version = "0.1.0"

[dependencies]
serde = "1.0"
"#,
        )
        .unwrap();
        let target_sub = dir.path().join("target").join("debug").join("build");
        std::fs::create_dir_all(&target_sub).unwrap();
        std::fs::write(
            target_sub.join("Cargo.toml"),
            r#"[package]
name = "hidden"
version = "0.1.0"

[dependencies]
vstd = { git = "https://github.com/verus-lang/verus" }
"#,
        )
        .unwrap();
        assert!(!is_verus_project(dir.path()));
    }
}
