//! Structure management utilities for verilib-cli.
//!
//! This module provides utilities for managing verification structure files,
//! including configuration, YAML frontmatter, certificates, and probe-verus integration.

pub mod certs;
pub mod frontmatter;
pub mod utils;

pub use crate::constants::{ATOMIZE_INTERMEDIATE_FILES, VERIFY_INTERMEDIATE_FILES};
pub use crate::executor::{CommandConfig, ExecutionMode, ExternalTool};
pub use certs::{create_cert, get_existing_certs};
pub use frontmatter::{parse as parse_frontmatter, write as write_frontmatter};
pub use utils::create_gitignore;
pub use utils::{cleanup_intermediate_files, display_menu, get_display_name, run_command};
