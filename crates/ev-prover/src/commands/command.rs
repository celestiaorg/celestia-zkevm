use std::fs;

use anyhow::{anyhow, bail, Result};
use celestia_grpc_client::types::ClientConfig;
use celestia_grpc_client::{
    CelestiaIsmClient, MsgAnnounceValidator, MsgCreateCollateralToken, MsgCreateMailbox,
    MsgCreateMerkleRootMultisigIsm, MsgCreateMerkleTreeHook, MsgCreateNoopHook,
    MsgCreateNoopIsm, MsgCreateZkExecutionIsm, MsgEnrollRemoteRouter, RemoteRouter,
};
use celestia_rpc::{client::Client as CelestiaClient, HeaderClient};
use celestia_types::nmt::Namespace;
use ev_types::v1::get_block_request::Identifier;
use ev_types::v1::store_service_client::StoreServiceClient;
use ev_types::v1::GetBlockRequest;
use sp1_sdk::{HashableKey, Prover, ProverClient};
use tonic::Request;
use tracing::info;

use crate::commands::cli::{HookType, IsmType, VERSION};
use crate::config::config::{
    Config, APP_HOME, CONFIG_DIR, CONFIG_FILE, DEFAULT_GENESIS_JSON, GENESIS_FILE, GROTH16_VK,
};
use crate::prover::programs::message::EV_HYPERLANE_ELF;
use crate::prover::programs::range::EV_RANGE_EXEC_ELF;
use crate::server::start_server;

pub fn init() -> Result<()> {
    let home_dir = dirs::home_dir().expect("cannot find home directory").join(APP_HOME);

    if !home_dir.exists() {
        info!("creating home directory at {home_dir:?}");
        fs::create_dir_all(&home_dir)?;
    }

    let config_dir = home_dir.join(CONFIG_DIR);
    if !config_dir.exists() {
        fs::create_dir_all(&config_dir)?;
    }

    let config_path = config_dir.join(CONFIG_FILE);
    if !config_path.exists() {
        info!("creating default config at {config_path:?}");
        let config = Config::default();
        let yaml = serde_yaml::to_string(&config)?;
        fs::write(config_path, yaml)?;
    } else {
        info!("config file already exists at {config_path:?}");
    }

    let genesis_path = config_dir.join(GENESIS_FILE);
    if !genesis_path.exists() {
        info!("writing embedded genesis to {genesis_path:?}");
        fs::write(&genesis_path, DEFAULT_GENESIS_JSON)?;
    }

    Ok(())
}

pub async fn start() -> Result<()> {
    let config_path = dirs::home_dir()
        .expect("cannot find home directory")
        .join(APP_HOME)
        .join(CONFIG_DIR)
        .join(CONFIG_FILE);

    if !config_path.exists() {
        bail!("config file not found at {}", config_path.display());
    }

    info!("reading config file at {}", config_path.display());
    let config_yaml = fs::read_to_string(&config_path)?;
    let config: Config = serde_yaml::from_str(&config_yaml)?;

    info!("starting gRPC server at {}", config.grpc_address);
    start_server(config).await?;

    Ok(())
}

pub fn version() {
    println!("version: {VERSION}");
}

pub async fn create_ism(
    ism_type: IsmType,
    validators: Option<Vec<String>>,
    threshold: Option<u32>,
) -> Result<()> {
    let client_config = ClientConfig::from_env()?;
    let client = CelestiaIsmClient::new(client_config).await?;
    let signer_address = client.signer_address().to_string();

    match ism_type {
        IsmType::Zk => {
            info!("Creating ZK Execution ISM");
            create_zk_ism(&client, signer_address).await?;
        }
        IsmType::Noop => {
            info!("Creating Noop ISM");
            let msg = MsgCreateNoopIsm {
                creator: signer_address,
            };
            let res = client.send_tx(msg).await?;
            info!("Successfully created Noop ISM, tx hash: {}", res.tx_hash);
        }
        IsmType::MerkleRootMultisig => {
            info!("Creating Merkle Root Multisig ISM");
            let validators = validators.ok_or_else(|| anyhow!("Validators required for multisig ISM"))?;
            let threshold = threshold.ok_or_else(|| anyhow!("Threshold required for multisig ISM"))?;

            if threshold as usize > validators.len() {
                bail!("Threshold ({}) cannot be greater than number of validators ({})", threshold, validators.len());
            }

            let msg = MsgCreateMerkleRootMultisigIsm {
                creator: signer_address,
                validators,
                threshold,
            };
            let res = client.send_tx(msg).await?;
            info!("Successfully created Merkle Root Multisig ISM, tx hash: {}", res.tx_hash);
        }
    }

    Ok(())
}

async fn create_zk_ism(client: &CelestiaIsmClient, signer_address: String) -> Result<()> {
    let config_path = dirs::home_dir()
        .expect("cannot find home directory")
        .join(APP_HOME)
        .join(CONFIG_DIR)
        .join(CONFIG_FILE);

    if !config_path.exists() {
        bail!("config file not found at {}", config_path.display());
    }

    info!("reading config file at {}", config_path.display());
    let config_yaml = fs::read_to_string(&config_path)?;
    let config: Config = serde_yaml::from_str(&config_yaml)?;

    let mut store_client = StoreServiceClient::connect("http://127.0.0.1:7331").await?;
    let block_req = GetBlockRequest {
        identifier: Some(Identifier::Height(1)),
    };
    let block_resp = store_client.get_block(block_req).await?;
    let pub_key = block_resp
        .into_inner()
        .block
        .unwrap()
        .header
        .unwrap()
        .signer
        .unwrap()
        .pub_key;

    let sequencer_public_key = pub_key
        .get(4..)
        .ok_or_else(|| anyhow!("sequencer public key is too short"))?
        .to_vec();

    let state_resp = store_client.get_state(Request::new(())).await?;
    let (height, state_root, celestia_height) = match state_resp.into_inner().state {
        Some(state) => {
            info!(?state, "State from sequencer");
            (state.last_block_height, state.app_hash, state.da_height)
        }
        None => bail!("sequencer state response did not include state"),
    };

    if state_root.is_empty() {
        bail!("sequencer state returned an empty app hash");
    }

    if celestia_height == 0 {
        bail!("sequencer state returned zero DA height");
    }

    let raw_ns = hex::decode(config.namespace_hex)?;
    let namespace = Namespace::new_v0(raw_ns.as_ref())?;

    let celestia_rpc = format!("http://{}", config.celestia_rpc);
    let celestia_client = CelestiaClient::new(&celestia_rpc, None).await?;

    let celestia_header = celestia_client
        .header_get_by_height(celestia_height)
        .await
        .map_err(|e| anyhow!("failed to fetch celestia header at height {celestia_height}: {e}"))?;

    let celestia_header_hash = celestia_header.hash();
    let celestia_header_hash = celestia_header_hash.as_bytes().to_vec();

    let groth16_vkey = GROTH16_VK.to_vec();

    let prover = ProverClient::builder().cpu().build();
    let (_, vk) = prover.setup(EV_RANGE_EXEC_ELF);
    let state_transition_vkey = vk.hash_bytes().to_vec();

    let (_, vk) = prover.setup(EV_HYPERLANE_ELF);
    let state_membership_vkey = vk.hash_bytes().to_vec();

    let create_ism_msg = MsgCreateZkExecutionIsm::new(
        signer_address,
        state_root,
        height,
        celestia_header_hash,
        celestia_height,
        namespace.as_bytes().to_vec(),
        sequencer_public_key,
        groth16_vkey,
        state_transition_vkey,
        state_membership_vkey,
    );

    let res = client.send_tx(create_ism_msg).await?;
    info!("Successfully submitted create ZK ISM tx with hash {}", res.tx_hash);

    Ok(())
}

pub async fn create_hook(hook_type: HookType, mailbox_id: Option<String>) -> Result<()> {
    let client_config = ClientConfig::from_env()?;
    let client = CelestiaIsmClient::new(client_config).await?;
    let signer_address = client.signer_address().to_string();

    match hook_type {
        HookType::Noop => {
            info!("Creating Noop Hook");
            let msg = MsgCreateNoopHook {
                owner: signer_address,
            };
            let res = client.send_tx(msg).await?;
            info!("Successfully created Noop Hook, tx hash: {}", res.tx_hash);
        }
        HookType::MerkleTree => {
            info!("Creating Merkle Tree Hook");
            let mailbox_id = mailbox_id.ok_or_else(|| anyhow!("Mailbox ID required for Merkle Tree hook"))?;
            let msg = MsgCreateMerkleTreeHook {
                owner: signer_address,
                mailbox_id,
            };
            let res = client.send_tx(msg).await?;
            info!("Successfully created Merkle Tree Hook, tx hash: {}", res.tx_hash);
        }
    }

    Ok(())
}

pub async fn create_mailbox(
    ism_id: String,
    local_domain: u32,
    default_hook: Option<String>,
    required_hook: Option<String>,
) -> Result<()> {
    let client_config = ClientConfig::from_env()?;
    let client = CelestiaIsmClient::new(client_config).await?;
    let signer_address = client.signer_address().to_string();

    info!("Creating mailbox with ISM ID: {}", ism_id);
    let msg = MsgCreateMailbox {
        owner: signer_address,
        default_ism: ism_id,
        local_domain,
        default_hook,
        required_hook,
    };

    let res = client.send_tx(msg).await?;
    info!("Successfully created mailbox, tx hash: {}", res.tx_hash);

    Ok(())
}

pub async fn create_warp_token(mailbox_id: String, _ism_id: String, denom: String) -> Result<()> {
    let client_config = ClientConfig::from_env()?;
    let client = CelestiaIsmClient::new(client_config).await?;
    let signer_address = client.signer_address().to_string();

    info!("Creating collateral warp token with denom: {}", denom);
    let create_msg = MsgCreateCollateralToken {
        owner: signer_address.clone(),
        origin_mailbox: mailbox_id,
        origin_denom: denom,
    };

    let res = client.send_tx(create_msg).await?;
    info!("Successfully created collateral token, tx hash: {}", res.tx_hash);

    // Note: We'd need to parse the response to get the token_id to set the ISM
    // For now, we'll just log success
    info!("Note: You may need to call MsgSetToken separately to associate the ISM");

    Ok(())
}

pub async fn enroll_router(token_id: String, remote_domain: u32, remote_contract: String) -> Result<()> {
    let client_config = ClientConfig::from_env()?;
    let client = CelestiaIsmClient::new(client_config).await?;
    let signer_address = client.signer_address().to_string();

    info!("Enrolling remote router for token {} on domain {}", token_id, remote_domain);
    let msg = MsgEnrollRemoteRouter {
        owner: signer_address,
        token_id,
        remote_router: Some(RemoteRouter {
            receiver_domain: remote_domain,
            receiver_contract: remote_contract,
            gas: "0".to_string(), // Zero gas by default
        }),
    };

    let res = client.send_tx(msg).await?;
    info!("Successfully enrolled remote router, tx hash: {}", res.tx_hash);

    Ok(())
}

pub async fn announce_validator(
    validator: String,
    storage_location: String,
    signature: String,
    mailbox_id: String,
) -> Result<()> {
    let client_config = ClientConfig::from_env()?;
    let client = CelestiaIsmClient::new(client_config).await?;
    let signer_address = client.signer_address().to_string();

    info!("Announcing validator: {}", validator);
    let msg = MsgAnnounceValidator {
        validator,
        storage_location,
        signature,
        mailbox_id,
        creator: signer_address,
    };

    let res = client.send_tx(msg).await?;
    info!("Successfully announced validator, tx hash: {}", res.tx_hash);

    Ok(())
}

pub async fn deploy_stack(
    ism_id: Option<String>,
    local_domain: u32,
    use_merkle_hook: bool,
    _denom: String,
) -> Result<()> {
    info!("Deploying full Hyperlane stack...");

    // Step 1: Create or use existing ISM
    let ism_id = if let Some(id) = ism_id {
        info!("Using existing ISM: {}", id);
        id
    } else {
        info!("Creating new ZK ISM...");
        // Create ZK ISM and parse response to get ID
        // For now, we'll require the ISM ID to be provided
        bail!("ISM ID must be provided for now. Create an ISM first using 'create-ism' command.");
    };

    // Step 2: Create hook
    info!("Creating hook...");
    let hook_type = if use_merkle_hook {
        HookType::MerkleTree
    } else {
        HookType::Noop
    };

    // For MerkleTree hook, we need mailbox_id first, so we'll create it after mailbox
    let hook_id = if use_merkle_hook {
        None
    } else {
        create_hook(hook_type.clone(), None).await?;
        // We'd need to parse the response to get the hook ID
        None
    };

    // Step 3: Create mailbox
    info!("Creating mailbox...");
    create_mailbox(ism_id.clone(), local_domain, hook_id.clone(), hook_id.clone()).await?;
    // We'd need to parse the response to get the mailbox ID

    info!("Hyperlane stack deployment initiated. Check transaction logs for component IDs.");

    Ok(())
}
