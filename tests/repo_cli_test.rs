use serde_json::Value;
use std::{
    fs,
    process::{Command, Stdio},
};
use tempfile::TempDir;

#[test]
fn init_closed_stdin_is_noninteractive_and_keeps_config() {
    let dir = TempDir::new().unwrap();
    let result = Command::new(env!("CARGO_BIN_EXE_verilib-cli"))
        .args([
            "--json",
            "init",
            "--id",
            "42",
            "--url",
            "http://localhost:1234",
            "--execution-mode",
            "docker",
        ])
        .env_remove("VERILIB_BASE_URL")
        .current_dir(dir.path())
        .stdin(Stdio::null())
        .output()
        .unwrap();
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    let output: Value = serde_json::from_slice(&result.stdout).unwrap();
    assert_eq!(output["repo_id"], "42");
    assert_eq!(
        output["repository_url"],
        "http://localhost:1234/repobrowser?id=42"
    );
    let config: Value =
        serde_json::from_str(&fs::read_to_string(dir.path().join(".verilib/config.json")).unwrap())
            .unwrap();
    assert_eq!(config["execution-mode"], "docker");
}

#[test]
fn invalid_metadata_fails_before_auth_or_any_request() {
    let dir = TempDir::new().unwrap();
    let result = Command::new(env!("CARGO_BIN_EXE_verilib-cli"))
        .args([
            "--json",
            "repo",
            "create",
            "--git-url",
            "https://github.com/example/repo",
            "--summary",
            "ok",
            "--language-id",
            "2",
            "--prooflanguage-id",
            "2",
            "--type-id",
            "8",
            "--description",
            &"é".repeat(513),
        ])
        .current_dir(dir.path())
        .stdin(Stdio::null())
        .output()
        .unwrap();
    assert!(!result.status.success());
    let output: Value = serde_json::from_slice(&result.stdout).unwrap();
    assert_eq!(output["error"]["code"], "VALIDATION_ERROR");
    assert_eq!(output["error"]["fields"][0]["actual_length"], 513);
    assert!(!dir.path().join(".verilib").exists());
}

#[test]
fn missing_init_id_and_invalid_wait_options_fail_promptly() {
    let dir = TempDir::new().unwrap();
    for args in [
        vec!["--json", "init"],
        vec!["wait-for-ready", "--id", "42", "--timeout", "0"],
        vec!["repo", "status", "--poll-interval", "0"],
    ] {
        let result = Command::new(env!("CARGO_BIN_EXE_verilib-cli"))
            .args(args)
            .current_dir(dir.path())
            .stdin(Stdio::null())
            .output()
            .unwrap();
        assert!(!result.status.success());
    }
}
