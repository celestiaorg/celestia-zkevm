use clap::Parser;
use tracing_subscriber::EnvFilter;

use ev_prover::command::{create_zkism, init, query, start, unsafe_reset_db, update_ism, version, Cli, Commands};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Filter out sp1 logs by default, show debug level for ev-prover
    // This can be changed to info for operational logging.
    let mut filter = EnvFilter::new("sp1_core=warn,sp1_runtime=warn,sp1_sdk=warn,sp1_vm=warn");
    if let Ok(env_filter) = std::env::var("RUST_LOG") {
        if let Ok(parsed) = env_filter.parse() {
            filter = filter.add_directive(parsed);
        }
    }
    tracing_subscriber::fmt().with_env_filter(filter).init();

    let cli = Cli::parse();
    dotenvy::dotenv().ok();

    rustls::crypto::aws_lc_rs::default_provider()
        .install_default()
        .expect("Failed to set default crypto provider");
    dotenvy::dotenv().ok();

    match cli.command {
        Commands::Init {} => init()?,
        Commands::Start {} => start().await?,
        Commands::Create {} => create_zkism().await?,
        Commands::Update { ism_id, token_id } => update_ism(ism_id, token_id).await?,
        Commands::Version {} => version(),
        Commands::Query(query_cmd) => query(query_cmd).await?,
        Commands::UnsafeResetDb {} => unsafe_reset_db()?,
    }

    Ok(())
}
