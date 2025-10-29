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

    /// Create ZKISM
    Create {},

    /// Update
    Update { ism_id: String, token_id: String },

    /// Show the service version
    Version {},

    /// Query stored proofs from the gRPC server
    #[command(subcommand)]
    Query(QueryCommands),
}

#[derive(Subcommand)]
pub enum QueryCommands {
    /// Get the latest block proof
    #[command(
        about = "Get the latest block proof",
        after_help = "EXAMPLES:\n    ev-prover query latest-block\n    ev-prover query latest-block --server http://localhost:50051"
    )]
    LatestBlock {
        /// gRPC server address (default: http://127.0.0.1:50051)
        #[arg(long, default_value = "http://127.0.0.1:50051")]
        server: String,
    },

    /// Get a block proof by Celestia height
    #[command(
        about = "Get a block proof by Celestia height",
        after_help = "EXAMPLES:\n    ev-prover query block 12345\n    ev-prover query block 12345 --server http://localhost:50051"
    )]
    Block {
        /// Celestia block height
        height: u64,

        /// gRPC server address (default: http://127.0.0.1:50051)
        #[arg(long, default_value = "http://127.0.0.1:50051")]
        server: String,
    },

    /// Get block proofs in a height range
    #[command(
        about = "Get block proofs in a height range",
        after_help = "EXAMPLES:\n    ev-prover query block-range 100 200\n    ev-prover query block-range 100 200 --server http://localhost:50051"
    )]
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
    #[command(
        about = "Get the latest membership proof",
        after_help = "EXAMPLES:\n    ev-prover query latest-membership\n    ev-prover query latest-membership --server http://localhost:50051"
    )]
    LatestMembership {
        /// gRPC server address (default: http://127.0.0.1:50051)
        #[arg(long, default_value = "http://127.0.0.1:50051")]
        server: String,
    },

    /// Get a membership proof by height
    #[command(
        about = "Get a membership proof by height",
        after_help = "EXAMPLES:\n    ev-prover query membership 12345\n    ev-prover query membership 12345 --server http://localhost:50051"
    )]
    Membership {
        /// Block height
        height: u64,

        /// gRPC server address (default: http://127.0.0.1:50051)
        #[arg(long, default_value = "http://127.0.0.1:50051")]
        server: String,
    },

    /// Get aggregated range proofs
    #[command(
        about = "Get aggregated range proofs",
        after_help = "EXAMPLES:\n    ev-prover query range-proofs 100 200\n    ev-prover query range-proofs 100 200 --server http://localhost:50051"
    )]
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
