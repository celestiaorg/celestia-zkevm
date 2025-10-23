use std::sync::Arc;

use anyhow::Result;
use ev_types::v1::get_block_request::Identifier;
use ev_types::v1::store_service_client::StoreServiceClient;
use ev_types::v1::GetBlockRequest;
use storage::proofs::RocksDbProofStorage;
use tokio::net::TcpListener;
use tokio::sync::mpsc;
use tokio_stream::wrappers::TcpListenerStream;
use tonic::transport::Server;
use tonic_reflection::server::Builder as ReflectionBuilder;
use tracing::{debug, error};

use crate::config::config::{Config, APP_HOME};
use crate::proto::celestia::prover::v1::prover_server::ProverServer;
use crate::prover::programs::block::{AppContext, BlockExecProver};
use crate::prover::programs::combined::EvCombinedProver;
use crate::prover::programs::range::BlockRangeExecProver;
use crate::prover::service::ProverService;

pub async fn start_server(config: Config) -> Result<()> {
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
    debug!("Successfully got pubkey from evnode: {}", config_clone.pub_key);

    // Initialize RocksDB storage in the default data directory
    let storage_path = dirs::home_dir()
        .expect("cannot find home directory")
        .join(APP_HOME)
        .join("data")
        .join("proofs.db");

    let storage = Arc::new(RocksDbProofStorage::new(storage_path)?);
    /*let (block_tx, block_rx) = mpsc::channel(256);
    let (range_tx, _range_rx) = mpsc::channel(256);

    let batch_size = config_clone.batch_size;
    let concurrency = config_clone.concurrency;
    let queue_capacity = config_clone.queue_capacity;

    tokio::spawn({
        let block_prover = BlockExecProver::new(
            AppContext::from_config(config_clone)?,
            block_tx,
            storage.clone(),
            queue_capacity,
            concurrency,
        );
        async move {
            if let Err(e) = block_prover.run().await {
                error!("Block prover task failed: {e:?}");
            }
        }
    });

    tokio::spawn({
        let range_prover = BlockRangeExecProver::new(batch_size, block_rx, range_tx, storage.clone())?;
        async move {
            if let Err(e) = range_prover.run().await {
                error!("Block prover task failed: {e:?}");
            }
        }
    });*/

    tokio::spawn({
        let combined_prover = EvCombinedProver::new().unwrap();
        async move {
            if let Err(e) = combined_prover.run().await {
                error!("Combined prover task failed: {e:?}");
            }
        }
    });

    // Todo: Integrate message prover and supply trusted_root, trusted_height from block prover
    // First generate the block proof, then generate the message proof inside a joined service.
    // We have a service implementation for each prover that can run in isolation, but for our ZK ISM
    // we will want to send both proofs together in a single request.

    let prover_service = ProverService::new(storage)?;

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
