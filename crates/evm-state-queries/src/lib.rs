use std::{env, str::FromStr};

use crate::hyperlane::indexer::HyperlaneIndexer;
use alloy_primitives::Address;
use alloy_provider::WsConnect;
use alloy_rpc_types::Filter;
use evm_state_types::events::Dispatch;

pub mod hyperlane;

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
}

impl Default for HyperlaneIndexer {
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
