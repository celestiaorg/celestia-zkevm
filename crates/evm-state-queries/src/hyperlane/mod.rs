pub mod indexer;

#[cfg(test)]
mod tests {
    use crate::hyperlane::indexer::HyperlaneIndexer;
    use alloy_provider::ProviderBuilder;
    use alloy_rpc_types::Filter;
    use evm_state_types::events::Dispatch;
    use std::sync::Arc;
    use storage::{Storage, hyperlane_messages::storage::HyperlaneMessageStore};

    #[test]
    fn test_get_message_from_db() {
        // this test will fail if the db is empty, make sure there is at least one message in the db at testdata/messages/hyperlane
        let store = HyperlaneMessageStore::from_env().unwrap();
        let message = store.get_message(0).unwrap();
        println!("Message decoded: {:?}", message);
    }

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
        let indexer = HyperlaneIndexer::default();
        let message_store = Arc::new(HyperlaneMessageStore::from_env().unwrap());
        message_store.prune_all().unwrap();

        let from_block = 0;
        let to_block = 10000;

        let provider = Arc::new(ProviderBuilder::new().connect_ws(indexer.socket.clone()).await.unwrap());
        let filter = Filter::new()
            .address(indexer.contract_address)
            .event(&Dispatch::id())
            .from_block(from_block)
            .to_block(to_block);
        indexer.index(message_store, filter, provider).await.unwrap();
    }
}
