use anyhow::Result;
use clap::Parser;

mod cli;
mod commands;
mod constants;
mod download;
mod storage;
mod structure;

use cli::{Cli, Commands};
use commands::{
    handle_atomize, handle_auth, handle_create, handle_init,
    handle_reclone, handle_specify, handle_status,
    handle_verify,
};

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Auth => {
            handle_auth().await?;
        }
        Commands::Status => {
            handle_status().await?;
        }
        Commands::Init { id, url } => {
            handle_init(id, url, cli.debug).await?;
        }
        Commands::Reclone => {
            handle_reclone(cli.debug).await?;
        }
        // Structure commands (merged from verilib-structure)
        Commands::Create {
            project_root,
            root,
        } => {
            handle_create(project_root, root).await?;
        }
        Commands::Atomize {
            project_root,
            update_stubs,
            no_probe,
            check_only,
        } => {
            handle_atomize(project_root, update_stubs, no_probe, check_only).await?;
        }
        Commands::Specify {
            project_root,
            no_probe,
            check_only,
        } => {
            handle_specify(project_root, no_probe, check_only).await?;
        }
        Commands::Verify {
            project_root,
            verify_only_module,
            no_probe,
            check_only,
        } => {
            handle_verify(project_root, verify_only_module, no_probe, check_only).await?;
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_api_key_validation() {
        // Add tests for API key validation logic
    }
}
