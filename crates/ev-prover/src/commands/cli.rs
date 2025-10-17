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
    Start {},

    /// Show the service version
    Version {},

    /// Query stored proofs from the gRPC server
    #[command(subcommand)]
    Query(QueryCommands),
}

#[derive(Subcommand)]
pub enum QueryCommands {
    /// Get the latest block proof
    LatestBlock {
        /// gRPC server address (default: http://127.0.0.1:50051)
        #[arg(long, default_value = "http://127.0.0.1:50051")]
        server: String,
    },

    /// Get a block proof by Celestia height
    Block {
        /// Celestia block height
        height: u64,

        /// gRPC server address (default: http://127.0.0.1:50051)
        #[arg(long, default_value = "http://127.0.0.1:50051")]
        server: String,
    },

    /// Get block proofs in a height range
    BlockRange {
        /// Start height (inclusive)
        start_height: u64,

        /// End height (inclusive)
        end_height: u64,

        /// gRPC server address (default: http://127.0.0.1:50051)
        #[arg(long, default_value = "http://127.0.0.1:50051")]
        server: String,
    },

    /// Get the latest membership proof
    LatestMembership {
        /// gRPC server address (default: http://127.0.0.1:50051)
        #[arg(long, default_value = "http://127.0.0.1:50051")]
        server: String,
    },

    /// Get a membership proof by height
    Membership {
        /// Block height
        height: u64,

        /// gRPC server address (default: http://127.0.0.1:50051)
        #[arg(long, default_value = "http://127.0.0.1:50051")]
        server: String,
    },

    /// Get aggregated range proofs
    RangeProofs {
        /// Start height (inclusive)
        start_height: u64,

        /// End height (inclusive)
        end_height: u64,

        /// gRPC server address (default: http://127.0.0.1:50051)
        #[arg(long, default_value = "http://127.0.0.1:50051")]
        server: String,
    },
}
