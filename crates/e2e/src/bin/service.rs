use alloy_primitives::{FixedBytes, hex::FromHex};
use alloy_provider::ProviderBuilder;
use celestia_grpc_client::{
    MsgProcessMessage, MsgSubmitMessages, MsgUpdateZkExecutionIsm, QueryIsmRequest, client::CelestiaIsmClient,
};
use e2e::config::e2e::ISM_ID;
use e2e::config::other::EV_RPC;
use e2e::prover::message::prove_messages;
use e2e::{config::e2e::TARGET_HEIGHT, prover::block::prove_blocks};
use ev_state_queries::MockStateQueryProvider;
use ev_types::v1::{GetMetadataRequest, store_service_client::StoreServiceClient};
use ev_zkevm_types::hyperlane::encode_hyperlane_message;
use sp1_sdk::{EnvProver, ProverClient};
use std::{str::FromStr, sync::Arc};
use storage::hyperlane::snapshot::HyperlaneSnapshotStore;
use url::Url;

// prove once every 10 blocks
const PROVER_INTERVAL: u64 = 10;

#[tokio::main]
async fn main() {
    rustls::crypto::aws_lc_rs::default_provider()
        .install_default()
        .expect("Failed to set default crypto provider");
    dotenvy::dotenv().ok();

    // instantiate ISM client for submitting payloads and querying state
    let ism_client = CelestiaIsmClient::from_env().await.unwrap();
    let client: Arc<EnvProver> = Arc::new(ProverClient::from_env());
    let mut snapshot_index = 0;

    let snapshot_storage_path = dirs::home_dir()
        .expect("cannot find home directory")
        .join(".ev-prover")
        .join("data")
        .join("snapshots.db");
    let hyperlane_snapshot_store = Arc::new(HyperlaneSnapshotStore::new(snapshot_storage_path).unwrap());
    hyperlane_snapshot_store.reset_db().unwrap();

    loop {
        // get trustd state from ISM
        let (trusted_root, trusted_height) = query_ism(&ism_client).await.unwrap();
        let target_inclusion_height = celestia_height("http://localhost:26657").unwrap();
        if target_inclusion_height < inclusion_height(trusted_height).await.unwrap() + PROVER_INTERVAL {
            continue;
        }
        let start_height = inclusion_height(trusted_height).await.unwrap() + 1;
        // prove at most PROVER_INTERVAL blocks at a time
        let num_blocks = (target_inclusion_height - start_height).min(PROVER_INTERVAL);

        println!(
            "ISM at height {} Proving block {} up to {}",
            trusted_height,
            start_height,
            start_height + num_blocks
        );

        let block_proof = prove_blocks(
            start_height,
            trusted_height,
            num_blocks,
            &mut trusted_root.into(),
            client.clone(),
        )
        .await
        .expect("Failed to prove blocks");

        let block_proof_msg = MsgUpdateZkExecutionIsm::new(
            ISM_ID.to_string(),
            target_inclusion_height,
            block_proof.bytes(),
            block_proof.public_values.as_slice().to_vec(),
            ism_client.signer_address().to_string(),
        );

        let response = ism_client.send_tx(block_proof_msg).await.unwrap();
        assert!(response.success);

        let evm_provider = ProviderBuilder::new().connect_http(Url::from_str(EV_RPC).unwrap());

        let mut snapshot = hyperlane_snapshot_store.get_snapshot(snapshot_index).unwrap();
        let message_proof = prove_messages(
            trusted_height + 1,
            TARGET_HEIGHT,
            &evm_provider.clone(),
            &MockStateQueryProvider::new(evm_provider),
            client.clone(),
            snapshot.clone(),
        )
        .await
        .unwrap();

        let message_proof_msg = MsgSubmitMessages::new(
            ISM_ID.to_string(),
            TARGET_HEIGHT,
            message_proof.0.bytes(),
            message_proof.0.public_values.as_slice().to_vec(),
            ism_client.signer_address().to_string(),
        );

        let response = ism_client.send_tx(message_proof_msg).await.unwrap();
        assert!(response.success);

        // don't prove if no messages occurred
        if message_proof.1.is_empty() {
            continue;
        }

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

async fn query_ism(ism_client: &CelestiaIsmClient) -> anyhow::Result<(FixedBytes<32>, u64)> {
    let resp = ism_client
        .ism(QueryIsmRequest { id: ISM_ID.to_string() })
        .await
        .unwrap();

    let ism = resp.ism.expect("ZKISM not found");
    let trusted_root = alloy::hex::encode(ism.state_root);
    let trusted_height = ism.height;
    Ok((FixedBytes::from_hex(trusted_root).unwrap(), trusted_height))
}
