use anyhow::Result;
use clap::Parser;

mod cli;
mod commands;
mod constants;
mod download;
mod storage;

use cli::{Cli, Commands, ApiCommands};
use commands::{handle_api, handle_auth, handle_deploy, handle_init, handle_pull, handle_reclone, handle_status, handle_status_update};

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
        Commands::Deploy { url } => {
            handle_deploy(url, cli.debug).await?;
        }
        Commands::Pull => {
            handle_pull(cli.debug).await?;
        }
        Commands::StatusUpdate => {
            handle_status_update().await?;
        }
        Commands::Api(api_cmd) => {
            use commands::api::{ApiSubcommand, StatusFilter};
            use std::path::PathBuf;
            
            let subcommand = match api_cmd {
                ApiCommands::Get { file } => ApiSubcommand::Get {
                    file: PathBuf::from(file),
                },
                ApiCommands::List { filter } => {
                    let parsed_filter = filter.and_then(|f| match f.to_lowercase().as_str() {
                        "specified" => Some(StatusFilter::Specified),
                        "ignored" => Some(StatusFilter::Ignored),
                        "verified" => Some(StatusFilter::Verified),
                        _ => None,
                    });
                    ApiSubcommand::List {
                        filter: parsed_filter,
                    }
                },
                ApiCommands::Set { file, specified, ignored, verified } => ApiSubcommand::Set {
                    file: PathBuf::from(file),
                    specified,
                    ignored,
                    verified,
                },
                ApiCommands::Batch { input } => ApiSubcommand::Batch {
                    input: PathBuf::from(input),
                },
                ApiCommands::CreateFile { 
                    path, 
                    content, 
                    from_file,
                    disabled,
                    specified,
                    status_id,
                    statement_type,
                    code_name,
                } => ApiSubcommand::CreateFile {
                    path: PathBuf::from(path),
                    content,
                    from_file: from_file.map(PathBuf::from),
                    disabled,
                    specified,
                    status_id,
                    statement_type,
                    code_name,
                },
            };
            
            handle_api(subcommand, cli.json, cli.dry_run).await?;
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
