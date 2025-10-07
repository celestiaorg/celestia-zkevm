use std::sync::Arc;

use alloy_provider::ProviderBuilder;
use ev_state_queries::hyperlane::indexer::HyperlaneIndexer;
use storage::hyperlane::message::HyperlaneMessageStore;

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
#[tokio::test]
async fn test_run_indexer() {
    let message_storage_path = dirs::home_dir()
        .expect("cannot find home directory")
        .join(".ev-prover")
        .join("data")
        .join("messages.db");
    let indexer = HyperlaneIndexer::default();
    let message_store = Arc::new(HyperlaneMessageStore::new(message_storage_path).unwrap());
    message_store.prune_all().unwrap();
    let provider = Arc::new(ProviderBuilder::new().connect_ws(indexer.socket.clone()).await.unwrap());
    indexer.index(message_store, provider).await.unwrap();
}
