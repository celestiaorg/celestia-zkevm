use alloy_primitives::{FixedBytes, hex::FromHex};
use alloy_provider::ProviderBuilder;
use celestia_grpc_client::{
    MsgProcessMessage, MsgSubmitMessages, MsgUpdateZkExecutionIsm, QueryIsmRequest, client::CelestiaIsmClient,
    types::ClientConfig,
};
use e2e::config::debug::EV_RPC;
use e2e::config::e2e::ISM_ID;
use e2e::prover::block::prove_blocks;
use e2e::prover::message::prove_messages;
use ev_state_queries::MockStateQueryProvider;
use ev_types::v1::{GetMetadataRequest, store_service_client::StoreServiceClient};
use ev_zkevm_types::{hyperlane::encode_hyperlane_message, programs::block::BlockRangeExecOutput};
use sp1_sdk::{EnvProver, ProverClient};
use std::{str::FromStr, sync::Arc, time::Duration};
use storage::hyperlane::snapshot::HyperlaneSnapshotStore;
use tokio::time::sleep;
use tracing::{info, warn};
use tracing_subscriber::EnvFilter;
use url::Url;

// prove once every 10 blocks
const PROVER_INTERVAL: u64 = 2;
const EXPECTED_LAG: u64 = 5;

#[tokio::main]
async fn main() {
    rustls::crypto::aws_lc_rs::default_provider()
        .install_default()
        .expect("Failed to set default crypto provider");
    dotenvy::dotenv().ok();

    let mut filter = EnvFilter::new("sp1_core=warn,sp1_runtime=warn,sp1_sdk=warn,sp1_vm=warn");
    if let Ok(env_filter) = std::env::var("RUST_LOG") {
        if let Ok(parsed) = env_filter.parse() {
            filter = filter.add_directive(parsed);
        }
    }
    tracing_subscriber::fmt().with_env_filter(filter).init();

    // instantiate ISM client for submitting payloads and querying state
    let config = ClientConfig::from_env().expect("failed to create celestia client config");
    let ism_client = CelestiaIsmClient::new(config).await.unwrap();
    let client: Arc<EnvProver> = Arc::new(ProverClient::from_env());
    let mut snapshot_index = 0;

    let snapshot_storage_path = dirs::home_dir()
        .expect("cannot find home directory")
        .join(".ev-prover")
        .join("data")
        .join("snapshots.db");
    let hyperlane_snapshot_store = Arc::new(HyperlaneSnapshotStore::new(snapshot_storage_path).unwrap());
    hyperlane_snapshot_store.reset_db().unwrap();

    // This variable is a trick to account for empty blocks that were proven,
    // by not relying on the fixed trusted_height in the ZKISM but instead remembering which height we are actually at.
    let mut prover_height: Option<u64> = None;

    // wait until all hyperlane deployments are done
    loop {
        match query_ism(&ism_client).await {
            Ok(_) => {
                break;
            }
            Err(_) => {
                warn!("ISM not yet deployed, waiting for 10 seconds");
                sleep(Duration::from_secs(10)).await;
                continue;
            }
        }
    }

    loop {
        // get trustd state from ISM
        let (trusted_root_hex, trusted_height) = query_ism(&ism_client).await.unwrap();
        let latest_celestia_height = celestia_height("http://localhost:26657").unwrap();

        let maybe_inclusion_height = match inclusion_height(trusted_height).await {
            Ok(height) => height,
            Err(_) => {
                warn!("Trusted Block not yet included, waiting for 30 seconds");
                sleep(Duration::from_secs(30)).await;
                continue;
            }
        };
        if latest_celestia_height < maybe_inclusion_height + PROVER_INTERVAL {
            continue;
        }

        // workaround to ensure we don't prove empty blocks again
        // we should be able to set the start height to the last block that has
        // the same state root as the ism
        let celestia_start_height = {
            if let Some(prover_height) = prover_height {
                prover_height
            } else {
                inclusion_height(trusted_height).await.unwrap() + 1
            }
        };

        // prove at most PROVER_INTERVAL blocks at a time
        let num_blocks = (latest_celestia_height - celestia_start_height).min(PROVER_INTERVAL);

        info!(
            "ISM at height {} Proving block {} up to {}",
            trusted_height,
            celestia_start_height,
            celestia_start_height + num_blocks
        );

        // warn the user if lag is too high
        let lag = latest_celestia_height - celestia_start_height;
        if lag > EXPECTED_LAG {
            warn!("Lagging behind by {} blocks", lag);
        } else {
            info!("Lagging behind by {} blocks", lag);
        }

        let block_proof = prove_blocks(
            celestia_start_height,
            trusted_height,
            num_blocks,
            &mut FixedBytes::from_hex(alloy::hex::encode(trusted_root_hex)).unwrap(),
            client.clone(),
        )
        .await
        .expect("Failed to prove blocks");

        let block_proof_msg = MsgUpdateZkExecutionIsm::new(
            ISM_ID.to_string(),
            celestia_start_height + num_blocks,
            block_proof.bytes(),
            block_proof.public_values.as_slice().to_vec(),
            ism_client.signer_address().to_string(),
        );

        let response = ism_client.send_tx(block_proof_msg).await.unwrap();
        assert!(response.success);

        let range_out: BlockRangeExecOutput = bincode::deserialize(block_proof.public_values.as_slice()).unwrap();
        let evm_provider = ProviderBuilder::new().connect_http(Url::from_str(EV_RPC).unwrap());
        prover_height = Some(celestia_start_height + num_blocks + 1);

        // don't prove messages if no progress was made
        if range_out.new_height <= trusted_height + 1 {
            continue;
        }

        let mut snapshot = hyperlane_snapshot_store.get_snapshot(snapshot_index).unwrap();
        let message_proof = prove_messages(
            trusted_height + 1,
            range_out.new_height,
            &evm_provider.clone(),
            &MockStateQueryProvider::new(evm_provider),
            client.clone(),
            snapshot.clone(),
        )
        .await
        .unwrap();

        let message_proof_msg = MsgSubmitMessages::new(
            ISM_ID.to_string(),
            celestia_start_height + num_blocks,
            message_proof.0.bytes(),
            message_proof.0.public_values.as_slice().to_vec(),
            ism_client.signer_address().to_string(),
        );

        let response = ism_client.send_tx(message_proof_msg).await.unwrap();
        assert!(response.success);

        // submit all now verified messages to hyperlane
        for message in message_proof.1.clone() {
            let message_hex = alloy::hex::encode(encode_hyperlane_message(&message.message).unwrap());
            let msg = MsgProcessMessage::new(
                "0x68797065726c616e650000000000000000000000000000000000000000000000".to_string(),
                ism_client.signer_address().to_string(),
                alloy::hex::encode(vec![]), // empty metadata; messages are pre-authorized before submission
                message_hex,
            );
            let response = ism_client.send_tx(msg).await.unwrap();
            assert!(response.success);
        }

        // insert messages into snapshot to get new snapshot for next proof
        for message in message_proof.1 {
            snapshot
                .insert(message.message.id())
                .expect("Failed to insert messages into snapshot");
        }

        // store snapshot
        hyperlane_snapshot_store
            .insert_snapshot(snapshot_index + 1, snapshot)
            .expect("Failed to insert snapshot into snapshot store");

        snapshot_index += 1;
    }
}

// todo: find a place for this function and remove it from the binaries
async fn inclusion_height(block_number: u64) -> anyhow::Result<u64> {
    let mut client = StoreServiceClient::connect(e2e::config::e2e::SEQUENCER_URL).await?;
    let req = GetMetadataRequest {
        key: format!("rhb/{block_number}/d"),
    };

    let resp = client.get_metadata(req).await?;
    let height = u64::from_le_bytes(resp.into_inner().value[..8].try_into()?);

    Ok(height)
}

fn celestia_height(base_url: &str) -> anyhow::Result<u64> {
    let url = format!("{}/status", base_url.trim_end_matches('/'));
    let v: serde_json::Value = reqwest::blocking::get(&url)?.json()?;
    let h = v
        .pointer("/result/sync_info/latest_block_height")
        .and_then(|x| x.as_str())
        .ok_or("missing result.sync_info.latest_block_height")
        .unwrap();
    Ok(h.parse::<u64>()?)
}

async fn query_ism(ism_client: &CelestiaIsmClient) -> anyhow::Result<(Vec<u8>, u64)> {
    let resp = ism_client.ism(QueryIsmRequest { id: ISM_ID.to_string() }).await?;

    let ism = resp.ism.expect("ZKISM not found");
    let trusted_root = ism.state_root;
    let trusted_height = ism.height;
    Ok((trusted_root, trusted_height))
}

#[tokio::test]
async fn test() {
    let height = inclusion_height(264).await.unwrap();
    println!("Height: {height}");
}
