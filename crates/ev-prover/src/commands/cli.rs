use clap::{Parser, Subcommand};

pub const VERSION: &str = "v0.1.0";

#[derive(Parser)]
#[command(name = "ev-prover", version = VERSION, about = "EVM Prover CLI", long_about = None)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Initialize configuration and home directory
    Init {},

    /// Start the gRPC server
    Start {
        /// Optionally backfill blocks starting from this Celestia height before subscribing to live events
        #[arg(long = "from-height")]
        from_height: Option<u64>,
    },

    /// Show the service version
    Version {},
}
