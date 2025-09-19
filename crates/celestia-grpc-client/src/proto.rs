//! Generated protobuf types for Celestia zkISM messages

// Include the generated protobuf types
pub mod celestia {
    pub mod zkism {
        pub mod v1 {
            include!(concat!(env!("OUT_DIR"), "/celestia.zkism.v1.rs"));
        }
    }
}

// Re-export the message types for convenience
pub use celestia::zkism::v1::{
    msg_client::MsgClient,
    MsgSubmitMessages as ProtobufMsgSubmitMessages,
    MsgSubmitMessagesResponse as ProtobufMsgSubmitMessagesResponse,
    MsgUpdateZkExecutionIsm as ProtobufMsgUpdateZkExecutionIsm,
    MsgUpdateZkExecutionIsmResponse as ProtobufMsgUpdateZkExecutionIsmResponse,
};