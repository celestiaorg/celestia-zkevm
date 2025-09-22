use alloy_sol_types::sol;

// Dispatch is emitted by the Hyperlane Mailbox contract
sol! {
    event Dispatch(
        address indexed sender,
        uint32 indexed destination,
        bytes32 indexed recipient,
        bytes message
    );
}

impl Dispatch {
    pub fn id() -> String {
        "Dispatch(address,uint32,bytes32,bytes)".to_string()
    }
}

// DispatchEvent is the Rust type of the Dispatch event
#[derive(Debug)]
pub struct DispatchEvent {
    pub sender: String,
    pub destination: u32,
    pub recipient: String,
    pub message: Vec<u8>,
}

impl From<Dispatch> for DispatchEvent {
    fn from(dispatch: Dispatch) -> Self {
        Self {
            sender: dispatch.sender.to_string(),
            destination: dispatch.destination,
            recipient: dispatch.recipient.to_string(),
            message: dispatch.message.to_vec(),
        }
    }
}
