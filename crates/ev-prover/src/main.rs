use clap::Parser;
use tracing_subscriber::EnvFilter;

use ev_prover::commands::{
    self,
    cli::{Cli, Commands},
};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize rustls crypto provider
    rustls::crypto::aws_lc_rs::default_provider()
        .install_default()
        .map_err(|_| anyhow::anyhow!("Failed to install default crypto provider"))?;

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

    match cli.command {
        Commands::Init {} => commands::command::init()?,
        Commands::Start {} => commands::command::start().await?,
        Commands::Version {} => commands::command::version(),
        Commands::CreateIsm {
            ism_type,
            validators,
            threshold,
        } => commands::command::create_ism(ism_type, validators, threshold).await?,
        Commands::DeployStack {
            ism_id,
            local_domain,
            use_merkle_hook,
            denom,
        } => commands::command::deploy_stack(ism_id, local_domain, use_merkle_hook, denom).await?,
        Commands::CreateMailbox {
            ism_id,
            local_domain,
            default_hook,
            required_hook,
        } => commands::command::create_mailbox(ism_id, local_domain, default_hook, required_hook).await?,
        Commands::CreateHook {
            hook_type,
            mailbox_id,
        } => commands::command::create_hook(hook_type, mailbox_id).await?,
        Commands::CreateWarpToken {
            mailbox_id,
            ism_id,
            denom,
        } => commands::command::create_warp_token(mailbox_id, ism_id, denom).await?,
        Commands::EnrollRouter {
            token_id,
            remote_domain,
            remote_contract,
        } => commands::command::enroll_router(token_id, remote_domain, remote_contract).await?,
        Commands::AnnounceValidator {
            validator,
            storage_location,
            signature,
            mailbox_id,
        } => commands::command::announce_validator(validator, storage_location, signature, mailbox_id).await?,
    }

    Ok(())
}
