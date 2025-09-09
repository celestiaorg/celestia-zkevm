pub mod indexer;

#[cfg(test)]
mod tests {
    use crate::hyperlane::indexer::HyperlaneIndexer;
    use alloy_provider::ProviderBuilder;
    use alloy_rpc_types::Filter;
    use evm_state_types::events::Dispatch;
    use std::{env, path::PathBuf, sync::Arc};
    use storage::{Storage, hyperlane_messages::storage::HyperlaneMessageStore};

    #[tokio::test]
    /* Context
        We want to generate proofs for events that occurred between one finalized block and another (latest)
        finalized block. Therefore we can query the relevant events using the filter and insert them into the tree,
        starting at a previous checkpoint (or the empty tree).

        When using the indexer we must ensure that we start indexing from the first block that includes a message to
        be able to replay all inserts into the tree and obtain the correct branch that exists on-chain.

        A storage proof to that branch will be included and verified inside the circuit against the root of said (latest)
        finalized block. It is advisible to maintain a window / root history on-chain so that this proof will verify even
        if a new block was posted in the meantime.
    */
    async fn test_run_indexer() {
        dotenvy::dotenv().ok();
        let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());
        let workspace_path = manifest_dir.parent().unwrap().parent().unwrap();
        let relative = env::var("HYPERLANE_MESSAGE_STORE").expect("HYPERLANE_MESSAGE_STORE must be set");
        let path = workspace_path.join(relative);
        let indexer = HyperlaneIndexer::default();

        let message_store = Arc::new(HyperlaneMessageStore::from_path_relative(&path).unwrap());
        message_store.prune_all().unwrap();

        let start_height = 0;
        let end_height = 10000;

        let provider = Arc::new(ProviderBuilder::new().connect_ws(indexer.socket.clone()).await.unwrap());
        let filter = Filter::new()
            .address(indexer.contract_address)
            .event(&Dispatch::id())
            .from_block(start_height)
            .to_block(end_height);
        indexer.index(message_store, filter, provider).await.unwrap();
    }
}
