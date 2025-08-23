use anyhow::Result;
use clap::Parser;

mod auth;
mod cli;
mod constants;
mod init;
mod keyring_utils;
mod reclone;
mod status;

use auth::handle_auth;
use cli::{Cli, Commands};
use init::handle_init;
use reclone::handle_reclone;
use status::handle_status;

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
        Commands::Init { repo_id, base_url } => {
            handle_init(repo_id, base_url).await?;
        }
        Commands::Reclone { base_url } => {
            handle_reclone(base_url, cli.debug).await?;
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
