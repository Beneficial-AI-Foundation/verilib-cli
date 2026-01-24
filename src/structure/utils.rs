//! General utility functions for verilib structure.

use anyhow::{Context, Result};
use serde_json::Value;
use std::collections::HashSet;
use std::io::{self, BufRead, Write};
use std::path::Path;
use std::process::Command;

/// Run an external command and return its output.
pub fn run_command(
    program: &str,
    args: &[&str],
    cwd: Option<&Path>,
) -> Result<std::process::Output> {
    let mut cmd = Command::new(program);
    cmd.args(args);

    if let Some(dir) = cwd {
        cmd.current_dir(dir);
    }

    let output = cmd
        .output()
        .context(format!("Failed to run {}", program))?;
    Ok(output)
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
pub fn parse_github_link(github_link: &str) -> Option<(String, u32)> {
    if github_link.is_empty() || !github_link.contains("/blob/main/") {
        return None;
    }

    let path_part = github_link.split("/blob/main/").nth(1)?;

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
