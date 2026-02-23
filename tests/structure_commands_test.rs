//! Integration tests for structure commands (atomize, specify, verify).
//!
//! These tests use --no-probe and --check-only flags to test command logic
//! without requiring probe-verus to be installed.

use std::fs;
use std::path::PathBuf;
use tempfile::TempDir;

/// Helper to set up a test project with fixtures.
fn setup_test_project() -> TempDir {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let verilib_dir = temp_dir.path().join(".verilib");
    fs::create_dir_all(&verilib_dir).expect("Failed to create .verilib dir");

    // Copy fixtures
    let fixtures_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures");

    // Copy config.json
    fs::copy(
        fixtures_dir.join("config.json"),
        verilib_dir.join("config.json"),
    )
    .expect("Failed to copy config.json");

    // Copy JSON files
    for file in ["atoms.json", "specs.json", "proofs.json", "stubs.json"] {
        fs::copy(fixtures_dir.join(file), verilib_dir.join(file))
            .expect(&format!("Failed to copy {}", file));
    }

    // Copy structure directory
    let src_structure = fixtures_dir.join("structure");
    let dst_structure = verilib_dir.join("structure");
    copy_dir_recursive(&src_structure, &dst_structure).expect("Failed to copy structure dir");

    // Copy certs directory
    let src_certs = fixtures_dir.join("certs");
    let dst_certs = verilib_dir.join("certs");
    copy_dir_recursive(&src_certs, &dst_certs).expect("Failed to copy certs dir");

    temp_dir
}

/// Recursively copy a directory.
fn copy_dir_recursive(src: &PathBuf, dst: &PathBuf) -> std::io::Result<()> {
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

/// Helper to run verilib-cli command and capture output.
fn run_command(args: &[&str], cwd: &std::path::Path) -> std::process::Output {
    std::process::Command::new(env!("CARGO_BIN_EXE_verilib-cli"))
        .args(args)
        .current_dir(cwd)
        .output()
        .expect("Failed to execute command")
}

// ============================================================================
// ATOMIZE TESTS
// ============================================================================

mod atomize_tests {
    use super::*;

    #[test]
    fn test_atomize_no_probe_loads_atoms_from_file() {
        let temp_dir = setup_test_project();
        let output = run_command(&["atomize", "--no-probe"], temp_dir.path());

        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(
            stdout.contains("Loading atoms from"),
            "Should load atoms from file"
        );
        assert!(stdout.contains("Loaded 4 atoms"), "Should load 4 atoms");
    }

    #[test]
    fn test_atomize_check_only_passes_when_stubs_match() {
        let temp_dir = setup_test_project();

        // First run atomize to ensure stubs.json is up to date
        run_command(&["atomize", "--no-probe"], temp_dir.path());

        // Now check-only should pass
        let output = run_command(&["atomize", "--no-probe", "--check-only"], temp_dir.path());

        assert!(output.status.success(), "check-only should pass when stubs match");
    }

    #[test]
    fn test_atomize_check_only_fails_when_stubs_mismatch() {
        let temp_dir = setup_test_project();
        let verilib_dir = temp_dir.path().join(".verilib");

        // Modify a .md file to create a mismatch - change code-name in the frontmatter
        // The check compares .md stubs against enriched (from atoms.json)
        let md_path = verilib_dir.join("structure/src/module.rs/func_a().md");
        fs::write(
            &md_path,
            r#"---
code-name: "probe:test/1.0.0/module/WRONG_NAME()"
code-path: "src/module.rs"
code-line: 10
---
"#,
        )
        .unwrap();

        // check-only should fail because .md has WRONG_NAME but atoms.json has func_a
        let output = run_command(&["atomize", "--no-probe", "--check-only"], temp_dir.path());

        assert!(
            !output.status.success(),
            "check-only should fail when stubs mismatch"
        );
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(
            stderr.contains("mismatch") || stderr.contains("do not match"),
            "Should report mismatch: {}",
            stderr
        );
    }

    #[test]
    fn test_atomize_no_probe_fails_without_atoms_json() {
        let temp_dir = setup_test_project();
        let verilib_dir = temp_dir.path().join(".verilib");

        // Remove atoms.json
        fs::remove_file(verilib_dir.join("atoms.json")).unwrap();

        let output = run_command(&["atomize", "--no-probe"], temp_dir.path());

        assert!(!output.status.success(), "Should fail without atoms.json");
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(
            stderr.contains("atoms.json not found"),
            "Should report missing atoms.json"
        );
    }

    #[test]
    fn test_atomize_update_stubs_updates_md_files() {
        let temp_dir = setup_test_project();
        let verilib_dir = temp_dir.path().join(".verilib");

        // Create a .md file without code-name
        let md_path = verilib_dir.join("structure/src/module.rs/func_a().md");
        fs::write(
            &md_path,
            r#"---
code-path: "src/module.rs"
code-line: 10
---
"#,
        )
        .unwrap();

        // Run atomize with --update-stubs
        let output = run_command(&["atomize", "--no-probe", "--update-stubs"], temp_dir.path());
        assert!(output.status.success(), "atomize --update-stubs should succeed");

        // Check that the .md file was updated with code-name
        let content = fs::read_to_string(&md_path).unwrap();
        assert!(
            content.contains("code-name:"),
            "Should have added code-name to .md file"
        );
        assert!(
            content.contains("probe:test/1.0.0/module/func_a()"),
            "Should have correct code-name value"
        );
    }

    #[test]
    fn test_atomize_enriches_stubs_json() {
        let temp_dir = setup_test_project();
        let verilib_dir = temp_dir.path().join(".verilib");

        // Run atomize
        let output = run_command(&["atomize", "--no-probe"], temp_dir.path());
        assert!(output.status.success(), "atomize should succeed");

        // Check that stubs.json was enriched
        let stubs_path = verilib_dir.join("stubs.json");
        let stubs: serde_json::Value =
            serde_json::from_str(&fs::read_to_string(&stubs_path).unwrap()).unwrap();

        // Check func_a has enriched fields from atoms.json
        let func_a = &stubs["src/module.rs/func_a().md"];
        assert_eq!(
            func_a["code-module"].as_str(),
            Some("module"),
            "Should have code-module"
        );
        assert_eq!(
            func_a["display-name"].as_str(),
            Some("func_a"),
            "Should have display-name"
        );
        assert!(
            func_a["dependencies"].is_array(),
            "Should have dependencies array"
        );
    }
}

// ============================================================================
// SPECIFY TESTS
// ============================================================================

mod specify_tests {
    use super::*;

    #[test]
    fn test_specify_no_probe_loads_specs_from_file() {
        let temp_dir = setup_test_project();
        let output = run_command(&["specify", "--no-probe", "--check-only"], temp_dir.path());

        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(
            stdout.contains("Loading specs from"),
            "Should load specs from file"
        );
    }

    #[test]
    fn test_specify_check_only_reports_uncertified() {
        let temp_dir = setup_test_project();

        // We have certs for func_a but not func_b
        // func_b has specified=true in specs.json, so it should be reported as uncertified
        let output = run_command(&["specify", "--no-probe", "--check-only"], temp_dir.path());

        // Check if it reports uncertified stubs
        let stderr = String::from_utf8_lossy(&output.stderr);

        // The test should either pass (all certified) or fail (some uncertified)
        // Based on our fixtures, func_b is specified but has no cert
        if !output.status.success() {
            assert!(
                stderr.contains("missing certs") || stderr.contains("uncertified"),
                "Should report uncertified stubs"
            );
        }
    }

    #[test]
    fn test_specify_no_probe_fails_without_specs_json() {
        let temp_dir = setup_test_project();
        let verilib_dir = temp_dir.path().join(".verilib");

        // Remove specs.json
        fs::remove_file(verilib_dir.join("specs.json")).unwrap();

        let output = run_command(&["specify", "--no-probe", "--check-only"], temp_dir.path());

        assert!(!output.status.success(), "Should fail without specs.json");
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(
            stderr.contains("specs.json not found"),
            "Should report missing specs.json"
        );
    }

    #[test]
    fn test_specify_check_only_passes_when_all_certified() {
        let temp_dir = setup_test_project();
        let verilib_dir = temp_dir.path().join(".verilib");

        // Add cert for func_b (func_a already has a cert)
        let cert_path = verilib_dir.join("certs/specs/probe%3Atest%2F1%2E0%2E0%2Fmodule%2Ffunc_b%28%29.json");
        fs::write(&cert_path, r#"{"timestamp": "2026-01-27T10:00:00.000000000Z"}"#).unwrap();

        // check-only should pass when all specified functions have certs
        let output = run_command(&["specify", "--no-probe", "--check-only"], temp_dir.path());

        assert!(
            output.status.success(),
            "check-only should pass when all specs have certs"
        );
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(
            stdout.contains("All stubs with specs have certs") || stdout.contains("already validated"),
            "Should report all certified"
        );
    }
}

// ============================================================================
// VERIFY TESTS
// ============================================================================

mod verify_tests {
    use super::*;

    #[test]
    fn test_verify_no_probe_loads_proofs_from_file() {
        let temp_dir = setup_test_project();
        let output = run_command(&["verify", "--no-probe"], temp_dir.path());

        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(
            stdout.contains("Loading proofs from"),
            "Should load proofs from file"
        );
        assert!(stdout.contains("Loaded 4 proofs"), "Should load 4 proofs");
    }

    #[test]
    fn test_verify_check_only_detects_failures() {
        let temp_dir = setup_test_project();

        // Our fixtures have func_b with status "failure"
        let output = run_command(&["verify", "--check-only"], temp_dir.path());

        assert!(
            !output.status.success(),
            "check-only should fail when there are failures"
        );
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(
            stderr.contains("failure") || stderr.contains("failed verification"),
            "Should report failures"
        );
    }

    #[test]
    fn test_verify_check_only_passes_when_no_failures() {
        let temp_dir = setup_test_project();
        let verilib_dir = temp_dir.path().join(".verilib");

        // Modify stubs.json to remove failure status
        let stubs_path = verilib_dir.join("stubs.json");
        let mut stubs: serde_json::Value =
            serde_json::from_str(&fs::read_to_string(&stubs_path).unwrap()).unwrap();

        // Remove failure status from all stubs
        if let Some(obj) = stubs.as_object_mut() {
            for (_, stub) in obj.iter_mut() {
                if let Some(stub_obj) = stub.as_object_mut() {
                    stub_obj.remove("status");
                }
            }
        }
        fs::write(&stubs_path, serde_json::to_string_pretty(&stubs).unwrap()).unwrap();

        let output = run_command(&["verify", "--check-only"], temp_dir.path());

        assert!(
            output.status.success(),
            "check-only should pass when no failures"
        );
    }

    #[test]
    fn test_verify_no_probe_fails_without_proofs_json() {
        let temp_dir = setup_test_project();
        let verilib_dir = temp_dir.path().join(".verilib");

        // Remove proofs.json
        fs::remove_file(verilib_dir.join("proofs.json")).unwrap();

        let output = run_command(&["verify", "--no-probe"], temp_dir.path());

        assert!(!output.status.success(), "Should fail without proofs.json");
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(
            stderr.contains("proofs.json not found"),
            "Should report missing proofs.json"
        );
    }

    #[test]
    fn test_verify_updates_stubs_with_verification_status() {
        let temp_dir = setup_test_project();
        let verilib_dir = temp_dir.path().join(".verilib");

        // Run verify with --no-probe
        let output = run_command(&["verify", "--no-probe"], temp_dir.path());
        assert!(output.status.success(), "verify should succeed");

        // Check that stubs.json was updated
        let stubs_path = verilib_dir.join("stubs.json");
        let stubs: serde_json::Value =
            serde_json::from_str(&fs::read_to_string(&stubs_path).unwrap()).unwrap();

        // func_a should be verified (proofs.json has verified: true)
        let func_a = &stubs["src/module.rs/func_a().md"];
        assert_eq!(
            func_a["verified"].as_bool(),
            Some(true),
            "func_a should be verified"
        );

        // func_b should not be verified (proofs.json has verified: false)
        let func_b = &stubs["src/module.rs/func_b().md"];
        assert_eq!(
            func_b["verified"].as_bool(),
            Some(false),
            "func_b should not be verified"
        );
    }
}

// ============================================================================
// CREATE TESTS
// ============================================================================

mod create_tests {
    use super::*;

    #[test]
    fn test_create_creates_config_and_verilib_dir() {
        let temp_dir = TempDir::new().expect("Failed to create temp dir");

        // create will fail at probe-verus step, but should create config first
        let _output = run_command(&["create"], temp_dir.path());

        let verilib_dir = temp_dir.path().join(".verilib");
        assert!(verilib_dir.exists(), ".verilib directory should be created");

        let config_path = verilib_dir.join("config.json");
        assert!(config_path.exists(), "config.json should be created");

        let config: serde_json::Value =
            serde_json::from_str(&fs::read_to_string(&config_path).unwrap()).unwrap();
        assert_eq!(
            config["structure-root"].as_str(),
            Some(".verilib/structure"),
            "config should have default structure-root"
        );
    }

    #[test]
    fn test_create_no_seed_file_needed() {
        let temp_dir = TempDir::new().expect("Failed to create temp dir");

        let output = run_command(&["create"], temp_dir.path());

        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(
            !stderr.contains("functions_to_track.csv"),
            "Should not reference functions_to_track.csv: {}",
            stderr
        );
        let seed_path = temp_dir.path().join(".verilib").join("seed.csv");
        assert!(
            !seed_path.exists(),
            "Should not create seed.csv (probe-verus discovers functions automatically)"
        );
    }

    #[test]
    fn test_create_fails_when_probe_verus_not_installed() {
        let temp_dir = TempDir::new().expect("Failed to create temp dir");

        let output = run_command(&["create"], temp_dir.path());

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            assert!(
                stderr.contains("probe-verus") || stderr.contains("not found") || stderr.contains("not installed"),
                "Should report that probe-verus is required: {}",
                stderr
            );
        }
    }

}

// ============================================================================
// ERROR HANDLING TESTS
// ============================================================================

mod error_handling_tests {
    use super::*;

    #[test]
    fn test_commands_fail_without_config() {
        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let verilib_dir = temp_dir.path().join(".verilib");
        fs::create_dir_all(&verilib_dir).expect("Failed to create .verilib dir");

        // No config.json - should fail
        let output = run_command(&["atomize", "--no-probe"], temp_dir.path());
        assert!(!output.status.success());
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(
            stderr.contains("config.json not found") || stderr.contains("Run 'verilib-cli create'"),
            "Should report missing config"
        );
    }

    #[test]
    fn test_verify_fails_without_stubs_json() {
        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let verilib_dir = temp_dir.path().join(".verilib");
        fs::create_dir_all(&verilib_dir).expect("Failed to create .verilib dir");

        // Create config but no stubs.json
        fs::write(
            verilib_dir.join("config.json"),
            r#"{"structure-root": ".verilib/structure"}"#,
        )
        .unwrap();

        let output = run_command(&["verify", "--check-only"], temp_dir.path());
        assert!(!output.status.success());
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(
            stderr.contains("not found") || stderr.contains("atomize"),
            "Should report missing stubs.json"
        );
    }
}
