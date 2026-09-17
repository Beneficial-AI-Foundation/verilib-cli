use clap::{Args, Parser, Subcommand, ValueEnum};
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
        #[arg(long, value_enum)]
        execution_mode: Option<Mode>,
        #[command(flatten)]
        wait: WaitOptions,
        /// Repository ID to bind (does not download source)

        #[arg(long)]
        id: Option<String>,
        /// API base URL (defaults to production)
        #[arg(long)]
        url: Option<String>,
    },
    /// Create or inspect a remote repository (not local structure generation)
    Repo {
        #[command(subcommand)]
        command: RepoCommands,
    },
    /// Wait for server-side upload/atomization using the existing logs API
    WaitForReady {
        #[command(flatten)]
        target: RepoTarget,
        #[command(flatten)]
        options: WaitOptions,
    },
    /// Reclone repository after checking for uncommitted changes
    Reclone,
    /// Push local repository / structure changes to the server
    Deploy {
        #[command(flatten)]
        wait: WaitOptions,
        /// Accept edited .verilib content without interactive prompts
        #[arg(long)]
        yes: bool,
        /// API base URL (defaults to config / VERILIB_BASE_URL / production)
        #[arg(long)]
        url: Option<String>,
    },
    /// Pull the latest repository structure from the server
    Pull {
        /// API base URL (defaults to config / VERILIB_BASE_URL / production)
        #[arg(long)]
        url: Option<String>,
    },
    /// Manage local .verilib metadata files
    Api {
        #[command(subcommand)]
        command: ApiCommands,
    },
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

        /// Only generate atoms.json, skip stubs enrichment (no create needed)
        #[arg(long)]
        atoms_only: bool,

        /// Use rust-analyzer instead of verus-analyzer for SCIP generation
        #[arg(long)]
        rust_analyzer: bool,
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

        /// Package to verify (for workspace projects, passed to probe-verus -p)
        #[arg(short, long)]
        package: Option<String>,

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

#[derive(Clone, Copy, ValueEnum)]
pub enum Mode {
    Local,
    Docker,
}

impl From<Mode> for crate::executor::ExecutionMode {
    fn from(mode: Mode) -> Self {
        match mode {
            Mode::Local => Self::Local,
            Mode::Docker => Self::Docker,
        }
    }
}

#[derive(Clone, Args)]
pub struct WaitOptions {
    /// Wait for remote upload and atomization (not proof verification)
    #[arg(long)]
    pub wait: bool,
    /// Overall wait/request deadline in seconds; timeout does not cancel server work
    #[arg(long, default_value_t = 600, value_parser = clap::value_parser!(u64).range(1..=86400))]
    pub timeout: u64,
    /// Seconds between status requests
    #[arg(long, default_value_t = 5, value_parser = clap::value_parser!(u64).range(1..=3600))]
    pub poll_interval: u64,
}

#[derive(Args)]
pub struct RepoTarget {
    /// Repository ID; defaults to .verilib/config.json
    #[arg(long)]
    pub id: Option<String>,
    #[arg(long)]
    pub url: Option<String>,
}

#[derive(Args)]
pub struct RepoCreate {
    /// HTTP(S) Git URL, optionally suffixed with @branch
    #[arg(long)]
    pub git_url: String,
    /// Required summary, at most 128 Unicode characters
    #[arg(long)]
    pub summary: String,
    /// Optional description, at most 512 Unicode characters
    #[arg(long)]
    pub description: Option<String>,
    #[arg(long, value_parser = clap::value_parser!(u32).range(1..=11))]
    pub language_id: u32,
    #[arg(long, value_parser = clap::value_parser!(u32).range(1..))]
    pub prooflanguage_id: u32,
    #[arg(long, value_parser = clap::value_parser!(u32).range(1..))]
    pub type_id: u32,
    #[arg(long, value_parser = clap::value_parser!(u32).range(1..))]
    pub verifierversion_id: Option<u32>,
    #[arg(long)]
    pub url: Option<String>,
    #[arg(long, value_enum, default_value = "local")]
    pub execution_mode: Mode,
    #[command(flatten)]
    pub options: WaitOptions,
}

#[derive(Subcommand)]
pub enum RepoCommands {
    Create(RepoCreate),
    Status {
        #[command(flatten)]
        target: RepoTarget,
        #[command(flatten)]
        options: WaitOptions,
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
