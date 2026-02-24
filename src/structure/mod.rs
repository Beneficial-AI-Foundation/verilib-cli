//! Structure management utilities for verilib-cli.
//!
//! This module provides utilities for managing verification structure files,
//! including configuration, YAML frontmatter, certificates, and probe-verus integration.

pub mod certs;
pub mod config;
pub mod executor;
pub mod frontmatter;
pub mod probe;
pub mod utils;

pub use certs::{create_cert, get_existing_certs};
pub use config::{create_gitignore, ConfigPaths, StructureConfig};
pub use executor::{CommandConfig, ExecutionMode};
pub use frontmatter::{parse as parse_frontmatter, write as write_frontmatter};
pub use probe::{
    cleanup_intermediate_files, require_probe_installed, ATOMIZE_INTERMEDIATE_FILES,
    VERIFY_INTERMEDIATE_FILES,
};
pub use utils::{display_menu, get_display_name, run_command};
