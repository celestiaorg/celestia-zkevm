pub mod storage;

#[cfg(test)]
mod tests {
    use evm_state_types::{StoredHyperlaneMessage, hyperlane::decode_hyperlane_message};

    use crate::{Storage, hyperlane_messages::storage::HyperlaneMessageStore};

    #[test]
    fn test_insert_message() {
        let store = HyperlaneMessageStore::default().unwrap();
        let message = hex::decode("0300000009000004d2000000000000000000000000a7578551bae89a96c3365b93493ad2d4ebcbae9700010f2c726f757465725f617070000000000000000000000000000100000000000000000000000000000000000000006a809b36caf0d46a935ee76835065ec5a8b3cea700000000000000000000000000000000000000000000000000000000000003e8").unwrap();
        let current_index = store.current_index().unwrap();
        let message = decode_hyperlane_message(&message).unwrap();
        let message = StoredHyperlaneMessage::new(message, None);
        store.insert_message(current_index, message.clone()).unwrap();
        let retrieved_message = store.get_message(current_index).unwrap();
        assert_eq!(retrieved_message.message, message.message);
    }
}
