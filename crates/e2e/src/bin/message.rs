use alloy_provider::ProviderBuilder;
use e2e::{config::debug::TARGET_HEIGHT, utils::message::prove_messages};
use ev_state_queries::MockStateQueryProvider;
use sp1_sdk::{EnvProver, ProverClient};
use std::{env, str::FromStr, sync::Arc};
use url::Url;

#[tokio::main]
async fn main() {
    dotenvy::dotenv().ok();
    let reth_rpc_url = env::var("RETH_RPC_URL").unwrap();
    let client: Arc<EnvProver> = Arc::new(ProverClient::from_env());
    let evm_provider = ProviderBuilder::new().connect_http(Url::from_str(&reth_rpc_url).unwrap());
    let _proof = prove_messages(
        TARGET_HEIGHT,
        &evm_provider.clone(),
        &MockStateQueryProvider::new(evm_provider),
        client,
    )
    .await
    .unwrap();
    println!("Proof generated successfully!");
}
