use clap::{Parser, Subcommand};
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "verilib-cli")]
#[command(about = "A CLI tool for Verilib API operations")]
#[command(version = env!("CARGO_PKG_VERSION"))]
pub struct Cli {
    /// Enable debug output
    #[arg(long, global = true)]
    pub debug: bool,

    /// Output in JSON format (for API commands)
    #[arg(long, global = true)]
    pub json: bool,

    /// Dry run mode - show changes without applying (for API commands)
    #[arg(long, global = true)]
    pub dry_run: bool,

    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Authenticate with API key (interactive prompt)
    Auth,
    /// Show current authentication status
    Status,
    /// Initialize project with repository tree
    Init {
        /// Repository ID to fetch
        #[arg(long)]
        id: Option<String>,
        /// API base URL (defaults to production)
        #[arg(long)]
        url: Option<String>,
    },
    /// Reclone repository after checking for uncommitted changes
    Reclone,
    // ===== Structure Commands (merged from verilib-structure) =====

    /// Initialize structure files from source analysis
    Create {
        /// Project root directory (default: current working directory)
        #[arg(default_value = ".")]
        project_root: PathBuf,

        /// Root directory for structure files (default: .verilib/structure)
        #[arg(long)]
        root: Option<PathBuf>,
    },

    /// Enrich structure files with metadata from SCIP atoms
    Atomize {
        /// Project root directory (default: current working directory)
        #[arg(default_value = ".")]
        project_root: PathBuf,

        /// Update .md structure files with code-name from atoms
        #[arg(short = 's', long)]
        update_stubs: bool,

        /// Skip running probe-verus atomize and read atoms.json from disk
        #[arg(short = 'n', long)]
        no_probe: bool,

        /// Check if .md stub files match enriched stubs.json without writing
        #[arg(short = 'c', long)]
        check_only: bool,
    },

    /// Check specification status and manage spec certs
    Specify {
        /// Project root directory (default: current working directory)
        #[arg(default_value = ".")]
        project_root: PathBuf,

        /// Skip running probe-verus specify and read specs.json from disk
        #[arg(short = 'n', long)]
        no_probe: bool,

        /// Check if all stubs with specs have certs, error if any are missing
        #[arg(short = 'c', long)]
        check_only: bool,
    },

    /// Run verification and update stubs with verification status
    #[command(name = "verify")]
    Verify {
        /// Project root directory (default: current working directory)
        #[arg(default_value = ".")]
        project_root: PathBuf,

        /// Only verify functions in this module
        #[arg(long)]
        verify_only_module: Option<String>,

        /// Skip running probe-verus verify and read proofs.json from disk
        #[arg(short = 'n', long)]
        no_probe: bool,

        /// Check if any stub has status "failure", error if any are found
        #[arg(short = 'c', long)]
        check_only: bool,
    },
}

#[derive(Subcommand)]
pub enum ApiCommands {
    /// Get metadata for a specific file
    Get {
        /// Path to the .meta.verilib file
        #[arg(long)]
        file: String,
    },
    /// List all files, optionally filtered by status
    List {
        /// Filter by status: specified, ignored, or verified
        #[arg(long)]
        filter: Option<String>,
    },
    /// Set metadata fields for a file
    Set {
        /// Path to the .meta.verilib file
        #[arg(long)]
        file: String,
        /// Set specified status
        #[arg(long)]
        specified: Option<bool>,
        /// Set ignored/disabled status
        #[arg(long)]
        ignored: Option<bool>,
        /// Set verified status (admin only)
        #[arg(long)]
        verified: Option<bool>,
    },
    /// Batch update multiple files from JSON input
    Batch {
        /// Path to JSON file with batch operations
        #[arg(long)]
        input: String,
    },
    /// Create a new file with content from string, file, or stdin
    CreateFile {
        /// Destination path for the new file
        #[arg(long)]
        path: String,
        /// Content string to write to the file
        #[arg(long, group = "source")]
        content: Option<String>,
        /// Path to a source file to read content from
        #[arg(long, group = "source")]
        from_file: Option<String>,
        /// Set disabled status
        #[arg(long, default_value_t = false)]
        disabled: bool,
        /// Set specified status
        #[arg(long, default_value_t = false)]
        specified: bool,
        /// Set status ID
        #[arg(long, default_value_t = 0)]
        status_id: u32,
        /// Set statement type
        #[arg(long)]
        statement_type: Option<String>,
        /// Set code name (defaults to parent directory name)
        #[arg(long)]
        code_name: Option<String>,
    },
}
