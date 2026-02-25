//! Regression tests for the verilib-cli pipeline.
//!
//! Every test verifies observable behavior: exit codes, artifact contents,
//! and file existence/integrity. No test asserts on stdout/stderr message text.

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use tempfile::TempDir;

// ---------------------------------------------------------------------------
// Helpers
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

/// Create a temp project with Verus-style Cargo.toml and all fixtures in `.verilib/`.
fn setup_project() -> TempDir {
    setup_project_with_config("config.json")
}

fn setup_project_with_config(config_name: &str) -> TempDir {
    let tmp = TempDir::new().expect("Failed to create temp dir");
    let verilib = tmp.path().join(".verilib");
    fs::create_dir_all(&verilib).expect("Failed to create .verilib dir");

    fs::write(
        tmp.path().join("Cargo.toml"),
        "[package]\nname = \"test-verus-project\"\nversion = \"0.1.0\"\nedition = \"2021\"\n\n\
         [dependencies]\nvstd = { git = \"https://github.com/verus-lang/verus\", rev = \"test\" }\n",
    )
    .expect("Failed to write Cargo.toml");

    let fix = fixtures_dir();
    fs::copy(fix.join(config_name), verilib.join("config.json")).expect("Failed to copy config");

    for file in ["atoms.json", "specs.json", "proofs.json", "stubs.json"] {
        fs::copy(fix.join(file), verilib.join(file))
            .unwrap_or_else(|_| panic!("Failed to copy {}", file));
    }

    copy_dir_recursive(&fix.join("structure"), &verilib.join("structure"))
        .expect("Failed to copy structure dir");
    copy_dir_recursive(&fix.join("certs"), &verilib.join("certs"))
        .expect("Failed to copy certs dir");

    tmp
}

fn cli(args: &[&str], cwd: &Path) -> Output {
    Command::new(env!("CARGO_BIN_EXE_verilib-cli"))
        .args(args)
        .current_dir(cwd)
        .output()
        .expect("Failed to execute verilib-cli")
}

fn read_json(path: &Path) -> serde_json::Value {
    let content = fs::read_to_string(path)
        .unwrap_or_else(|e| panic!("Failed to read {}: {}", path.display(), e));
    serde_json::from_str(&content)
        .unwrap_or_else(|e| panic!("Failed to parse {}: {}", path.display(), e))
}

fn read_stubs(project: &Path) -> HashMap<String, serde_json::Value> {
    let v = read_json(&project.join(".verilib/stubs.json"));
    serde_json::from_value(v).expect("stubs.json is not an object")
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

fn collect_md_checksums(dir: &Path) -> HashMap<PathBuf, Vec<u8>> {
    use sha2::{Digest, Sha256};
    let mut result = HashMap::new();
    if !dir.exists() {
        return result;
    }
    for entry in walk(dir) {
        if entry.extension().is_some_and(|e| e == "md") {
            let content = fs::read(&entry).unwrap();
            result.insert(entry, Sha256::digest(&content).to_vec());
        }
    }
    result
}

fn walk(dir: &Path) -> Vec<PathBuf> {
    let mut files = Vec::new();
    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                files.extend(walk(&path));
            } else {
                files.push(path);
            }
        }
    }
    files
}

// ===========================================================================
// atomize
// ===========================================================================

mod atomize {
    use super::*;

    /// Enrichment must populate every stub with the six fields that downstream
    /// commands depend on: code-name, code-path, code-text, code-module,
    /// dependencies, and display-name.
    #[test]
    fn enriched_stubs_have_all_required_fields() {
        let tmp = setup_project();
        assert_success(&cli(&["atomize", "--no-probe"], tmp.path()), "atomize");

        let stubs = read_stubs(tmp.path());
        let required = [
            "code-name",
            "code-path",
            "code-text",
            "code-module",
            "dependencies",
            "display-name",
        ];
        for (key, stub) in &stubs {
            for field in &required {
                assert!(
                    stub.get(*field).is_some(),
                    "stub '{}' missing required field '{}'",
                    key,
                    field
                );
            }
        }
    }

    /// When a stub already has a code-name that matches an atom, the code-name
    /// takes precedence over code-line for atom matching -- even when the
    /// code-line falls inside a different atom's range.
    #[test]
    fn code_name_takes_precedence_over_code_line() {
        let tmp = setup_project();

        let md = tmp
            .path()
            .join(".verilib/structure/src/module.rs/func_a().md");
        fs::write(
            &md,
            "---\ncode-name: \"probe:test/1.0.0/module/func_a()\"\n\
             code-path: \"src/module.rs\"\ncode-line: 25\n---\n",
        )
        .unwrap();

        assert_success(&cli(&["atomize", "--no-probe"], tmp.path()), "atomize");

        let stubs = read_stubs(tmp.path());
        let func_a = &stubs["src/module.rs/func_a().md"];
        assert_eq!(
            func_a["code-name"].as_str(),
            Some("probe:test/1.0.0/module/func_a()"),
        );
        assert_eq!(func_a["display-name"].as_str(), Some("func_a"));
    }

    /// The dependency arrays in enriched stubs must be populated from atom
    /// data, not invented or left empty.
    #[test]
    fn dependencies_come_from_atoms() {
        let tmp = setup_project();
        assert_success(&cli(&["atomize", "--no-probe"], tmp.path()), "atomize");

        let stubs = read_stubs(tmp.path());

        let deps_a: Vec<&str> = stubs["src/module.rs/func_a().md"]["dependencies"]
            .as_array()
            .unwrap()
            .iter()
            .filter_map(|v| v.as_str())
            .collect();
        assert!(deps_a.contains(&"probe:test/1.0.0/module/helper()"));

        assert!(stubs["src/module.rs/func_b().md"]["dependencies"]
            .as_array()
            .unwrap()
            .is_empty());

        let deps_c: Vec<&str> = stubs["src/other.rs/func_c().md"]["dependencies"]
            .as_array()
            .unwrap()
            .iter()
            .filter_map(|v| v.as_str())
            .collect();
        assert!(deps_c.contains(&"probe:test/1.0.0/module/func_a()"));
    }

    /// `--check-only` exits successfully when .md stub frontmatter is
    /// consistent with the enriched output (no drift).
    #[test]
    fn check_only_passes_when_stubs_match() {
        let tmp = setup_project();
        assert_success(&cli(&["atomize", "--no-probe"], tmp.path()), "atomize setup");
        assert_success(
            &cli(&["atomize", "--no-probe", "--check-only"], tmp.path()),
            "atomize --check-only",
        );
    }

    /// `--check-only` exits non-zero when a .md file has a code-name that
    /// disagrees with the enriched atom data.
    #[test]
    fn check_only_detects_code_name_mismatch() {
        let tmp = setup_project();
        let md = tmp
            .path()
            .join(".verilib/structure/src/module.rs/func_a().md");
        fs::write(
            &md,
            "---\ncode-name: \"probe:test/1.0.0/module/WRONG_NAME()\"\n\
             code-path: \"src/module.rs\"\ncode-line: 10\n---\n",
        )
        .unwrap();

        assert_failure(
            &cli(&["atomize", "--no-probe", "--check-only"], tmp.path()),
            "atomize --check-only with wrong code-name",
        );
    }

    /// `--update-stubs` writes the enriched code-name back into the .md
    /// frontmatter so that future `--check-only` runs pass.
    #[test]
    fn update_stubs_writes_code_name_to_md_files() {
        let tmp = setup_project();

        let md = tmp
            .path()
            .join(".verilib/structure/src/module.rs/func_a().md");
        fs::write(
            &md,
            "---\ncode-path: \"src/module.rs\"\ncode-line: 10\n---\n",
        )
        .unwrap();

        assert_success(
            &cli(&["atomize", "--no-probe", "--update-stubs"], tmp.path()),
            "atomize --update-stubs",
        );

        let content = fs::read_to_string(&md).unwrap();
        assert!(
            content.contains("code-name:") && content.contains("probe:test/1.0.0/module/func_a()"),
            "code-name should have been written to .md"
        );
    }

    /// Enrichment is idempotent: running atomize twice with the same inputs
    /// must produce byte-identical stubs.json.
    #[test]
    fn two_runs_produce_identical_output() {
        let tmp = setup_project();

        assert_success(&cli(&["atomize", "--no-probe"], tmp.path()), "first run");
        let first: serde_json::Value = read_json(&tmp.path().join(".verilib/stubs.json"));

        assert_success(&cli(&["atomize", "--no-probe"], tmp.path()), "second run");
        let second: serde_json::Value = read_json(&tmp.path().join(".verilib/stubs.json"));

        assert_eq!(first, second, "atomize must be idempotent");
    }

    /// `atomize --no-probe` requires atoms.json on disk; without it the
    /// command must exit non-zero.
    #[test]
    fn fails_without_atoms_json() {
        let tmp = setup_project();
        fs::remove_file(tmp.path().join(".verilib/atoms.json")).unwrap();
        assert_failure(
            &cli(&["atomize", "--no-probe"], tmp.path()),
            "atomize without atoms.json",
        );
    }

    /// A Verus project (vstd dependency) without .verilib/config.json must
    /// exit non-zero -- the user needs to run `create` first.
    #[test]
    fn fails_on_verus_project_without_config() {
        let tmp = TempDir::new().unwrap();
        fs::write(
            tmp.path().join("Cargo.toml"),
            "[package]\nname = \"verus-test\"\nversion = \"0.1.0\"\n\n\
             [dependencies]\nvstd = { git = \"https://github.com/verus-lang/verus\" }\n",
        )
        .unwrap();

        assert_failure(
            &cli(&["atomize"], tmp.path()),
            "atomize on Verus project without config",
        );
    }
}

// ===========================================================================
// atomize --atoms-only
// ===========================================================================

mod atomize_atoms_only {
    use super::*;

    /// Atoms-only mode does not require a .verilib/config.json because it
    /// skips stubs enrichment entirely.
    #[test]
    fn works_without_project_config() {
        let tmp = TempDir::new().unwrap();
        let verilib = tmp.path().join(".verilib");
        fs::create_dir_all(&verilib).unwrap();
        fs::copy(
            fixtures_dir().join("atoms.json"),
            verilib.join("atoms.json"),
        )
        .unwrap();

        assert_success(
            &cli(&["atomize", "--atoms-only", "--no-probe"], tmp.path()),
            "atoms-only without config",
        );
    }

    /// A Cargo.toml with no Verus dependencies causes atomize to auto-select
    /// atoms-only mode and exit successfully.
    #[test]
    fn auto_detected_for_pure_rust_project() {
        let tmp = TempDir::new().unwrap();
        fs::write(
            tmp.path().join("Cargo.toml"),
            "[package]\nname = \"pure-rust\"\nversion = \"0.1.0\"\nedition = \"2021\"\n\n\
             [dependencies]\nserde = \"1.0\"\n",
        )
        .unwrap();
        let verilib = tmp.path().join(".verilib");
        fs::create_dir_all(&verilib).unwrap();
        fs::copy(
            fixtures_dir().join("atoms.json"),
            verilib.join("atoms.json"),
        )
        .unwrap();

        assert_success(
            &cli(&["atomize", "--no-probe"], tmp.path()),
            "auto atoms-only for pure Rust",
        );
    }

    /// Atoms-only must not touch stubs.json, even when a full project setup
    /// exists on disk.
    #[test]
    fn does_not_modify_stubs_json() {
        let tmp = setup_project();
        let stubs_before = fs::read_to_string(tmp.path().join(".verilib/stubs.json")).unwrap();

        assert_success(
            &cli(&["atomize", "--atoms-only", "--no-probe"], tmp.path()),
            "atoms-only",
        );

        let stubs_after = fs::read_to_string(tmp.path().join(".verilib/stubs.json")).unwrap();
        assert_eq!(stubs_before, stubs_after, "stubs.json must be unchanged");
    }
}

// ===========================================================================
// specify
// ===========================================================================

mod specify {
    use super::*;

    /// After specify, stubs whose specs have `specified=true` in specs.json
    /// (func_a, func_b) must gain a `spec-text` field. Stubs with
    /// `specified=false` (func_c) must not.
    #[test]
    fn populates_spec_text_for_specified_stubs_only() {
        let tmp = setup_project_with_config("config_auto_validate.json");
        assert_success(&cli(&["atomize", "--no-probe"], tmp.path()), "atomize setup");
        assert_success(&cli(&["specify", "--no-probe"], tmp.path()), "specify");

        let stubs = read_stubs(tmp.path());
        assert!(stubs["src/module.rs/func_a().md"]
            .get("spec-text")
            .is_some());
        assert!(stubs["src/module.rs/func_b().md"]
            .get("spec-text")
            .is_some());
        assert!(stubs["src/other.rs/func_c().md"].get("spec-text").is_none());
    }

    /// The full specify flow (with auto-validate creating certs) must never
    /// modify the .md structure files on disk.
    #[test]
    fn does_not_modify_md_files() {
        let tmp = setup_project_with_config("config_auto_validate.json");
        assert_success(&cli(&["atomize", "--no-probe"], tmp.path()), "atomize setup");

        let dir = tmp.path().join(".verilib/structure");
        let before = collect_md_checksums(&dir);
        assert!(!before.is_empty());

        assert_success(
            &cli(&["specify", "--no-probe"], tmp.path()),
            "specify (auto-validate)",
        );

        assert_eq!(before, collect_md_checksums(&dir));
    }

    /// Cert files created by specify must contain an ISO 8601 timestamp.
    #[test]
    fn certs_contain_iso8601_timestamp() {
        let tmp = setup_project_with_config("config_auto_validate.json");
        assert_success(&cli(&["atomize", "--no-probe"], tmp.path()), "atomize setup");
        assert_success(
            &cli(&["specify", "--no-probe"], tmp.path()),
            "specify (auto-validate)",
        );

        let certs_dir = tmp.path().join(".verilib/certs/specs");
        assert!(certs_dir.exists());

        let certs: Vec<_> = fs::read_dir(&certs_dir)
            .unwrap()
            .flatten()
            .filter(|e| e.path().extension().is_some_and(|ext| ext == "json"))
            .collect();
        assert!(!certs.is_empty(), "auto-validate should create cert files");

        for entry in &certs {
            let cert = read_json(&entry.path());
            let ts = cert["timestamp"]
                .as_str()
                .unwrap_or_else(|| panic!("cert {} has no timestamp", entry.path().display()));
            assert!(
                ts.contains('T') && ts.contains(':'),
                "timestamp '{}' should be ISO 8601",
                ts
            );
        }
    }

    /// `--check-only` exits successfully when every specified stub has a
    /// corresponding cert file on disk.
    #[test]
    fn check_only_passes_when_all_stubs_certified() {
        let tmp = setup_project();

        let cert_path = tmp
            .path()
            .join(".verilib/certs/specs/probe%3Atest%2F1%2E0%2E0%2Fmodule%2Ffunc_b%28%29.json");
        fs::write(
            &cert_path,
            r#"{"timestamp": "2026-01-27T10:00:00.000000000Z"}"#,
        )
        .unwrap();

        assert_success(
            &cli(&["specify", "--no-probe", "--check-only"], tmp.path()),
            "specify --check-only (all certified)",
        );
    }

    /// `specify --no-probe` requires specs.json on disk; without it the
    /// command must exit non-zero.
    #[test]
    fn fails_without_specs_json() {
        let tmp = setup_project();
        fs::remove_file(tmp.path().join(".verilib/specs.json")).unwrap();
        assert_failure(
            &cli(&["specify", "--no-probe", "--check-only"], tmp.path()),
            "specify without specs.json",
        );
    }
}

// ===========================================================================
// verify
// ===========================================================================

mod verify {
    use super::*;

    /// After verify, each stub's `verified` field must reflect the
    /// corresponding entry in proofs.json.
    #[test]
    fn sets_verified_field_from_proofs() {
        let tmp = setup_project();
        assert_success(&cli(&["atomize", "--no-probe"], tmp.path()), "atomize setup");
        assert_success(&cli(&["verify", "--no-probe"], tmp.path()), "verify");

        let stubs = read_stubs(tmp.path());
        assert_eq!(
            stubs["src/module.rs/func_a().md"]["verified"].as_bool(),
            Some(true)
        );
        assert_eq!(
            stubs["src/module.rs/func_b().md"]["verified"].as_bool(),
            Some(false)
        );
    }

    /// The verify flow must never modify the .md structure files on disk.
    #[test]
    fn does_not_modify_md_files() {
        let tmp = setup_project();
        let dir = tmp.path().join(".verilib/structure");
        let before = collect_md_checksums(&dir);
        assert!(!before.is_empty());

        assert_success(&cli(&["verify", "--no-probe"], tmp.path()), "verify");

        assert_eq!(before, collect_md_checksums(&dir));
    }

    /// `--check-only` exits non-zero when stubs.json contains entries with
    /// `status: "failure"`.
    #[test]
    fn check_only_detects_failure_status() {
        let tmp = setup_project();
        assert_failure(
            &cli(&["verify", "--check-only"], tmp.path()),
            "verify --check-only with failures",
        );
    }

    /// `--check-only` exits successfully when no stub has a failure status.
    #[test]
    fn check_only_passes_when_no_failures() {
        let tmp = setup_project();
        let stubs_path = tmp.path().join(".verilib/stubs.json");
        let mut stubs: serde_json::Value =
            serde_json::from_str(&fs::read_to_string(&stubs_path).unwrap()).unwrap();

        if let Some(obj) = stubs.as_object_mut() {
            for stub in obj.values_mut() {
                if let Some(o) = stub.as_object_mut() {
                    o.remove("status");
                }
            }
        }
        fs::write(&stubs_path, serde_json::to_string_pretty(&stubs).unwrap()).unwrap();

        assert_success(
            &cli(&["verify", "--check-only"], tmp.path()),
            "verify --check-only (clean)",
        );
    }

    /// Verification is idempotent: running verify twice with the same inputs
    /// must produce byte-identical stubs.json.
    #[test]
    fn two_runs_produce_identical_output() {
        let tmp = setup_project();
        assert_success(&cli(&["atomize", "--no-probe"], tmp.path()), "atomize setup");

        assert_success(&cli(&["verify", "--no-probe"], tmp.path()), "first run");
        let first: serde_json::Value = read_json(&tmp.path().join(".verilib/stubs.json"));

        assert_success(&cli(&["verify", "--no-probe"], tmp.path()), "second run");
        let second: serde_json::Value = read_json(&tmp.path().join(".verilib/stubs.json"));

        assert_eq!(first, second, "verify must be idempotent");
    }

    /// `verify --no-probe` requires proofs.json on disk; without it the
    /// command must exit non-zero.
    #[test]
    fn fails_without_proofs_json() {
        let tmp = setup_project();
        fs::remove_file(tmp.path().join(".verilib/proofs.json")).unwrap();
        assert_failure(
            &cli(&["verify", "--no-probe"], tmp.path()),
            "verify without proofs.json",
        );
    }

    /// `verify --check-only` requires stubs.json to exist; without it the
    /// command must exit non-zero.
    #[test]
    fn check_only_fails_without_stubs_json() {
        let tmp = TempDir::new().unwrap();
        let verilib = tmp.path().join(".verilib");
        fs::create_dir_all(&verilib).unwrap();
        fs::write(
            verilib.join("config.json"),
            r#"{"structure-root": ".verilib/structure"}"#,
        )
        .unwrap();

        assert_failure(
            &cli(&["verify", "--check-only"], tmp.path()),
            "verify without stubs.json",
        );
    }
}

// ===========================================================================
// create
// ===========================================================================

mod create {
    use super::*;

    /// `create` generates `.verilib/config.json` containing a `structure-root`
    /// field, even when probe-verus is not available (config is written before
    /// the probe-verus step).
    #[test]
    fn produces_config_with_structure_root() {
        let tmp = TempDir::new().unwrap();
        cli(&["create"], tmp.path());

        let config_path = tmp.path().join(".verilib/config.json");
        assert!(
            config_path.exists(),
            "config.json should exist after create"
        );

        let config = read_json(&config_path);
        assert_eq!(
            config["structure-root"].as_str(),
            Some(".verilib/structure"),
        );
    }
}

// ===========================================================================
// pipeline (requires unix for mock probe-verus symlink)
// ===========================================================================

#[cfg(unix)]
mod pipeline {
    use super::*;

    fn setup_mock_probe_dir() -> TempDir {
        let mock_dir = TempDir::new().expect("Failed to create mock dir");
        let mock_binary = PathBuf::from(env!("CARGO_BIN_EXE_mock-probe-verus"));
        std::os::unix::fs::symlink(&mock_binary, mock_dir.path().join("probe-verus"))
            .expect("Failed to symlink mock probe-verus");
        mock_dir
    }

    fn cli_with_mock(args: &[&str], cwd: &Path, mock_bin_dir: &Path) -> Output {
        let mut paths = vec![mock_bin_dir.to_path_buf()];
        paths.extend(std::env::split_paths(
            &std::env::var("PATH").unwrap_or_default(),
        ));
        let new_path = std::env::join_paths(paths).expect("Failed to join PATH");
        Command::new(env!("CARGO_BIN_EXE_verilib-cli"))
            .args(args)
            .current_dir(cwd)
            .env("PATH", new_path)
            .env("MOCK_FIXTURES_DIR", fixtures_dir())
            .output()
            .expect("Failed to execute verilib-cli")
    }

    /// End-to-end: create -> atomize --update-stubs -> specify -> verify,
    /// all driven by a mock probe-verus binary. Verifies the pipeline
    /// produces the expected artifacts at each stage.
    #[test]
    fn full_create_atomize_specify_verify_workflow() {
        let mock_dir = setup_mock_probe_dir();
        let tmp = TempDir::new().unwrap();

        fs::write(
            tmp.path().join("Cargo.toml"),
            "[package]\nname = \"test-verus-project\"\nversion = \"0.1.0\"\nedition = \"2021\"\n\n\
             [dependencies]\nvstd = { git = \"https://github.com/verus-lang/verus\", rev = \"test\" }\n",
        )
        .unwrap();

        // create
        assert_success(
            &cli_with_mock(&["create"], tmp.path(), mock_dir.path()),
            "create",
        );
        let config_path = tmp.path().join(".verilib/config.json");
        assert!(config_path.exists());
        let config = read_json(&config_path);
        assert!(config.get("structure-root").is_some());

        // Enable auto-validate for the specify step
        let mut cfg: serde_json::Value =
            serde_json::from_str(&fs::read_to_string(&config_path).unwrap()).unwrap();
        cfg["auto-validate-specs"] = serde_json::Value::Bool(true);
        fs::write(&config_path, serde_json::to_string_pretty(&cfg).unwrap()).unwrap();

        // atomize --update-stubs
        assert_success(
            &cli_with_mock(&["atomize", "--update-stubs"], tmp.path(), mock_dir.path()),
            "atomize --update-stubs",
        );
        let stubs_path = tmp.path().join(".verilib/stubs.json");
        assert!(stubs_path.exists());
        let stubs = read_stubs(tmp.path());
        assert!(!stubs.is_empty());
        for (key, stub) in &stubs {
            assert!(
                stub.get("code-name").is_some(),
                "stub '{}' should have code-name",
                key
            );
        }

        // specify
        assert_success(
            &cli_with_mock(&["specify"], tmp.path(), mock_dir.path()),
            "specify",
        );

        // verify
        assert_success(
            &cli_with_mock(&["verify"], tmp.path(), mock_dir.path()),
            "verify",
        );
        let final_stubs = read_stubs(tmp.path());
        for (key, stub) in &final_stubs {
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
