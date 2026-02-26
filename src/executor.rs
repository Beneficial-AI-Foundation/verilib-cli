use anyhow::{bail, Context, Result};
use crate::constants::{DEFAULT_DOCKER_IMAGE, PROBE_VERUS_MIN_VERSION, PROBE_VERUS_TESTED_MAX_VERSION};
use semver::{Version, VersionReq};
use serde::{Deserialize, Serialize};
use std::path::Path;
use std::process::{Command, Output};

pub const PROBE_REPO_URL: &str = "https://github.com/Beneficial-AI-Foundation/probe-verus";


#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExternalTool {
    /// The `probe-verus` CLI tool.
    Probe,
}

impl ExternalTool {
    pub fn binary_name(&self) -> &str {
        match self {
            ExternalTool::Probe => "probe-verus",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ExecutionMode {
    Local,
    Docker,
}

impl Default for ExecutionMode {
    fn default() -> Self {
        Self::Local
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommandConfig {
    #[serde(default)]
    pub execution_mode: ExecutionMode,
    #[serde(default = "default_docker_image")]
    pub docker_image: String,
}

fn default_docker_image() -> String {
    DEFAULT_DOCKER_IMAGE.to_string()
}

impl Default for CommandConfig {
    fn default() -> Self {
        Self {
            execution_mode: ExecutionMode::Local,
            docker_image: default_docker_image(),
        }
    }
}

pub fn check_tool_available(tool: &ExternalTool, config: &CommandConfig) -> Result<()> {
    match config.execution_mode {
        ExecutionMode::Docker => {
            if which::which("docker").is_err() {
                eprintln!("Error: Docker is not installed or not in PATH.");
                eprintln!("Docker is required for execution mode 'docker'.");
                eprintln!("Please install Docker: https://docs.docker.com/get-docker/");
                bail!("docker not installed");
            }
        }
        ExecutionMode::Local => match tool {
            ExternalTool::Probe => {
                if which::which("probe-verus").is_err() {
                    eprintln!("Error: probe-verus is not installed.");
                    eprintln!(
                        "Please visit {} for installation instructions.",
                        PROBE_REPO_URL
                    );
                    eprintln!();
                    eprintln!("Quick install:");
                    eprintln!("  git clone {}", PROBE_REPO_URL);
                    eprintln!("  cd probe-verus");
                    eprintln!("  cargo install --path .");
                    bail!("probe-verus not installed");
                }
                check_probe_verus_version()?;
            }
        },
    }
    Ok(())
}

fn check_probe_verus_version() -> Result<()> {
    let output = Command::new("probe-verus")
        .arg("--version")
        .output()
        .context("Failed to run 'probe-verus --version'")?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    let raw = format!("{}{}", stdout, stderr);

    let version = raw
        .split_whitespace()
        .find_map(|token| Version::parse(token).ok())
        .ok_or_else(|| {
            anyhow::anyhow!(
                "Could not parse a semver version from 'probe-verus --version' output: {}",
                raw.trim()
            )
        })?;

    let min_req = VersionReq::parse(PROBE_VERUS_MIN_VERSION)
        .expect("PROBE_VERUS_MIN_VERSION is a valid semver requirement");
    let tested_max_req = VersionReq::parse(PROBE_VERUS_TESTED_MAX_VERSION)
        .expect("PROBE_VERUS_TESTED_MAX_VERSION is a valid semver requirement");

    if !min_req.matches(&version) {
        eprintln!("Error: probe-verus {} is too old for this version of verilib-cli.", version);
        eprintln!("  Minimum required: {}", PROBE_VERUS_MIN_VERSION);
        eprintln!("  Installed:        {}", version);
        eprintln!();
        eprintln!("Please update probe-verus:");
        eprintln!("  git clone {}", PROBE_REPO_URL);
        eprintln!("  cd probe-verus");
        eprintln!("  cargo install --path .");
        bail!("probe-verus {} is below the minimum required version ({})", version, PROBE_VERUS_MIN_VERSION);
    }

    if !tested_max_req.matches(&version) {
        eprintln!(
            "Warning: probe-verus {} has not been tested with this version of verilib-cli (tested up to {}).",
            version, PROBE_VERUS_TESTED_MAX_VERSION
        );
        eprintln!("  It may work, but you could encounter unexpected behaviour.");
        eprintln!("  Consider filing an issue at {} if you hit problems.", PROBE_REPO_URL);
    }

    Ok(())
}

pub fn run_command(
    tool: &ExternalTool,
    args: &[&str],
    cwd: Option<&Path>,
    config: &CommandConfig,
) -> Result<Output> {
    check_tool_available(tool, config)?;
    let program = tool.binary_name();
    match config.execution_mode {
        ExecutionMode::Local => run_local(program, args, cwd),
        ExecutionMode::Docker => run_docker(program, args, cwd, &config.docker_image),
    }
}

fn run_local(program: &str, args: &[&str], cwd: Option<&Path>) -> Result<Output> {
    let mut cmd = Command::new(program);
    cmd.args(args);

    if let Some(dir) = cwd {
        cmd.current_dir(dir);
    }

    let output = cmd
        .output()
        .context(format!("Failed to run local command: {}", program))?;
    Ok(output)
}

fn ensure_image_pulled(image: &str) -> Result<()> {
    let status = Command::new("docker")
        .args(&["image", "inspect", image])
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status();

    if let Ok(status) = status {
        if status.success() {
            return Ok(());
        }
    }

    println!("Docker image {} not found locally. Pulling...", image);

    let status = Command::new("docker")
        .args(&["pull", "--platform", "linux/amd64", image])
        .status()
        .context(format!("Failed to pull docker image {}", image))?;

    if !status.success() {
        anyhow::bail!("Failed to pull docker image {}", image);
    }

    Ok(())
}

fn run_docker(
    program: &str,
    args: &[&str],
    cwd: Option<&Path>,
    image: &str,
) -> Result<Output> {
    ensure_image_pulled(image)?;

    let host_cwd = cwd
        .map(|p| p.to_path_buf())
        .unwrap_or_else(|| std::env::current_dir().unwrap_or_else(|_| Path::new(".").to_path_buf()));

    let host_cwd_str = host_cwd.to_string_lossy();

    #[cfg(unix)]
    let user_arg = {
        let uid = users::get_current_uid();
        let gid = users::get_current_gid();
        format!("{}:{}", uid, gid)
    };

    #[cfg(not(unix))]
    let user_arg = "1000:1000".to_string();

    let mut docker_args = vec![
        "run",
        "--rm",
        "--platform", "linux/amd64",
        "--entrypoint", program,
        "-u", &user_arg,
        "-v",
    ];

    let mount_arg = format!("{}:/workspace:rw", host_cwd_str);
    docker_args.push(&mount_arg);

    docker_args.extend_from_slice(&[
        "--tmpfs", "/tmp",
        "--tmpfs", "/home/tooluser/.cache",
        "--security-opt=no-new-privileges",
        "-w", "/workspace",
        image,
    ]);

    docker_args.extend_from_slice(args);

    let output = Command::new("docker")
        .args(&docker_args)
        .output()
        .context(format!("Failed to run docker command with image {}", image))?;

    Ok(output)
}
