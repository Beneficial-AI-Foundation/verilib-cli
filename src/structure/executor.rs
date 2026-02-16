use anyhow::{Context, Result};
use crate::constants::DEFAULT_DOCKER_IMAGE;
use serde::{Deserialize, Serialize};
use std::path::Path;
use std::process::{Command, Output};

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

pub fn run_command(
    program: &str,
    args: &[&str],
    cwd: Option<&Path>,
    config: &CommandConfig,
) -> Result<Output> {
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
