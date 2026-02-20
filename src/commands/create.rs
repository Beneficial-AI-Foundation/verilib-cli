//! Create subcommand implementation.
//!
//! Initialize structure files from source analysis using probe-verus.

use crate::structure::{
    parse_github_link, require_probe_installed, run_command, write_frontmatter, CommandConfig,
    ConfigPaths, StructureConfig,
};
use anyhow::{bail, Context, Result};
use serde_json::{json, Value};
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

/// Run the create subcommand.
pub async fn handle_create(
    project_root: PathBuf,
    root: Option<PathBuf>,
    github_base_url: Option<String>,
) -> Result<()> {
    let project_root = project_root
        .canonicalize()
        .context("Failed to resolve project root")?;
    let verilib_path = project_root.join(".verilib");
    std::fs::create_dir_all(&verilib_path).context("Failed to create .verilib directory")?;

    let structure_root_relative = root
        .map(|r| r.to_string_lossy().to_string())
        .unwrap_or_else(|| ".verilib/structure".to_string());

    let config = StructureConfig::new(&structure_root_relative);
    let config_path = config.save(&project_root)?;
    println!("Wrote config to {}", config_path.display());

    let github_base = resolve_github_base_url(github_base_url, &project_root)?;
    let tracked_output_path = verilib_path.join("tracked_functions.csv");

    let paths = ConfigPaths::load(&project_root)?;
    run_probe_verus_tracked_csv(&project_root, &tracked_output_path, &github_base, &paths.command_config)?;

    let tracked = read_tracked_csv(&tracked_output_path)?;
    let tracked = disambiguate_names(tracked);
    let structure = tracked_to_structure(&tracked);

    println!("\nGenerating structure files...");
    let structure_root = project_root.join(&structure_root_relative);
    generate_structure_files(&structure, &structure_root)?;

    Ok(())
}

/// Derive a GitHub base URL from the git remote, or use the explicit override.
///
/// The returned URL always ends with `/blob/<branch>/` so that probe-verus
/// produces links compatible with `parse_github_link()`.
fn resolve_github_base_url(
    explicit: Option<String>,
    project_root: &Path,
) -> Result<String> {
    let repo_url = match explicit {
        Some(url) => {
            if !url.starts_with("https://") && !url.starts_with("http://") {
                bail!(
                    "Invalid --github-base-url '{}': must start with https:// or http://",
                    url
                );
            }
            strip_to_repo_url(&url)
        }
        None => {
            let output = git_command(project_root, &["remote", "get-url", "origin"])
                .context("Failed to run 'git remote get-url origin'")?;

            if !output.status.success() {
                bail!(
                    "Could not auto-detect GitHub URL (no git remote 'origin' found). \
                     Please pass --github-base-url explicitly."
                );
            }

            let remote = String::from_utf8_lossy(&output.stdout).trim().to_string();
            parse_git_remote_to_https(&remote).ok_or_else(|| {
                anyhow::anyhow!(
                    "Git remote '{}' is not a recognized GitHub URL. \
                     Pass --github-base-url explicitly.",
                    remote
                )
            })?
        }
    };

    let branch = detect_default_branch(project_root);
    Ok(format!("{}/blob/{}/", repo_url, branch))
}

/// Strip a GitHub URL to the repo root (e.g. `https://github.com/Org/Repo`).
///
/// Handles URLs that contain `/blob/...` paths or a `.git` suffix.
fn strip_to_repo_url(url: &str) -> String {
    let url = url.trim_end_matches('/');
    if let Some(idx) = url.find("/blob/") {
        return url[..idx].to_string();
    }
    url.trim_end_matches(".git").to_string()
}

/// Detect the default branch by inspecting the remote HEAD ref, falling back
/// to `"main"`.
///
/// We intentionally skip the current local branch as a fallback because it is
/// often a feature branch, which would produce broken GitHub links once merged.
fn detect_default_branch(project_root: &Path) -> String {
    if let Some(branch) = detect_remote_default_branch(project_root) {
        return branch;
    }
    eprintln!("Warning: Could not detect remote default branch, defaulting to 'main'");
    "main".to_string()
}

fn detect_remote_default_branch(project_root: &Path) -> Option<String> {
    let output = git_command(project_root, &["symbolic-ref", "refs/remotes/origin/HEAD"]).ok()?;

    if !output.status.success() {
        return None;
    }

    let refname = String::from_utf8_lossy(&output.stdout).trim().to_string();
    refname.strip_prefix("refs/remotes/origin/").map(String::from)
}

/// Run a git command in the given project root directory.
///
/// Git always inspects the local repo and must run locally, bypassing the
/// `CommandConfig` execution-mode abstraction.
fn git_command(project_root: &Path, args: &[&str]) -> std::io::Result<std::process::Output> {
    std::process::Command::new("git")
        .args(args)
        .current_dir(project_root)
        .output()
}

/// Convert a git remote URL to a plain `https://github.com/Org/Repo` URL.
///
/// Supports SCP-style SSH, URL-style SSH, HTTPS, and HTTP remotes.
/// Only github.com remotes are recognized; all others return `None`.
fn parse_git_remote_to_https(remote: &str) -> Option<String> {
    let remote = remote.trim();

    // SCP-style SSH: git@github.com:Org/Repo.git
    if let Some(rest) = remote.strip_prefix("git@github.com:") {
        let repo = rest.trim_end_matches(".git").trim_end_matches('/');
        return Some(format!("https://github.com/{}", repo));
    }

    // URL-style SSH: ssh://git@github.com/Org/Repo.git
    if let Some(rest) = remote.strip_prefix("ssh://git@github.com/") {
        let repo = rest.trim_end_matches(".git").trim_end_matches('/');
        return Some(format!("https://github.com/{}", repo));
    }

    if remote.starts_with("https://github.com/") {
        let url = remote.trim_end_matches(".git").trim_end_matches('/');
        return Some(url.to_string());
    }
    if remote.starts_with("http://github.com/") {
        let url = remote.trim_end_matches(".git").trim_end_matches('/');
        return Some(url.replacen("http://", "https://", 1));
    }

    None
}

/// Run `probe-verus tracked-csv` to generate the tracked functions CSV.
fn run_probe_verus_tracked_csv(
    project_root: &Path,
    output_path: &Path,
    github_base_url: &str,
    config: &CommandConfig,
) -> Result<()> {
    require_probe_installed(config)?;

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
        "probe-verus",
        &[
            "tracked-csv",
            ".",
            "--output",
            output_str,
            "--github-base-url",
            github_base_url,
        ],
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

#[cfg(test)]
mod tests {
    use super::*;

    // --- parse_git_remote_to_https ---

    #[test]
    fn test_parse_ssh_remote() {
        assert_eq!(
            parse_git_remote_to_https("git@github.com:Org/Repo.git"),
            Some("https://github.com/Org/Repo".to_string())
        );
    }

    #[test]
    fn test_parse_ssh_remote_no_suffix() {
        assert_eq!(
            parse_git_remote_to_https("git@github.com:Org/Repo"),
            Some("https://github.com/Org/Repo".to_string())
        );
    }

    #[test]
    fn test_parse_https_remote() {
        assert_eq!(
            parse_git_remote_to_https("https://github.com/Org/Repo.git"),
            Some("https://github.com/Org/Repo".to_string())
        );
    }

    #[test]
    fn test_parse_https_remote_no_suffix() {
        assert_eq!(
            parse_git_remote_to_https("https://github.com/Org/Repo"),
            Some("https://github.com/Org/Repo".to_string())
        );
    }

    #[test]
    fn test_parse_https_remote_trailing_slash() {
        assert_eq!(
            parse_git_remote_to_https("https://github.com/Org/Repo/"),
            Some("https://github.com/Org/Repo".to_string())
        );
    }

    #[test]
    fn test_parse_http_remote_normalized_to_https() {
        assert_eq!(
            parse_git_remote_to_https("http://github.com/Org/Repo.git"),
            Some("https://github.com/Org/Repo".to_string())
        );
    }

    #[test]
    fn test_parse_http_remote_no_suffix_normalized() {
        assert_eq!(
            parse_git_remote_to_https("http://github.com/Org/Repo"),
            Some("https://github.com/Org/Repo".to_string())
        );
    }

    #[test]
    fn test_parse_non_github_remote_returns_none() {
        assert_eq!(
            parse_git_remote_to_https("git@gitlab.com:Org/Repo.git"),
            None
        );
    }

    #[test]
    fn test_parse_ssh_url_remote() {
        assert_eq!(
            parse_git_remote_to_https("ssh://git@github.com/Org/Repo.git"),
            Some("https://github.com/Org/Repo".to_string())
        );
    }

    #[test]
    fn test_parse_empty_remote_returns_none() {
        assert_eq!(parse_git_remote_to_https(""), None);
    }

    // --- strip_to_repo_url ---

    #[test]
    fn test_strip_plain_url_unchanged() {
        assert_eq!(
            strip_to_repo_url("https://github.com/Org/Repo"),
            "https://github.com/Org/Repo"
        );
    }

    #[test]
    fn test_strip_url_with_blob_path() {
        assert_eq!(
            strip_to_repo_url("https://github.com/Org/Repo/blob/main/src/lib.rs"),
            "https://github.com/Org/Repo"
        );
    }

    #[test]
    fn test_strip_url_with_blob_branch_only() {
        assert_eq!(
            strip_to_repo_url("https://github.com/Org/Repo/blob/master"),
            "https://github.com/Org/Repo"
        );
    }

    #[test]
    fn test_strip_url_trailing_slash() {
        assert_eq!(
            strip_to_repo_url("https://github.com/Org/Repo/"),
            "https://github.com/Org/Repo"
        );
    }

    #[test]
    fn test_strip_url_dot_git_suffix() {
        assert_eq!(
            strip_to_repo_url("https://github.com/Org/Repo.git"),
            "https://github.com/Org/Repo"
        );
    }

    // --- disambiguate_names ---

    #[test]
    fn test_disambiguate_names_deterministic() {
        let mut tracked = HashMap::new();
        tracked.insert(
            "dup::mod_b".to_string(),
            TrackedFunction { link: String::new(), qualified_name: "dup".into() },
        );
        tracked.insert(
            "dup::mod_a".to_string(),
            TrackedFunction { link: String::new(), qualified_name: "dup".into() },
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
            TrackedFunction { link: String::new(), qualified_name: "foo".into() },
        );
        tracked.insert(
            "bar::mod_b".to_string(),
            TrackedFunction { link: String::new(), qualified_name: "bar".into() },
        );

        let result = disambiguate_names(tracked);
        assert_eq!(result["foo::mod_a"].qualified_name, "foo");
        assert_eq!(result["bar::mod_b"].qualified_name, "bar");
    }

    // --- resolve_github_base_url ---

    #[test]
    fn test_resolve_explicit_plain_url() {
        let tmp = tempfile::TempDir::new().unwrap();
        let result =
            resolve_github_base_url(Some("https://github.com/Org/Repo".into()), tmp.path())
                .unwrap();
        assert_eq!(result, "https://github.com/Org/Repo/blob/main/");
    }

    #[test]
    fn test_resolve_explicit_url_strips_blob_and_redetects() {
        // /blob/master is stripped; branch re-detected as "main" (no git repo in tmp)
        let tmp = tempfile::TempDir::new().unwrap();
        let result = resolve_github_base_url(
            Some("https://github.com/Org/Repo/blob/master".into()),
            tmp.path(),
        )
        .unwrap();
        assert_eq!(result, "https://github.com/Org/Repo/blob/main/");
    }

    #[test]
    fn test_resolve_explicit_url_rejects_non_http_scheme() {
        let tmp = tempfile::TempDir::new().unwrap();
        let result =
            resolve_github_base_url(Some("file:///etc/passwd".into()), tmp.path());
        assert!(result.is_err());
        let msg = result.unwrap_err().to_string();
        assert!(
            msg.contains("must start with https://"),
            "Error should mention scheme requirement: {}",
            msg
        );
    }

    #[test]
    fn test_resolve_explicit_url_rejects_bare_string() {
        let tmp = tempfile::TempDir::new().unwrap();
        let result = resolve_github_base_url(Some("foobar".into()), tmp.path());
        assert!(result.is_err());
    }

    #[test]
    fn test_resolve_no_explicit_no_remote_fails() {
        let tmp = tempfile::TempDir::new().unwrap();
        let result = resolve_github_base_url(None, tmp.path());
        assert!(result.is_err());
        let msg = result.unwrap_err().to_string();
        assert!(
            msg.contains("--github-base-url"),
            "Error should mention --github-base-url: {}",
            msg
        );
    }
}
