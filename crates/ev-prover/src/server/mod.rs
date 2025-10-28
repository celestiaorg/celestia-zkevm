use std::sync::Arc;

use alloy_primitives::Address;
use alloy_primitives::FixedBytes;
use alloy_provider::ProviderBuilder;
use anyhow::Result;
use celestia_grpc_client::types::ClientConfig;
use celestia_grpc_client::{CelestiaIsmClient, QueryIsmRequest};
use ev_state_queries::{DefaultProvider, MockStateQueryProvider};
use ev_types::v1::get_block_request::Identifier;
use ev_types::v1::store_service_client::StoreServiceClient;
use ev_types::v1::GetBlockRequest;
use reqwest::Url;
use std::str::FromStr;
use storage::hyperlane::message::HyperlaneMessageStore;
use storage::hyperlane::snapshot::HyperlaneSnapshotStore;
use storage::proofs::RocksDbProofStorage;
use tokio::net::TcpListener;
use tokio::sync::mpsc;
use tokio::sync::Mutex;
use tokio_stream::wrappers::TcpListenerStream;
use tonic::transport::Server;
use tonic_reflection::server::Builder as ReflectionBuilder;
use tracing::{debug, error};

use crate::config::config::{Config, APP_HOME};
use crate::proto::celestia::prover::v1::prover_server::ProverServer;
use crate::prover::programs::block::TrustedState;
use crate::prover::programs::message::HyperlaneMessageProver;
use crate::prover::service::ProverService;

#[cfg(not(feature = "combined"))]
use crate::prover::programs::{
    block::{AppContext, BlockExecProver},
    range::{BlockRangeExecProver, BlockRangeExecService},
};

#[cfg(feature = "combined")]
use crate::prover::programs::combined::AppContext as CombinedAppContext;

use crate::prover::programs::message::AppContext as MessageAppContext;

pub async fn start_server(config: Config) -> Result<()> {
    let listener = TcpListener::bind(config.grpc_address.clone()).await?;
    let sequencer_rpc_url = std::env::var("SEQUENCER_RPC_URL").expect("SEQUENCER_RPC_URL must be set");
    let reth_rpc_url = std::env::var("RETH_RPC_URL").expect("RETH_RPC_URL must be set");

    let descriptor_bytes = include_bytes!("../../src/proto/descriptor.bin");
    let reflection_service = ReflectionBuilder::configure()
        .register_encoded_file_descriptor_set(descriptor_bytes)
        .build()
        .unwrap();

    // TODO: Remove this config cloning when we can rely on the public key from config
    // https://github.com/evstack/ev-node/issues/2603
    let mut config_clone = config.clone();
    config_clone.pub_key = public_key(sequencer_rpc_url).await?;
    debug!("Successfully got pubkey from evnode: {}", config_clone.pub_key);

    let client_config = ClientConfig::from_env()?;
    let client = CelestiaIsmClient::new(client_config).await?;

    let trusted_state = get_trusted_state(&client).await?;
    debug!("Successfully got trusted state from ism: {}", trusted_state);

    // Initialize RocksDB storage in the default data directory
    let storage_path = dirs::home_dir()
        .expect("cannot find home directory")
        .join(APP_HOME)
        .join("data")
        .join("proofs.db");

    let storage = Arc::new(RocksDbProofStorage::new(storage_path)?);
    // shared resources
    let config = ClientConfig::from_env()?;
    let ism_client = Arc::new(CelestiaIsmClient::new(config).await?);
    #[allow(unused_mut)]
    let (tx_range, mut rx_range) = mpsc::channel(256);
    let is_proving_messages: Arc<Mutex<bool>> = Arc::new(Mutex::new(false));

    #[cfg(not(feature = "combined"))]
    {
        let batch_size = config_clone.batch_size;
        let concurrency = config_clone.concurrency;
        let queue_capacity = config_clone.queue_capacity;
        let (tx_block, rx_block) = mpsc::channel(256);
        tokio::spawn({
            let block_prover = BlockExecProver::new(
                AppContext::new(config_clone, trusted_state)?,
                tx_block,
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

        let prover = Arc::new(BlockRangeExecProver::new()?);
        let service =
            BlockRangeExecService::new(client, prover, storage.clone(), rx_block, tx_range, batch_size, 16).await?;

        tokio::spawn(async move {
            if let Err(e) = service.run().await {
                error!("Block prover task failed: {e:?}");
            }
        });
    }

    #[cfg(feature = "combined")]
    {
        tokio::spawn({
            use crate::prover::programs::combined::EvCombinedProver;

            let ism_client = ism_client.clone();
            let combined_prover = EvCombinedProver::new(CombinedAppContext::default(), tx_range).unwrap();
            let is_proving_messages = is_proving_messages.clone();
            async move {
                if let Err(e) = combined_prover.run(ism_client, is_proving_messages).await {
                    error!("Combined prover task failed: {e:?}");
                }
            }
        });
    }

    // Always spawn message prover
    tokio::spawn({
        let message_storage_path = dirs::home_dir()
            .expect("cannot find home directory")
            .join(APP_HOME)
            .join("data")
            .join("messages.db");
        let snapshot_storage_path = dirs::home_dir()
            .expect("cannot find home directory")
            .join(APP_HOME)
            .join("data")
            .join("snapshots.db");
        let hyperlane_message_store = Arc::new(HyperlaneMessageStore::new(message_storage_path).unwrap());
        let hyperlane_snapshot_store = Arc::new(HyperlaneSnapshotStore::new(snapshot_storage_path, None).unwrap());

        let ctx = MessageAppContext {
            evm_rpc: reth_rpc_url.clone(),
            evm_ws: "ws://127.0.0.1:8546".to_string(),
            mailbox_address: Address::from_str("0xb1c938f5ba4b3593377f399e12175e8db0c787ff").unwrap(),
            merkle_tree_address: Address::from_str("0xfcb1d485ef46344029d9e8a7925925e146b3430e").unwrap(),
        };

        let evm_provider: DefaultProvider = ProviderBuilder::new().connect_http(Url::from_str(&reth_rpc_url).unwrap());

        let message_prover = HyperlaneMessageProver::new(
            ctx,
            hyperlane_message_store,
            hyperlane_snapshot_store,
            storage.clone(),
            Arc::new(MockStateQueryProvider::new(evm_provider)),
        )
        .unwrap();

        let is_proving_messages = is_proving_messages.clone();

        async move {
            let ism_client = ism_client.clone();
            if let Err(e) = message_prover.run(rx_range, ism_client, is_proving_messages).await {
                error!("Message prover task failed: {e:?}");
            }
        }
    });

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
pub async fn public_key(sequencer_rpc_url: String) -> Result<String> {
    let mut sequencer_client = StoreServiceClient::connect(sequencer_rpc_url).await?;
    let block_req = GetBlockRequest {
        identifier: Some(Identifier::Height(1)),
    };
    let resp = sequencer_client.get_block(block_req).await?;
    let pub_key = resp.into_inner().block.unwrap().header.unwrap().signer.unwrap().pub_key;
    Ok(hex::encode(&pub_key[4..]))
}

pub async fn get_trusted_state(client: &CelestiaIsmClient) -> Result<TrustedState> {
    let resp = client
        .ism(QueryIsmRequest {
            id: client.ism_id().to_string(),
        })
        .await?;

    let ism = resp.ism.unwrap();

    Ok(TrustedState::new(ism.height, FixedBytes::from_slice(&ism.state_root)))
}
