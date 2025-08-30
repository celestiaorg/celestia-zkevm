pub mod indexer;

#[cfg(test)]
mod tests {
    use std::sync::Arc;

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
        indexer.index(message_store, 0, 10000).await.unwrap();
    }
}
