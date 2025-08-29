pub mod service;

#[cfg(test)]
mod tests {
    use evm_state_types::decode_hyperlane_message;
    use storage::{Storage, hyperlane_messages::storage::HyperlaneMessageStore};

    #[test]
    fn test_parse_event_from_db() {
        // this test will fail if the db is empty, make sure there is at least one message in the db at testdata/messages/hyperlane
        let store = HyperlaneMessageStore::from_env().unwrap();
        let message = store.get_message(0).unwrap();
        let _message_decoded = decode_hyperlane_message(&message).unwrap();
        //println!("Message decoded: {:?}", _message_decoded);
    }
}
