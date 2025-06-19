use clap::Parser;

mod commands;
mod config;
mod grpc;
mod proto;
mod prover;

use commands::cli::{CLI, Commands};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = CLI::parse();

    match cli.command {
        Commands::Init {} => commands::command::init()?,
        Commands::Start {} => commands::command::start().await?,
        Commands::Version {} => commands::command::version(),
    }

    Ok(())
}
