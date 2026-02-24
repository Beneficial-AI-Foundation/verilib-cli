//! Regression tests for the verilib-cli pipeline.
//!
//! Tests verify observable behavior (exit codes, artifact contents, file existence)
//! rather than implementation details (stdout/stderr message formats).

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use tempfile::TempDir;

// ---------------------------------------------------------------------------
// Shared helpers
// ---------------------------------------------------------------------------

fn fixtures_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures")
}

fn copy_dir_recursive(src: &Path, dst: &Path) -> std::io::Result<()> {
    fs::create_dir_all(dst)?;
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let src_path = entry.path();
        let dst_path = dst.join(entry.file_name());
        if src_path.is_dir() {
            copy_dir_recursive(&src_path, &dst_path)?;
        } else {
            fs::copy(&src_path, &dst_path)?;
        }
    }
    Ok(())
}

/// Set up a temp project with Verus-style Cargo.toml and all fixtures in .verilib/.
fn setup_test_project() -> TempDir {
    setup_test_project_with_config("config.json")
}

fn setup_test_project_with_config(config_name: &str) -> TempDir {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let verilib_dir = temp_dir.path().join(".verilib");
    fs::create_dir_all(&verilib_dir).expect("Failed to create .verilib dir");

    fs::write(
        temp_dir.path().join("Cargo.toml"),
        r#"[package]
name = "test-verus-project"
version = "0.1.0"
edition = "2021"

[dependencies]
vstd = { git = "https://github.com/verus-lang/verus", rev = "test" }
"#,
    )
    .expect("Failed to write Cargo.toml");

    let fix = fixtures_dir();

    fs::copy(fix.join(config_name), verilib_dir.join("config.json"))
        .expect("Failed to copy config");

    for file in ["atoms.json", "specs.json", "proofs.json", "stubs.json"] {
        fs::copy(fix.join(file), verilib_dir.join(file))
            .unwrap_or_else(|_| panic!("Failed to copy {}", file));
    }

    copy_dir_recursive(&fix.join("structure"), &verilib_dir.join("structure"))
        .expect("Failed to copy structure dir");
    copy_dir_recursive(&fix.join("certs"), &verilib_dir.join("certs"))
        .expect("Failed to copy certs dir");

    temp_dir
}

fn run_cmd(args: &[&str], cwd: &Path) -> Output {
    Command::new(env!("CARGO_BIN_EXE_verilib-cli"))
        .args(args)
        .current_dir(cwd)
        .output()
        .expect("Failed to execute command")
}

/// Run verilib-cli with the mock probe-verus binary on PATH.
fn run_with_mock(args: &[&str], cwd: &Path, mock_bin_dir: &Path) -> Output {
    let original_path = std::env::var("PATH").unwrap_or_default();
    let new_path = format!("{}:{}", mock_bin_dir.display(), original_path);
    Command::new(env!("CARGO_BIN_EXE_verilib-cli"))
        .args(args)
        .current_dir(cwd)
        .env("PATH", new_path)
        .env("MOCK_FIXTURES_DIR", fixtures_dir())
        .output()
        .expect("Failed to execute command")
}

/// Create a temp directory with a symlink probe-verus -> mock-probe-verus binary.
fn setup_mock_probe_dir() -> TempDir {
    let mock_dir = TempDir::new().expect("Failed to create mock dir");
    let mock_binary = PathBuf::from(env!("CARGO_BIN_EXE_mock-probe-verus"));
    #[cfg(unix)]
    std::os::unix::fs::symlink(&mock_binary, mock_dir.path().join("probe-verus"))
        .expect("Failed to symlink mock probe-verus");
    mock_dir
}

fn read_stubs_json(project: &Path) -> HashMap<String, serde_json::Value> {
    let path = project.join(".verilib/stubs.json");
    let content = fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("Failed to read {}: {}", path.display(), e));
    serde_json::from_str(&content)
        .unwrap_or_else(|e| panic!("Failed to parse {}: {}", path.display(), e))
}

/// Collect SHA-256 checksums of all .md files under a directory.
fn collect_md_checksums(dir: &Path) -> HashMap<PathBuf, Vec<u8>> {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    let mut result = HashMap::new();
    if !dir.exists() {
        return result;
    }
    for entry in walkdir(dir) {
        if entry.extension().is_some_and(|e| e == "md") {
            let content = fs::read(&entry).unwrap();
            let mut hasher = DefaultHasher::new();
            content.hash(&mut hasher);
            result.insert(entry, hasher.finish().to_le_bytes().to_vec());
        }
    }
    result
}

fn walkdir(dir: &Path) -> Vec<PathBuf> {
    let mut files = Vec::new();
    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                files.extend(walkdir(&path));
            } else {
                files.push(path);
            }
        }
    }
    files
}

fn assert_success(output: &Output, context: &str) {
    assert!(
        output.status.success(),
        "{} failed (exit {:?}).\nstderr: {}",
        context,
        output.status.code(),
        String::from_utf8_lossy(&output.stderr)
    );
}

fn assert_failure(output: &Output, context: &str) {
    assert!(
        !output.status.success(),
        "{} should have failed but succeeded.\nstdout: {}",
        context,
        String::from_utf8_lossy(&output.stdout)
    );
}

// ===========================================================================
// Category 1: Full Pipeline (mock probe-verus)
// ===========================================================================

mod pipeline_tests {
    use super::*;

    #[test]
    fn test_full_pipeline_create_atomize_specify_verify() {
        let mock_dir = setup_mock_probe_dir();
        let temp_dir = TempDir::new().expect("Failed to create temp dir");

        // Need a Cargo.toml so is_verus_project() works for atomize
        fs::write(
            temp_dir.path().join("Cargo.toml"),
            r#"[package]
name = "test-verus-project"
version = "0.1.0"
edition = "2021"

[dependencies]
vstd = { git = "https://github.com/verus-lang/verus", rev = "test" }
"#,
        )
        .unwrap();

        // --- Step 1: create ---
        let output = run_with_mock(&["create"], temp_dir.path(), mock_dir.path());
        assert_success(&output, "create");

        let config_path = temp_dir.path().join(".verilib/config.json");
        assert!(
            config_path.exists(),
            "config.json should exist after create"
        );
        let config: serde_json::Value =
            serde_json::from_str(&fs::read_to_string(&config_path).unwrap()).unwrap();
        assert!(
            config.get("structure-root").is_some(),
            "config.json must have structure-root"
        );

        // .md structure files should have been created
        let structure_root_str = config["structure-root"].as_str().unwrap();
        let structure_root = temp_dir.path().join(structure_root_str);
        let md_files: Vec<_> = walkdir(&structure_root)
            .into_iter()
            .filter(|p| p.extension().is_some_and(|e| e == "md"))
            .collect();
        assert!(
            !md_files.is_empty(),
            "create should generate .md structure files"
        );

        // --- Enable auto-validate for specify step ---
        let config_content = fs::read_to_string(&config_path).unwrap();
        let mut config_json: serde_json::Value = serde_json::from_str(&config_content).unwrap();
        config_json["auto-validate-specs"] = serde_json::Value::Bool(true);
        fs::write(
            &config_path,
            serde_json::to_string_pretty(&config_json).unwrap(),
        )
        .unwrap();

        // --- Step 2: atomize --update-stubs ---
        let output = run_with_mock(
            &["atomize", "--update-stubs"],
            temp_dir.path(),
            mock_dir.path(),
        );
        assert_success(&output, "atomize --update-stubs");

        let stubs_path = temp_dir.path().join(".verilib/stubs.json");
        assert!(stubs_path.exists(), "stubs.json should exist after atomize");
        let stubs: HashMap<String, serde_json::Value> =
            serde_json::from_str(&fs::read_to_string(&stubs_path).unwrap()).unwrap();
        assert!(!stubs.is_empty(), "stubs.json should not be empty");

        for (key, stub) in &stubs {
            assert!(
                stub.get("code-name").is_some(),
                "stub '{}' should have code-name after atomize",
                key
            );
        }

        // --- Step 3: specify ---
        let output = run_with_mock(&["specify"], temp_dir.path(), mock_dir.path());
        assert_success(&output, "specify");

        let certs_dir = temp_dir.path().join(".verilib/certs/specs");
        if certs_dir.exists() {
            let cert_files: Vec<_> = fs::read_dir(&certs_dir)
                .unwrap()
                .flatten()
                .filter(|e| e.path().extension().is_some_and(|ext| ext == "json"))
                .collect();
            assert!(
                !cert_files.is_empty(),
                "specify should create at least one cert"
            );
        }

        // --- Step 4: verify ---
        let output = run_with_mock(&["verify"], temp_dir.path(), mock_dir.path());
        assert_success(&output, "verify");

        let final_stubs: HashMap<String, serde_json::Value> =
            serde_json::from_str(&fs::read_to_string(&stubs_path).unwrap()).unwrap();
        for (key, stub) in &final_stubs {
            if stub.get("code-name").and_then(|v| v.as_str()).is_some() {
                assert!(
                    stub.get("verified").is_some(),
                    "stub '{}' should have 'verified' field after verify",
                    key
                );
            }
        }
    }
}

// ===========================================================================
// Category 2: Pipeline Data Flow (--no-probe)
// ===========================================================================

mod data_flow_tests {
    use super::*;

    #[test]
    fn test_atomize_output_feeds_into_specify() {
        let temp_dir = setup_test_project();

        let output = run_cmd(&["atomize", "--no-probe"], temp_dir.path());
        assert_success(&output, "atomize --no-probe");

        // specify should be able to consume the stubs.json that atomize produced
        let output = run_cmd(&["specify", "--no-probe", "--check-only"], temp_dir.path());
        // May fail (uncertified stubs) but should NOT crash/panic
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(
            !stderr.contains("panic") && !stderr.contains("Failed to parse"),
            "specify should be able to read atomize output without errors"
        );

        // stubs.json should remain valid JSON after the pipeline
        let _stubs = read_stubs_json(temp_dir.path());
    }

    /// Run specify with auto-validate to verify spec-text is written to stubs.json.
    #[test]
    fn test_atomize_then_specify_writes_spec_text() {
        let temp_dir = setup_test_project_with_config("config_auto_validate.json");

        let output = run_cmd(&["atomize", "--no-probe"], temp_dir.path());
        assert_success(&output, "atomize --no-probe");

        let output = run_cmd(&["specify", "--no-probe"], temp_dir.path());
        assert_success(&output, "specify --no-probe (auto-validate)");

        let stubs = read_stubs_json(temp_dir.path());
        let with_spec_text: Vec<_> = stubs
            .values()
            .filter(|s| s.get("spec-text").is_some())
            .collect();
        assert!(
            with_spec_text.len() >= 2,
            "at least 2 stubs should have spec-text after specify (got {})",
            with_spec_text.len()
        );
    }

    #[test]
    fn test_atomize_then_verify_data_flow() {
        let temp_dir = setup_test_project();

        let output = run_cmd(&["atomize", "--no-probe"], temp_dir.path());
        assert_success(&output, "atomize --no-probe");

        let output = run_cmd(&["verify", "--no-probe"], temp_dir.path());
        assert_success(&output, "verify --no-probe");

        let stubs = read_stubs_json(temp_dir.path());
        for (key, stub) in &stubs {
            if stub.get("code-name").and_then(|v| v.as_str()).is_some() {
                assert!(
                    stub.get("verified").is_some(),
                    "stub '{}' should have 'verified' after verify",
                    key
                );
            }
        }
    }
}

// ===========================================================================
// Category 3: Quantitative Regression (artifact-based)
// ===========================================================================

mod quantitative_tests {
    use super::*;

    #[test]
    fn test_atomize_enriches_all_fixture_stubs() {
        let temp_dir = setup_test_project();

        let output = run_cmd(&["atomize", "--no-probe"], temp_dir.path());
        assert_success(&output, "atomize --no-probe");

        let stubs = read_stubs_json(temp_dir.path());
        assert_eq!(stubs.len(), 3, "fixture has 3 stubs");

        for (key, stub) in &stubs {
            let code_name = stub.get("code-name").and_then(|v| v.as_str());
            assert!(
                code_name.is_some() && !code_name.unwrap().is_empty(),
                "stub '{}' should have a non-empty code-name (enrichment should not skip it)",
                key
            );
        }
    }

    #[test]
    fn test_specify_adds_spec_text_to_specified_stubs() {
        let temp_dir = setup_test_project_with_config("config_auto_validate.json");

        run_cmd(&["atomize", "--no-probe"], temp_dir.path());

        // auto-validate writes stubs.json with spec-text incorporated
        let output = run_cmd(&["specify", "--no-probe"], temp_dir.path());
        assert_success(&output, "specify --no-probe (auto-validate)");

        let stubs = read_stubs_json(temp_dir.path());

        // func_a and func_b are specified=true in specs.json
        let func_a = &stubs["src/module.rs/func_a().md"];
        assert!(
            func_a.get("spec-text").is_some(),
            "func_a should have spec-text (it is specified)"
        );

        let func_b = &stubs["src/module.rs/func_b().md"];
        assert!(
            func_b.get("spec-text").is_some(),
            "func_b should have spec-text (it is specified)"
        );

        // func_c is specified=false
        let func_c = &stubs["src/other.rs/func_c().md"];
        assert!(
            func_c.get("spec-text").is_none(),
            "func_c should NOT have spec-text (it is not specified)"
        );
    }

    #[test]
    fn test_verify_sets_verified_field_from_proofs() {
        let temp_dir = setup_test_project();

        run_cmd(&["atomize", "--no-probe"], temp_dir.path());
        let output = run_cmd(&["verify", "--no-probe"], temp_dir.path());
        assert_success(&output, "verify --no-probe");

        let stubs = read_stubs_json(temp_dir.path());

        // proofs.json has func_a verified=true, func_b verified=false
        let func_a = &stubs["src/module.rs/func_a().md"];
        assert_eq!(
            func_a["verified"].as_bool(),
            Some(true),
            "func_a should be verified"
        );

        let func_b = &stubs["src/module.rs/func_b().md"];
        assert_eq!(
            func_b["verified"].as_bool(),
            Some(false),
            "func_b should not be verified"
        );
    }

    /// verify --check-only on fixture stubs that already have status="failure"
    /// should detect the failure and return non-zero exit code.
    #[test]
    fn test_verify_check_only_detects_existing_failures() {
        let temp_dir = setup_test_project();
        // fixture stubs.json has func_b with status: "failure" already

        let output = run_cmd(&["verify", "--check-only"], temp_dir.path());
        assert_failure(&output, "verify --check-only with status=failure in stubs");
    }
}

// ===========================================================================
// Category 4: Requirement-Derived Tests (from PDF)
// ===========================================================================

mod requirement_tests {
    use super::*;

    /// PDF p.8: "code-name will always take precedence" over code-path/code-line.
    #[test]
    fn test_code_name_takes_precedence_over_code_line() {
        let temp_dir = setup_test_project();
        let verilib_dir = temp_dir.path().join(".verilib");

        // Give func_a's .md a code-name that points to func_a's atom,
        // but a code-line that points into func_b's range (line 25).
        // The code-name should win.
        let md_path = verilib_dir.join("structure/src/module.rs/func_a().md");
        fs::write(
            &md_path,
            r#"---
code-name: "probe:test/1.0.0/module/func_a()"
code-path: "src/module.rs"
code-line: 25
---
"#,
        )
        .unwrap();

        let output = run_cmd(&["atomize", "--no-probe"], temp_dir.path());
        assert_success(&output, "atomize --no-probe");

        let stubs = read_stubs_json(temp_dir.path());
        let func_a = &stubs["src/module.rs/func_a().md"];

        assert_eq!(
            func_a["code-name"].as_str(),
            Some("probe:test/1.0.0/module/func_a()"),
            "code-name should be func_a (not func_b despite code-line=25)"
        );
        assert_eq!(
            func_a["display-name"].as_str(),
            Some("func_a"),
            "display-name should come from func_a's atom"
        );
    }

    /// PDF p.8: ".md files will not be overwritten during specification/verification"
    #[test]
    fn test_md_files_unchanged_during_specify() {
        let temp_dir = setup_test_project();
        let structure_dir = temp_dir.path().join(".verilib/structure");

        let before = collect_md_checksums(&structure_dir);
        assert!(!before.is_empty(), "should have .md files to check");

        run_cmd(&["specify", "--no-probe", "--check-only"], temp_dir.path());

        let after = collect_md_checksums(&structure_dir);
        assert_eq!(before, after, ".md files should not change during specify");
    }

    /// PDF p.8: same requirement for verify
    #[test]
    fn test_md_files_unchanged_during_verify() {
        let temp_dir = setup_test_project();
        let structure_dir = temp_dir.path().join(".verilib/structure");

        let before = collect_md_checksums(&structure_dir);
        assert!(!before.is_empty(), "should have .md files to check");

        run_cmd(&["verify", "--no-probe"], temp_dir.path());

        let after = collect_md_checksums(&structure_dir);
        assert_eq!(before, after, ".md files should not change during verify");
    }

    /// PDF p.9: "For the MVP, the spec cert will just contain a timestamp"
    #[test]
    fn test_cert_contains_timestamp() {
        let temp_dir = setup_test_project_with_config("config_auto_validate.json");

        // atomize first to enrich stubs, then specify with auto-validate
        run_cmd(&["atomize", "--no-probe"], temp_dir.path());
        let output = run_cmd(&["specify", "--no-probe"], temp_dir.path());
        assert_success(&output, "specify --no-probe (auto-validate)");

        let certs_dir = temp_dir.path().join(".verilib/certs/specs");
        assert!(certs_dir.exists(), "certs/specs/ should exist");

        let cert_files: Vec<_> = fs::read_dir(&certs_dir)
            .unwrap()
            .flatten()
            .filter(|e| e.path().extension().is_some_and(|ext| ext == "json"))
            .collect();
        assert!(
            !cert_files.is_empty(),
            "auto-validate should create cert files"
        );

        for entry in &cert_files {
            let content = fs::read_to_string(entry.path()).unwrap();
            let cert: serde_json::Value = serde_json::from_str(&content).unwrap_or_else(|e| {
                panic!("cert {} is not valid JSON: {}", entry.path().display(), e)
            });
            assert!(
                cert.get("timestamp").is_some(),
                "cert {} must contain a 'timestamp' field",
                entry.path().display()
            );
            let ts = cert["timestamp"].as_str().unwrap();
            assert!(
                ts.contains('T') && ts.contains(':'),
                "timestamp '{}' should look like ISO 8601",
                ts
            );
        }
    }

    /// PDF p.7: "dependencies can only be code dependencies (comes from atomization)"
    #[test]
    fn test_dependencies_populated_from_atoms() {
        let temp_dir = setup_test_project();

        let output = run_cmd(&["atomize", "--no-probe"], temp_dir.path());
        assert_success(&output, "atomize --no-probe");

        let stubs = read_stubs_json(temp_dir.path());

        // func_a depends on helper
        let func_a_deps = stubs["src/module.rs/func_a().md"]["dependencies"]
            .as_array()
            .expect("func_a dependencies should be an array");
        let func_a_dep_strs: Vec<&str> = func_a_deps.iter().filter_map(|v| v.as_str()).collect();
        assert!(
            func_a_dep_strs.contains(&"probe:test/1.0.0/module/helper()"),
            "func_a should depend on helper, got {:?}",
            func_a_dep_strs
        );

        // func_b has no dependencies
        let func_b_deps = stubs["src/module.rs/func_b().md"]["dependencies"]
            .as_array()
            .expect("func_b dependencies should be an array");
        assert!(
            func_b_deps.is_empty(),
            "func_b should have empty dependencies"
        );

        // func_c depends on func_a
        let func_c_deps = stubs["src/other.rs/func_c().md"]["dependencies"]
            .as_array()
            .expect("func_c dependencies should be an array");
        let func_c_dep_strs: Vec<&str> = func_c_deps.iter().filter_map(|v| v.as_str()).collect();
        assert!(
            func_c_dep_strs.contains(&"probe:test/1.0.0/module/func_a()"),
            "func_c should depend on func_a, got {:?}",
            func_c_dep_strs
        );
    }

    /// PDF p.14: atoms have code-module, code-path, code-text, dependencies
    #[test]
    fn test_enriched_stubs_have_required_fields() {
        let temp_dir = setup_test_project();

        let output = run_cmd(&["atomize", "--no-probe"], temp_dir.path());
        assert_success(&output, "atomize --no-probe");

        let stubs = read_stubs_json(temp_dir.path());
        let required_fields = [
            "code-name",
            "code-path",
            "code-text",
            "code-module",
            "dependencies",
            "display-name",
        ];

        for (key, stub) in &stubs {
            for field in &required_fields {
                assert!(
                    stub.get(*field).is_some(),
                    "stub '{}' is missing required field '{}'",
                    key,
                    field
                );
            }
        }
    }
}

// ===========================================================================
// Category 5: Idempotency
// ===========================================================================

mod idempotency_tests {
    use super::*;

    #[test]
    fn test_atomize_idempotent() {
        let temp_dir = setup_test_project();

        let output = run_cmd(&["atomize", "--no-probe"], temp_dir.path());
        assert_success(&output, "atomize --no-probe (first run)");
        let stubs_first = fs::read_to_string(temp_dir.path().join(".verilib/stubs.json")).unwrap();

        let output = run_cmd(&["atomize", "--no-probe"], temp_dir.path());
        assert_success(&output, "atomize --no-probe (second run)");
        let stubs_second = fs::read_to_string(temp_dir.path().join(".verilib/stubs.json")).unwrap();

        let first: serde_json::Value = serde_json::from_str(&stubs_first).unwrap();
        let second: serde_json::Value = serde_json::from_str(&stubs_second).unwrap();
        assert_eq!(
            first, second,
            "running atomize twice should produce identical stubs.json"
        );
    }

    #[test]
    fn test_verify_idempotent() {
        let temp_dir = setup_test_project();

        run_cmd(&["atomize", "--no-probe"], temp_dir.path());

        let output = run_cmd(&["verify", "--no-probe"], temp_dir.path());
        assert_success(&output, "verify --no-probe (first run)");
        let stubs_first = fs::read_to_string(temp_dir.path().join(".verilib/stubs.json")).unwrap();

        let output = run_cmd(&["verify", "--no-probe"], temp_dir.path());
        assert_success(&output, "verify --no-probe (second run)");
        let stubs_second = fs::read_to_string(temp_dir.path().join(".verilib/stubs.json")).unwrap();

        let first: serde_json::Value = serde_json::from_str(&stubs_first).unwrap();
        let second: serde_json::Value = serde_json::from_str(&stubs_second).unwrap();
        assert_eq!(
            first, second,
            "running verify twice should produce identical stubs.json"
        );
    }
}
