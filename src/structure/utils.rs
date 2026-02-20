//! General utility functions for verilib structure.

use super::executor::{self, CommandConfig};
use anyhow::Result;
use serde_json::Value;
use std::collections::HashSet;
use std::io::{self, BufRead, Write};
use std::path::Path;

/// Run an external command and return its output.
pub fn run_command(
    program: &str,
    args: &[&str],
    cwd: Option<&Path>,
    config: &CommandConfig,
) -> Result<std::process::Output> {
    executor::run_command(program, args, cwd, config)
}

/// Display a multiple choice menu and get user selections.
pub fn display_menu<F>(items: &[(String, Value)], format_item: F) -> Result<Vec<usize>>
where
    F: Fn(usize, &str, &Value) -> String,
{
    println!();
    println!("{}", "=".repeat(60));
    println!("Functions with specs but no certification:");
    println!("{}", "=".repeat(60));
    println!();

    for (i, (name, info)) in items.iter().enumerate() {
        println!("{}", format_item(i + 1, name, info));
        println!();
    }

    println!("{}", "=".repeat(60));
    println!();
    println!("Enter selection:");
    println!("  - Individual numbers: 1, 3, 5");
    println!("  - Ranges: 1-5");
    println!("  - 'all' to select all");
    println!("  - 'none' or empty to skip");
    println!();

    print!("Your selection: ");
    io::stdout().flush()?;

    let mut input = String::new();
    io::stdin().lock().read_line(&mut input)?;
    let input = input.trim().to_lowercase();

    if input.is_empty() || input == "none" {
        return Ok(vec![]);
    }

    if input == "all" {
        return Ok((0..items.len()).collect());
    }

    let mut selected = HashSet::new();
    for part in input.replace(',', " ").split_whitespace() {
        if part.contains('-') {
            let parts: Vec<&str> = part.splitn(2, '-').collect();
            if parts.len() == 2 {
                if let (Ok(start), Ok(end)) = (parts[0].parse::<usize>(), parts[1].parse::<usize>())
                {
                    for i in start..=end {
                        if i >= 1 && i <= items.len() {
                            selected.insert(i - 1);
                        }
                    }
                } else {
                    eprintln!("Warning: Invalid range '{}', skipping", part);
                }
            }
        } else if let Ok(idx) = part.parse::<usize>() {
            if idx >= 1 && idx <= items.len() {
                selected.insert(idx - 1);
            } else {
                eprintln!("Warning: {} out of range, skipping", idx);
            }
        } else {
            eprintln!("Warning: Invalid number '{}', skipping", part);
        }
    }

    let mut result: Vec<usize> = selected.into_iter().collect();
    result.sort();
    Ok(result)
}

/// Extract code path and line number from a GitHub link.
///
/// Accepts any `/blob/<branch>/` pattern (e.g. `main`, `master`, `develop`).
/// Branch names containing `/` (e.g. `feature/foo`) are not supported: the
/// first `/` after `/blob/` is taken as the branch/path delimiter.
pub fn parse_github_link(github_link: &str) -> Option<(String, u32)> {
    if github_link.is_empty() {
        return None;
    }

    let blob_idx = github_link.find("/blob/")?;
    let after_blob = &github_link[blob_idx + "/blob/".len()..];
    // Skip the branch name segment to get the file path
    let path_part = after_blob.split_once('/')?.1;

    if let Some((code_path, line_str)) = path_part.rsplit_once("#L") {
        let line_number: u32 = line_str.parse().ok()?;
        Some((code_path.to_string(), line_number))
    } else {
        Some((path_part.to_string(), 0))
    }
}

/// Get a display name from a full identifier (e.g., extract "func" from "probe:crate/mod#func()").
pub fn get_display_name(name: &str) -> String {
    if let Some(pos) = name.rfind('#') {
        name[pos + 1..].trim_end_matches("()").to_string()
    } else {
        name.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_github_link_main_branch() {
        assert_eq!(
            parse_github_link("https://github.com/Org/Repo/blob/main/src/lib.rs#L42"),
            Some(("src/lib.rs".to_string(), 42))
        );
    }

    #[test]
    fn test_parse_github_link_master_branch() {
        assert_eq!(
            parse_github_link("https://github.com/Org/Repo/blob/master/src/lib.rs#L10"),
            Some(("src/lib.rs".to_string(), 10))
        );
    }

    #[test]
    fn test_parse_github_link_develop_branch() {
        assert_eq!(
            parse_github_link("https://github.com/Org/Repo/blob/develop/src/foo.rs#L1"),
            Some(("src/foo.rs".to_string(), 1))
        );
    }

    #[test]
    fn test_parse_github_link_no_line_number() {
        assert_eq!(
            parse_github_link("https://github.com/Org/Repo/blob/main/src/lib.rs"),
            Some(("src/lib.rs".to_string(), 0))
        );
    }

    #[test]
    fn test_parse_github_link_empty() {
        assert_eq!(parse_github_link(""), None);
    }

    #[test]
    fn test_parse_github_link_no_blob() {
        assert_eq!(
            parse_github_link("https://github.com/Org/Repo/tree/main/src"),
            None
        );
    }

    #[test]
    fn test_parse_github_link_blob_without_path() {
        // /blob/<branch> with no trailing path -> split_once('/') returns None
        assert_eq!(
            parse_github_link("https://github.com/Org/Repo/blob/main"),
            None
        );
    }

    #[test]
    fn test_parse_github_link_nested_path() {
        assert_eq!(
            parse_github_link(
                "https://github.com/Org/Repo/blob/main/src/deeply/nested/file.rs#L99"
            ),
            Some(("src/deeply/nested/file.rs".to_string(), 99))
        );
    }
}
