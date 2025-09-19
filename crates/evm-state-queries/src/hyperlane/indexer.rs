/// This service listens for Dispatch events emitted from the Mailbox contract
/// using the reth websocket.
/// Events are then processed and inserted into the storage (rocksDB)
use alloy_primitives::Address;
use alloy_provider::{Provider, WsConnect, fillers::FillProvider};
use alloy_rpc_types::Filter;
use alloy_sol_types::SolEvent;
use anyhow::Result;
use evm_state_types::{
    StoredHyperlaneMessage,
    events::{Dispatch, DispatchEvent},
    hyperlane::decode_hyperlane_message,
};
use std::{env, str::FromStr, sync::Arc};
use storage::hyperlane::message::HyperlaneMessageStore;

type DefaultProvider = FillProvider<
    alloy_provider::fillers::JoinFill<
        alloy_provider::Identity,
        alloy_provider::fillers::JoinFill<
            alloy_provider::fillers::GasFiller,
            alloy_provider::fillers::JoinFill<
                alloy_provider::fillers::BlobGasFiller,
                alloy_provider::fillers::JoinFill<
                    alloy_provider::fillers::NonceFiller,
                    alloy_provider::fillers::ChainIdFiller,
                >,
            >,
        >,
    >,
    alloy_provider::RootProvider,
>;

/// HyperlaneIndexer is a service that indexes Hyperlane messages from the Dispatch event emitted from the Mailbox contract.
pub struct HyperlaneIndexer {
    pub socket: WsConnect,
    pub contract_address: Address,
    pub filter: Filter,
}

/// Implementation of the HyperlaneIndexer that queries the network for Dispatch messages.
impl HyperlaneIndexer {
    pub fn new(socket: WsConnect, contract_address: Address, filter: Filter) -> Self {
        Self {
            socket,
            contract_address,
            filter,
        }
    }

    pub fn from_env() -> Result<Self> {
        dotenvy::dotenv().ok();
        let reth_url =
            env::var("RETH_WS_URL").map_err(|_| anyhow::anyhow!("RETH_WS_URL environment variable not set"))?;
        let socket = WsConnect::new(reth_url);

        let mailbox_addr = env::var("MAILBOX_CONTRACT_ADDRESS")
            .map_err(|_| anyhow::anyhow!("MAILBOX_CONTRACT_ADDRESS environment variable not set"))?;
        let contract_address =
            Address::from_str(&mailbox_addr).map_err(|e| anyhow::anyhow!("Invalid mailbox contract address: {}", e))?;

        let filter = Filter::new().address(contract_address).event(&Dispatch::id());
        Ok(Self::new(socket, contract_address, filter))
    }

    pub async fn index(&self, message_store: Arc<HyperlaneMessageStore>, provider: Arc<DefaultProvider>) -> Result<()> {
        let logs = provider.get_logs(&self.filter).await?;
        for log in logs {
            match Dispatch::decode_log_data(log.data()) {
                Ok(event) => {
                    let dispatch_event: DispatchEvent = event.into();
                    let current_index = message_store.current_index()?;
                    let next_index = current_index;
                    let hyperlane_message =
                        decode_hyperlane_message(&dispatch_event.message).expect("Failed to decode Hyperlane message");
                    let stored_message = StoredHyperlaneMessage::new(hyperlane_message, log.block_number);
                    message_store.insert_message(next_index, stored_message).unwrap();
                    println!("Inserted Hyperlane Message at index: {next_index}");
                }
                Err(e) => {
                    eprintln!("Failed to decode Dispatch Event: {e:?}");
                }
            }
        }

        Ok(())
    }
}

impl Default for HyperlaneIndexer {
    fn default() -> Self {
        let socket = WsConnect::new("ws://127.0.0.1:8546");
        let contract_address = Address::from_str("0xb1c938f5ba4b3593377f399e12175e8db0c787ff").unwrap();
        let filter = Filter::new()
            .address(contract_address)
            .event(&Dispatch::id())
            .from_block(0)
            .to_block(10000);
        Self {
            socket,
            contract_address,
            filter,
        }
    }
}

/* Query all messages in range using cast
cast logs \
    --rpc-url http://127.0.0.1:8545 \
    --from-block 0 --to-block 1000 \
    --address 0xb1c938F5BA4B3593377F399e12175e8db0C787Ff \
    "Dispatch(address,uint32,bytes32,bytes)"
*/
