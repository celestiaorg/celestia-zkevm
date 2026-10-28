use ev_zkevm_types::hyperlane::HyperlaneMessage;
use serde::{Deserialize, Serialize};

pub mod message;
pub mod snapshot;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct StoredHyperlaneMessage {
    pub block_number: Option<u64>,
    pub message: HyperlaneMessage,
}

impl StoredHyperlaneMessage {
    pub fn new(message: HyperlaneMessage, block_number: Option<u64>) -> Self {
        Self { block_number, message }
    }
}

#[cfg(test)]
mod tests {
    use crate::hyperlane::{
        StoredHyperlaneMessage,
        message::HyperlaneMessageStore,
        snapshot::{HyperlaneSnapshot, HyperlaneSnapshotStore},
    };
    use ev_zkevm_types::{hyperlane::decode_hyperlane_message, hyperlane::merkle::MerkleTree};
    use tempfile::TempDir;

    const DEFAULT_MESSAGE: &str = "0300000009000004d2000000000000000000000000a7578551bae89a96c3365b93493ad2d4ebcbae9700010f2c726f757465725f617070000000000000000000000000000100000000000000000000000000000000000000006a809b36caf0d46a935ee76835065ec5a8b3cea700000000000000000000000000000000000000000000000000000000000003e8";

    #[test]
    fn test_insert_messages() {
        let tmp = TempDir::new().expect("cannot create temp directory");
        let message = hex::decode(DEFAULT_MESSAGE).unwrap();
        let storage_path = dirs::home_dir()
            .expect("cannot find home directory")
            .join(&tmp)
            .join("data");
        let store = HyperlaneMessageStore::from_path(storage_path).unwrap();
        let message = decode_hyperlane_message(&message).unwrap();
        let message = StoredHyperlaneMessage::new(message, Some(100));
        let mut current_index = store.current_index().unwrap();
        for _i in 0..10 {
            let message = StoredHyperlaneMessage::new(message.message.clone(), Some(100));
            store.insert_message(current_index, message.clone()).unwrap();
            current_index += 1;
        }
        for i in 0..200 {
            let retrieved_messages = store.get_by_block(i).unwrap();
            if i == 100 {
                println!("I: {i}");
                assert_eq!(retrieved_messages.len(), 10);
                assert_eq!(retrieved_messages[0].message, message.message);
            } else {
                println!("I: {i}");
                assert_eq!(retrieved_messages.len(), 0);
            }
        }
        store.reset_db().unwrap();
    }

    #[test]
    fn test_get_up_to_block() {
        let tmp = TempDir::new().expect("cannot create temp directory");
        let storage_path = dirs::home_dir()
            .expect("cannot find home directory")
            .join(&tmp)
            .join("data");
        let store = HyperlaneMessageStore::from_path(storage_path).unwrap();
        let message = decode_hyperlane_message(&hex::decode(DEFAULT_MESSAGE).unwrap()).unwrap();
        for (index, block) in [(0, 5), (1, 5), (2, 7), (3, 300)] {
            let mut message = message.clone();
            message.nonce = index as u32;
            store
                .insert_message(index, StoredHyperlaneMessage::new(message, Some(block)))
                .unwrap();
        }

        let nonces = |block| -> Vec<u32> {
            store
                .get_up_to_block(block)
                .unwrap()
                .into_iter()
                .map(|m| m.message.nonce)
                .collect()
        };
        assert_eq!(nonces(4), Vec::<u32>::new());
        assert_eq!(nonces(5), vec![0, 1]);
        assert_eq!(nonces(299), vec![0, 1, 2]);
        assert_eq!(nonces(300), vec![0, 1, 2, 3]);
        store.reset_db().unwrap();
    }

    #[test]
    fn test_insert_snapshot() {
        let tmp = TempDir::new().expect("cannot create temp directory");
        let storage_path = dirs::home_dir()
            .expect("cannot find home directory")
            .join(&tmp)
            .join("data");
        let store = HyperlaneSnapshotStore::new(storage_path, None).unwrap();
        let snapshot = MerkleTree::default();
        let current_index = store.current_index().unwrap();
        store
            .insert_snapshot(current_index, HyperlaneSnapshot::new(0, snapshot.clone()))
            .unwrap();
        let retrieved_snapshot = store.get_snapshot(current_index).unwrap();
        assert_eq!(retrieved_snapshot, HyperlaneSnapshot::new(0, snapshot));
        store.reset_db().unwrap();
    }
}
