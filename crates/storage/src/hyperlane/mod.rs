pub mod message;
pub mod snapshot;

#[cfg(test)]
mod tests {
    use crate::{hyperlane::{message::HyperlaneMessageStore, snapshot::HyperlaneSnapshotStore}, APP_HOME};
    use ev_zkevm_types::{
        StoredHyperlaneMessage, hyperlane::decode_hyperlane_message, programs::hyperlane::tree::MerkleTree,
    };

    const DEFAULT_MESSAGE: &str = "0300000009000004d2000000000000000000000000a7578551bae89a96c3365b93493ad2d4ebcbae9700010f2c726f757465725f617070000000000000000000000000000100000000000000000000000000000000000000006a809b36caf0d46a935ee76835065ec5a8b3cea700000000000000000000000000000000000000000000000000000000000003e8";

    #[test]
    fn test_insert_message() {
        let message_storage_path = dirs::home_dir()
            .expect("cannot find home directory")
            .join(APP_HOME)
            .join("data")
            .join("messages.db");
        let store = HyperlaneMessageStore::new(message_storage_path).unwrap();
        let message = hex::decode(DEFAULT_MESSAGE).unwrap();
        let current_index = store.current_index().unwrap();
        let message = decode_hyperlane_message(&message).unwrap();
        let message = StoredHyperlaneMessage::new(message, None);
        store.insert_message(current_index, message.clone()).unwrap();
        let retrieved_message = store.get_by_block(current_index).unwrap();
        assert_eq!(retrieved_message.first().unwrap().message, message.message);
        store.prune_all().unwrap();
    }

    #[test]
    fn test_insert_message_by_block() {
        let message = hex::decode(DEFAULT_MESSAGE).unwrap();
        let message_storage_path = dirs::home_dir()
            .expect("cannot find home directory")
            .join(APP_HOME)
            .join("data")
            .join("messages.db");
        let store = HyperlaneMessageStore::new(message_storage_path).unwrap();
        let message = decode_hyperlane_message(&message).unwrap();
        let message = StoredHyperlaneMessage::new(message, Some(100));
        let current_index = store.current_index().unwrap();
        store.insert_message(current_index, message.clone()).unwrap();
        let retrieved_messages = store.get_by_block(100).unwrap();
        assert_eq!(retrieved_messages.len(), 1);
        assert_eq!(retrieved_messages[0].message, message.message);
        store.prune_all().unwrap();
    }

    #[test]
    fn test_insert_snapshot() {
        let snapshot_storage_path = dirs::home_dir()
            .expect("cannot find home directory")
            .join(APP_HOME)
            .join("data")
            .join("snapshots.db");
        let store = HyperlaneSnapshotStore::new(snapshot_storage_path).unwrap();
        let snapshot = MerkleTree::default();
        let current_index = store.current_index().unwrap();
        store.insert_snapshot(current_index, snapshot.clone()).unwrap();
        let retrieved_snapshot = store.get_snapshot(current_index).unwrap();
        assert_eq!(retrieved_snapshot, snapshot);
        store.prune_all().unwrap();
    }
}
