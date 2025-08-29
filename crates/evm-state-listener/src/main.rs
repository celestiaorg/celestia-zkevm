use std::{env, str::FromStr, sync::Arc, time::Duration};

use crate::listener::service::HyperlaneEventListener;
use alloy_primitives::Address;
use alloy_provider::WsConnect;
use alloy_rpc_types::Filter;
use anyhow::Result;
use evm_state_types::Dispatch;
use storage::{Storage, hyperlane_messages::storage::HyperlaneMessageStore};
use tokio::task;

pub mod listener;

pub struct HyperlaneListener {
    pub socket: WsConnect,
    pub contract_address: Address,
    pub filter: Filter,
}

impl HyperlaneListener {
    pub fn new(socket: WsConnect, contract_address: Address, filter: Filter) -> Self {
        Self {
            socket,
            contract_address,
            filter,
        }
    }
    pub fn from_env() -> Self {
        dotenvy::dotenv().ok();
        let socket = WsConnect::new(env::var("RETH_WS_URL").unwrap());
        let contract_address = Address::from_str(&env::var("MAILBOX_CONTRACT_ADDRESS").unwrap()).unwrap();
        let filter = Filter::new().address(contract_address).event(&Dispatch::id());
        Self::new(socket, contract_address, filter)
    }

    // default mode, listener will start listening for Dispatch events at the contract address and socket specified in the .env file
    pub async fn start(&self) -> Result<()> {
        let store = Arc::new(HyperlaneMessageStore::from_env()?);
        let listener = HyperlaneEventListener::new(self.socket.clone(), self.contract_address, self.filter.clone());
        listener.start(store).await
    }

    // this is a test mode where we will concurrently read from the db in a separate task
    pub async fn start_with_mock_concurrency(&self) -> Result<()> {
        let store = Arc::new(HyperlaneMessageStore::from_env()?);
        let listener = HyperlaneEventListener::new(self.socket.clone(), self.contract_address, self.filter.clone());
        let store1 = store.clone();
        let listener_task = task::spawn(async move { listener.start(store1).await });
        let store2 = store.clone();
        // This query task only exists to test concurrent access to the db,
        // Later this will be replaced with the prover task and the prover task will be joined
        // with the listener
        let query_task = task::spawn(async move {
            loop {
                let first_message = store2.get_message(0)?;
                println!("First message: {:?}", first_message);
                tokio::time::sleep(Duration::from_secs(1)).await;
            }
            #[allow(unreachable_code)]
            Ok::<(), anyhow::Error>(())
        });
        let (r1, r2) = tokio::try_join!(listener_task, query_task)?;
        r1?;
        r2?;
        Ok(())
    }
}

impl Default for HyperlaneListener {
    fn default() -> Self {
        let socket = WsConnect::new("ws://127.0.0.1:8546");
        let contract_address = Address::from_str("0xb1c938f5ba4b3593377f399e12175e8db0c787ff").unwrap();
        let filter = Filter::new().address(contract_address).event(&Dispatch::id());
        Self {
            socket,
            contract_address,
            filter,
        }
    }
}

#[tokio::main]
async fn main() {
    let listener = HyperlaneListener::default();
    listener.start().await.unwrap();
}
