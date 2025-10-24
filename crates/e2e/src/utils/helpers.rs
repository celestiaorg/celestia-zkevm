use alloy_provider::{Provider, ProviderBuilder};
use alloy_rpc_types::TransactionRequest;
use alloy_signer_local::PrivateKeySigner;
use alloy_sol_types::{SolCall, sol};
use eyre::Result;
use std::str::FromStr;
use url::Url;

use alloy::hex::FromHex;
use alloy::primitives::{Address, FixedBytes, U256};

sol! {
    interface IERC20Bridge {
        function transferRemote(
            uint32 destinationDomain,
            bytes32 recipient,
            uint256 amount
        ) external returns (bytes32);
    }
}

pub async fn transfer_back() -> Result<u64> {
    let rpc_url: Url = "http://localhost:8545".parse()?;
    let private_key = "0x82bfcfadbf1712f6550d8d2c00a39f05b33ec78939d0167be2a737d691f33a6a";
    let contract_addr: Address = "0x345a583028762De4d733852c9D4f419077093A48".parse()?;

    let signer = PrivateKeySigner::from_str(private_key)?;
    let provider = ProviderBuilder::new().wallet(signer).connect_http(rpc_url);

    let dest_domain: u32 = 69420;
    let recipient = FixedBytes::<32>::from_hex("0x0000000000000000000000006A809B36CAF0D46A935EE76835065EC5A8B3CEA7")?;
    let amount = U256::from(1000u64);

    // encode the calldata manually
    let call = IERC20Bridge::transferRemoteCall {
        destinationDomain: dest_domain,
        recipient,
        amount,
    };
    let calldata = call.abi_encode();

    // build and send the tx
    let tx = TransactionRequest::default().to(contract_addr).input(calldata.into());

    let pending = provider.send_transaction(tx).await?;
    let receipt = pending.get_receipt().await?;
    // transaction must be successful
    assert!(receipt.status(), "Transfer back failed");
    Ok(receipt.block_number.unwrap())
}
