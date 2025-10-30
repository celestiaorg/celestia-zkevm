use clap::{Parser, Subcommand, ValueEnum};

pub const VERSION: &str = "v0.1.0";

#[derive(Parser)]
#[command(name = "ev-prover", version = VERSION, about = "EVM Prover CLI", long_about = None)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Debug, Clone, ValueEnum)]
pub enum IsmType {
    /// Zero-knowledge execution ISM
    Zk,
    /// No-op ISM (for testing)
    Noop,
    /// Merkle root multisig ISM
    MerkleRootMultisig,
}

#[derive(Debug, Clone, ValueEnum)]
pub enum HookType {
    /// No-op hook (no verification)
    Noop,
    /// Merkle tree hook (for multisig ISMs)
    MerkleTree,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Initialize configuration and home directory
    Init {},

    /// Start the gRPC server
    Start {},

    /// Show the service version
    Version {},

    /// Create a new ISM
    CreateIsm {
        /// Type of ISM to create
        #[arg(long, value_enum, default_value = "zk")]
        ism_type: IsmType,

        /// Validators (required for multisig ISM, comma-separated ethereum addresses)
        #[arg(long, value_delimiter = ',', required_if_eq("ism_type", "merkle-root-multisig"))]
        validators: Option<Vec<String>>,

        /// Threshold (required for multisig ISM)
        #[arg(long, required_if_eq("ism_type", "merkle-root-multisig"))]
        threshold: Option<u32>,
    },

    /// Deploy full Hyperlane stack (ISM + Mailbox + Hooks + Warp Token)
    DeployStack {
        /// ISM ID to use (if not provided, will create a ZK ISM)
        #[arg(long)]
        ism_id: Option<String>,

        /// Local domain ID
        #[arg(long)]
        local_domain: u32,

        /// Use Merkle Tree hook instead of Noop hook
        #[arg(long, default_value = "false")]
        use_merkle_hook: bool,

        /// Token denom for warp token
        #[arg(long, default_value = "utia")]
        denom: String,
    },

    /// Create a mailbox
    CreateMailbox {
        /// ISM ID
        #[arg(long)]
        ism_id: String,

        /// Local domain ID
        #[arg(long)]
        local_domain: u32,

        /// Default hook ID (optional)
        #[arg(long)]
        default_hook: Option<String>,

        /// Required hook ID (optional)
        #[arg(long)]
        required_hook: Option<String>,
    },

    /// Create a hook
    CreateHook {
        /// Type of hook to create
        #[arg(long, value_enum)]
        hook_type: HookType,

        /// Mailbox ID (required for merkle tree hook)
        #[arg(long, required_if_eq("hook_type", "merkle-tree"))]
        mailbox_id: Option<String>,
    },

    /// Create a warp token
    CreateWarpToken {
        /// Mailbox ID
        #[arg(long)]
        mailbox_id: String,

        /// ISM ID
        #[arg(long)]
        ism_id: String,

        /// Token denom
        #[arg(long, default_value = "utia")]
        denom: String,
    },

    /// Enroll a remote router for warp token
    EnrollRouter {
        /// Token ID
        #[arg(long)]
        token_id: String,

        /// Remote domain ID
        #[arg(long)]
        remote_domain: u32,

        /// Remote contract address
        #[arg(long)]
        remote_contract: String,
    },

    /// Announce a validator for multisig ISM
    AnnounceValidator {
        /// Validator ethereum address
        #[arg(long)]
        validator: String,

        /// Storage location URL
        #[arg(long)]
        storage_location: String,

        /// Signature (hex string)
        #[arg(long)]
        signature: String,

        /// Mailbox ID
        #[arg(long)]
        mailbox_id: String,
    },
}
