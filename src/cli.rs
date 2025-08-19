use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "verilib-cli")]
#[command(about = "A CLI tool for Verilib API operations")]
#[command(version = env!("CARGO_PKG_VERSION"))]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    Auth,
    Status,
}
