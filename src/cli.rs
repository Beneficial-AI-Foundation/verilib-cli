use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "verilib-cli")]
#[command(about = "A CLI tool for Verilib API operations")]
#[command(version = env!("CARGO_PKG_VERSION"))]
pub struct Cli {
    /// Enable debug output
    #[arg(long, global = true)]
    pub debug: bool,
    
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
        repo_id: String,
        /// API base URL (defaults to production)
        #[arg(long)]
        url: Option<String>,
    },
    /// Reclone repository after checking for uncommitted changes
    Reclone,
}
