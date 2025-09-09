pub mod storage;

#[cfg(test)]
mod tests {
    use crate::{Storage, hyperlane_messages::storage::HyperlaneMessageStore};
    use evm_state_types::{StoredHyperlaneMessage, hyperlane::decode_hyperlane_message};
    use std::{env, path::PathBuf};

    #[test]
    fn test_insert_message() {
        dotenvy::dotenv().ok();
        let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());
        let workspace_path = manifest_dir.parent().unwrap().parent().unwrap();
        let relative = env::var("HYPERLANE_MESSAGE_STORE").expect("HYPERLANE_MESSAGE_STORE must be set");
        let path = workspace_path.join(relative);

        let store = HyperlaneMessageStore::from_path_relative(&path).unwrap();
        let message = hex::decode("0300000009000004d2000000000000000000000000a7578551bae89a96c3365b93493ad2d4ebcbae9700010f2c726f757465725f617070000000000000000000000000000100000000000000000000000000000000000000006a809b36caf0d46a935ee76835065ec5a8b3cea700000000000000000000000000000000000000000000000000000000000003e8").unwrap();
        let current_index = store.current_index().unwrap();
        let message = decode_hyperlane_message(&message).unwrap();
        let message = StoredHyperlaneMessage::new(message, None);
        store.insert_message(current_index, message.clone()).unwrap();
        let retrieved_message = store.get_message(current_index).unwrap();
        assert_eq!(retrieved_message.message, message.message);
    }
}
