use alloy_provider::ProviderBuilder;
use e2e::{
    config::{EV_RPC, TARGET_HEIGHT},
    prover::message::prove_messages,
};
use ev_state_queries::MockStateQueryProvider;
use std::str::FromStr;
use url::Url;

#[tokio::main]
async fn main() {
    dotenvy::dotenv().ok();
    let evm_provider = ProviderBuilder::new().connect_http(Url::from_str(EV_RPC).unwrap());
    let _proof = prove_messages(
        TARGET_HEIGHT,
        &evm_provider.clone(),
        &MockStateQueryProvider::new(evm_provider),
    )
    .await
    .unwrap();
    println!("Proof generated successfully!");
}
