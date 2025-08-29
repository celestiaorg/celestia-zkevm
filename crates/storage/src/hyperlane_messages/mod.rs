pub mod storage;

#[cfg(test)]
mod tests {
    use crate::{Storage, hyperlane_messages::storage::HyperlaneMessageStore};

    #[test]
    fn test_insert_message() {
        let store = HyperlaneMessageStore::default().unwrap();
        let message = [
            3, 0, 0, 0, 9, 0, 0, 4, 210, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 167, 87, 133, 81, 186, 232, 154, 150, 195,
            54, 91, 147, 73, 58, 210, 212, 235, 203, 174, 151, 0, 1, 15, 44, 114, 111, 117, 116, 101, 114, 95, 97, 112,
            112, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            106, 128, 155, 54, 202, 240, 212, 106, 147, 94, 231, 104, 53, 6, 94, 197, 168, 179, 206, 167, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 3, 232,
        ];
        let current_index = store.current_index().unwrap();
        store.insert_message(current_index, &message).unwrap();
        let retrieved_message = store.get_message(current_index).unwrap();
        assert_eq!(retrieved_message, message);
    }
}
