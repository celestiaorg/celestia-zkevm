use clap::Parser;
use ev_prover::commands::{
    self,
    cli::{Cli, Commands},
};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Init {} => commands::command::init()?,
        Commands::Start {} => commands::command::start().await?,
        Commands::Version {} => commands::command::version(),
    }

    Ok(())
}
