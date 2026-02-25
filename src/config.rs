use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

use crate::constants::DEFAULT_DOCKER_IMAGE;
use crate::executor::{CommandConfig, ExecutionMode};

static GLOBAL_CONFIG: OnceLock<ProjectConfig> = OnceLock::new();

/// Configuration for the repository stored in .verilib/config.json
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct RepoConfig {
    pub id: String,
    pub url: String,
    pub is_admin: bool,
}

/// Global configuration for the project stored in .verilib/config.json
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectConfig {
    /// Resolved at runtime from the CLI argument, not persisted to disk.
    #[serde(skip)]
    pub project_root: PathBuf,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub repo: Option<RepoConfig>,
    
    #[serde(rename = "structure-root", skip_serializing_if = "Option::is_none")]
    pub structure_root: Option<String>,
    
    #[serde(default, rename = "execution-mode")]
    pub execution_mode: ExecutionMode,
    
    #[serde(default = "default_docker_image", rename = "docker-image")]
    pub docker_image: String,
    
    #[serde(default, rename = "auto-validate-specs")]
    pub auto_validate_specs: bool,
}

fn default_docker_image() -> String {
    DEFAULT_DOCKER_IMAGE.to_string()
}

impl Default for ProjectConfig {
    fn default() -> Self {
        Self {
            project_root: PathBuf::new(),
            repo: None,
            structure_root: None,
            execution_mode: ExecutionMode::Local,
            docker_image: default_docker_image(),
            auto_validate_specs: false,
        }
    }
}

impl ProjectConfig {
    /// Initialize the global config from a project root. Safe to call multiple times.
    pub fn init(project_root: &Path) -> Result<()> {
        if GLOBAL_CONFIG.get().is_some() {
            return Ok(());
        }
        let mut config = Self::load(project_root)?;
        config.project_root = project_root.to_path_buf();
        let _ = GLOBAL_CONFIG.set(config);
        Ok(())
    }

    pub fn global() -> Option<&'static Self> {
        GLOBAL_CONFIG.get()
    }

    pub fn command_config(&self) -> CommandConfig {
        let mut mode = self.execution_mode.clone();
        let mut docker_image = self.docker_image.clone();

        if let Ok(env_mode) = std::env::var("VERILIB_EXECUTION_MODE") {
            if env_mode.eq_ignore_ascii_case("docker") {
                mode = ExecutionMode::Docker;
            } else if env_mode.eq_ignore_ascii_case("local") {
                mode = ExecutionMode::Local;
            }
        }
        if let Ok(env_img) = std::env::var("VERILIB_DOCKER_IMAGE") {
            docker_image = env_img;
        }

        CommandConfig {
            execution_mode: mode,
            docker_image,
        }
    }

    pub fn verilib_path(&self) -> PathBuf {
        self.project_root.join(".verilib")
    }

    pub fn stubs_path(&self) -> PathBuf {
        self.verilib_path().join("stubs.json")
    }

    pub fn atoms_path(&self) -> PathBuf {
        self.verilib_path().join("atoms.json")
    }

    pub fn certs_specify_dir(&self) -> PathBuf {
        self.verilib_path().join("certs").join("specs")
    }

    pub fn structure_root_path(&self) -> Result<PathBuf> {
        let root = self.structure_root.as_deref().ok_or_else(|| {
            anyhow::anyhow!(
                "No 'structure-root' in config.json. Run 'verilib-cli create' first."
            )
        })?;
        Ok(self.project_root.join(root))
    }

    pub fn load(project_root: &Path) -> Result<Self> {
        let config_path = project_root.join(".verilib").join("config.json");

        if !config_path.exists() {
            return Ok(Self::default());
        }

        let content = std::fs::read_to_string(&config_path)
            .context("Failed to read config.json")?;

        let config: Self = serde_json::from_str(&content)
            .context("Failed to parse config.json")?;

        Ok(config)
    }

    pub fn save(&self, project_root: &Path) -> Result<PathBuf> {
        let verilib_path = project_root.join(".verilib");
        std::fs::create_dir_all(&verilib_path)
            .context("Failed to create .verilib directory")?;

        let config_path = verilib_path.join("config.json");

        let content = serde_json::to_string_pretty(self)
            .context("Failed to serialize config")?;
            
        std::fs::write(&config_path, content)
            .context("Failed to write config.json")?;

        Ok(config_path)
    }
}
