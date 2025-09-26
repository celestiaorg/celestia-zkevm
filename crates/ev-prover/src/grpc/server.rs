use anyhow::Result;
use ev_types::v1::get_block_request::Identifier;
use ev_types::v1::store_service_client::StoreServiceClient;
use ev_types::v1::GetBlockRequest;
use tokio::net::TcpListener;
use tokio_stream::wrappers::TcpListenerStream;
use tonic::transport::Server;
use tonic_reflection::server::Builder as ReflectionBuilder;

use crate::config::config::Config;
use crate::proto::celestia::prover::v1::prover_server::ProverServer;
use crate::prover::programs::block::{AppContext, BlockExecProver};
use crate::prover::service::ProverService;

pub async fn create_grpc_server(config: Config, from_height: Option<u64>) -> Result<()> {
    let listener = TcpListener::bind(config.grpc_address.clone()).await?;

    let descriptor_bytes = include_bytes!("../../src/proto/descriptor.bin");
    let reflection_service = ReflectionBuilder::configure()
        .register_encoded_file_descriptor_set(descriptor_bytes)
        .build()
        .unwrap();

    // TODO: Remove this config cloning when we can rely on the public key from config
    // https://github.com/evstack/ev-node/issues/2603
    let mut config_clone = config.clone();
    config_clone.pub_key = public_key().await?;
    println!("Successfully got pubkey from evnode: {}", config_clone.pub_key);

    tokio::spawn({
        let block_prover = BlockExecProver::new(AppContext::from_config(config_clone)?)?;
        async move {
            if let Err(e) = block_prover.run(from_height).await {
                eprintln!("Block prover task failed: {e:?}");
            }
        }
    });

    // Todo: Integrate message prover and supply trusted_root, trusted_height from block prover
    // First generate the block proof, then generate the message proof inside a joined service.
    // We have a service implementation for each prover that can run in isolation, but for our ZK ISM
    // we will want to send both proofs together in a single request.

    let prover_service = ProverService::new(config)?;

    Server::builder()
        .add_service(reflection_service)
        .add_service(ProverServer::new(prover_service))
        .serve_with_incoming(TcpListenerStream::new(listener))
        .await?;

    Ok(())
}

// TODO: Use from config file when we can have a reproducible key in docker-compose.
// For now query the pubkey on startup from evnode.
// https://github.com/evstack/ev-node/issues/2603
pub async fn public_key() -> Result<String> {
    let mut sequencer_client = StoreServiceClient::connect("http://127.0.0.1:7331").await?;
    let block_req = GetBlockRequest {
        identifier: Some(Identifier::Height(1)),
    };

    let resp = sequencer_client.get_block(block_req).await?;
    let pub_key = resp.into_inner().block.unwrap().header.unwrap().signer.unwrap().pub_key;

    Ok(hex::encode(&pub_key[4..]))
}
