use std::fs;
use std::result::Result::Ok;
use std::sync::Arc;

use alloy_genesis::Genesis as AlloyGenesis;
use alloy_primitives::FixedBytes;
use anyhow::{anyhow, Context, Result};
use celestia_types::nmt::Namespace;
use reth_chainspec::ChainSpec;
use rsp_primitives::genesis::Genesis;
use sp1_sdk::include_elf;
use tokio::sync::RwLock;

use crate::config::config::{Config, APP_HOME, CONFIG_DIR, GENESIS_FILE};

/// The ELF (executable and linkable format) file for the Succinct RISC-V zkVM.
pub const EVM_EXEC_ELF: &[u8] = include_elf!("evm-exec-program");

/// The ELF (executable and linkable format) file for the Succinct RISC-V zkVM.
pub const EVM_RANGE_EXEC_ELF: &[u8] = include_elf!("evm-range-exec-program");

/// The ELF (executable and linkable format) file for the Succinct RISC-V zkVM.
pub const EVM_HYPERLANE_ELF: &[u8] = include_elf!("evm-hyperlane-program");

/// AppContext encapsulates the full set of RPC endpoints and configuration
/// needed to fetch input data for execution and data availability proofs.
///
/// This separates RPC concerns from the proving logic, allowing `AppContext`
/// to be responsible for gathering the data required for the proof system inputs.
pub struct AppContext {
    pub chain_spec: Arc<ChainSpec>,
    pub genesis: Genesis,
    pub namespace: Namespace,
    pub celestia_rpc: String,
    pub evm_rpc: String,
    pub pub_key: Vec<u8>,
    pub trusted_state: RwLock<TrustedState>,
}

/// TrustedState tracks the trusted height and state root which is provided to the proof system as inputs.
/// This type is wrapped in a RwLock by the AppContext such that it can be updated safely across concurrent tasks.
/// Updates are made optimisticly using the EthClientExecutorInputs queried from the configured EVM full node.
pub struct TrustedState {
    pub(crate) height: u64,
    pub(crate) root: FixedBytes<32>,
    // the height of the snapshot used to prove the hyperlane message replay
    pub(crate) snapshot_height: u64,
}

impl TrustedState {
    pub fn new(height: u64, root: FixedBytes<32>, snapshot_height: u64) -> Self {
        Self {
            height,
            root,
            snapshot_height,
        }
    }
}

impl AppContext {
    pub fn from_config(config: Config) -> Result<Self> {
        let genesis = AppContext::load_genesis().context("Error loading app genesis")?;
        let chain_spec: Arc<ChainSpec> = Arc::new(
            (&genesis)
                .try_into()
                .map_err(|e| anyhow!("Failed to convert genesis to chain spec: {e}"))?,
        );

        let raw_ns = hex::decode(config.namespace_hex)?;
        let namespace = Namespace::new_v0(raw_ns.as_ref()).context("Failed to construct Namespace")?;
        let pub_key = hex::decode(config.pub_key)?;
        let trusted_state = RwLock::new(TrustedState::new(0, chain_spec.genesis_header().state_root, 0));

        Ok(AppContext {
            chain_spec,
            genesis,
            namespace,
            celestia_rpc: config.celestia_rpc,
            evm_rpc: config.evm_rpc,
            pub_key,
            trusted_state,
        })
    }

    fn load_genesis() -> Result<Genesis> {
        let path = dirs::home_dir()
            .expect("cannot find home directory")
            .join(APP_HOME)
            .join(CONFIG_DIR)
            .join(GENESIS_FILE);

        let raw_genesis = fs::read_to_string(path).context("Failed to read genesis file from path")?;
        let alloy_genesis: AlloyGenesis = serde_json::from_str(&raw_genesis)?;

        let genesis = Genesis::Custom(alloy_genesis.config);
        Ok(genesis)
    }
}
