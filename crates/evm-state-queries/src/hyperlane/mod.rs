pub mod indexer;

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use alloy_rpc_types::Filter;
    use evm_state_types::events::Dispatch;
    use storage::{Storage, hyperlane_messages::storage::HyperlaneMessageStore};

    use crate::hyperlane::indexer::HyperlaneIndexer;

    #[test]
    fn test_get_message_from_db() {
        // this test will fail if the db is empty, make sure there is at least one message in the db at testdata/messages/hyperlane
        let store = HyperlaneMessageStore::from_env().unwrap();
        let message = store.get_message(0).unwrap();
        println!("Message decoded: {:?}", message);
    }

    #[tokio::test]
    async fn test_run_indexer() {
        let indexer = HyperlaneIndexer::default();
        let message_store = Arc::new(HyperlaneMessageStore::from_env().unwrap());
        message_store.prune_all().unwrap();

        let from_block = 0;
        let to_block = 10000;

        let filter = Filter::new()
            .address(indexer.contract_address)
            .event(&Dispatch::id())
            .from_block(from_block)
            .to_block(to_block);
        indexer.index(message_store, filter).await.unwrap();
    }
}
