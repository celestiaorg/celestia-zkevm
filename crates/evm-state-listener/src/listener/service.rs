use std::sync::Arc;

/// This service listens for Dispatch events emitted from the Mailbox contract
/// using the reth websocket.
/// Events are then processed and inserted into the storage (rocksDB)
use alloy_primitives::Address;
use alloy_provider::{Provider, ProviderBuilder, WsConnect};
use alloy_rpc_types::Filter;
use alloy_sol_types::SolEvent;
use anyhow::Result;
use evm_state_types::{Dispatch, DispatchEvent};
use storage::hyperlane_messages::storage::HyperlaneMessageStore;
use tokio_stream::StreamExt;
pub struct HyperlaneEventListener {
    pub socket: WsConnect,
    pub contract_address: Address,
    pub filter: Filter,
}

impl HyperlaneEventListener {
    pub fn new(socket: WsConnect, contract_address: Address, filter: Filter) -> Self {
        Self {
            socket,
            contract_address,
            filter,
        }
    }

    pub async fn start(&self, message_store: Arc<HyperlaneMessageStore>) -> Result<()> {
        let provider = ProviderBuilder::new().connect_ws(self.socket.clone()).await?;
        let filter = Filter::new().address(self.contract_address).event(&Dispatch::id());
        let mut sub = provider.subscribe_logs(&filter).await?.into_stream();
        println!("Subscribed, waiting for deposit events...");
        let handle = tokio::spawn(async move {
            while let Some(log) = sub.next().await {
                match Dispatch::decode_log_data(log.data()) {
                    Ok(event) => {
                        let dispatch_event: DispatchEvent = event.into();
                        let current_index = message_store.current_index().unwrap();
                        // Insert the new message into the persistent service database
                        message_store
                            .insert_message(current_index, &dispatch_event.message)
                            .unwrap();
                        println!("Inserted Hyperlane Message at index: {}", current_index);
                    }
                    Err(e) => {
                        eprintln!("Failed to decode Dispatch Event: {:?}", e);
                    }
                }
            }
        });

        handle.await?;
        Ok(())
    }
}

/* Query all messages in range using cast
cast logs \
    --rpc-url http://127.0.0.1:8545 \
    --from-block 0 --to-block 1000 \
    --address 0xb1c938F5BA4B3593377F399e12175e8db0C787Ff \
    "Dispatch(address,uint32,bytes32,bytes)"
*/
