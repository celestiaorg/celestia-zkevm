use alloy_primitives::B256;
use alloy_primitives::{U256, keccak256};
use alloy_sol_types::sol;
use anyhow::{Result, anyhow};

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

#[derive(Debug, Clone)]
pub struct HyperlaneMessage<'a> {
    pub version: u8,
    pub nonce: u32,
    pub origin: u32,
    pub sender: [u8; 32],
    pub destination: u32,
    pub recipient: [u8; 32],
    pub body: &'a [u8],
    pub id: B256,
}

pub fn decode_hyperlane_message(message: &[u8]) -> Result<HyperlaneMessage<'_>> {
    const VERSION_OFFSET: usize = 0;
    const NONCE_OFFSET: usize = 1;
    const ORIGIN_OFFSET: usize = 5;
    const SENDER_OFFSET: usize = 9;
    const DESTINATION_OFFSET: usize = 41;
    const RECIPIENT_OFFSET: usize = 45;
    const BODY_OFFSET: usize = 77;

    if message.len() < BODY_OFFSET {
        return Err(anyhow!("message too short: {} < {}", message.len(), BODY_OFFSET));
    }

    let version = message[VERSION_OFFSET];
    let nonce = u32::from_be_bytes(message[NONCE_OFFSET..ORIGIN_OFFSET].try_into().unwrap());
    let origin = u32::from_be_bytes(message[ORIGIN_OFFSET..SENDER_OFFSET].try_into().unwrap());
    let mut sender = [0u8; 32];
    sender.copy_from_slice(&message[SENDER_OFFSET..DESTINATION_OFFSET]);
    let destination = u32::from_be_bytes(message[DESTINATION_OFFSET..RECIPIENT_OFFSET].try_into().unwrap());
    let mut recipient = [0u8; 32];
    recipient.copy_from_slice(&message[RECIPIENT_OFFSET..BODY_OFFSET]);
    let body = &message[BODY_OFFSET..];
    let id = B256::from(keccak256(message));

    Ok(HyperlaneMessage {
        version,
        nonce,
        origin,
        sender,
        destination,
        recipient,
        body,
        id,
    })
}

#[derive(Debug)]
pub struct TokenMessageBody<'a> {
    pub recipient: [u8; 32],
    pub amount: U256,
    pub metadata: &'a [u8],
}

pub fn decode_token_message_body(body: &[u8]) -> Result<TokenMessageBody<'_>> {
    if body.len() < 64 {
        return Err(anyhow!("TokenMessage body too short: {}", body.len()));
    }
    let mut recipient = [0u8; 32];
    recipient.copy_from_slice(&body[0..32]);
    let amount = U256::from_be_slice(&body[32..64]);

    Ok(TokenMessageBody {
        recipient,
        amount,
        metadata: &body[64..],
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_decode_hyperlane_message() {
        let message = [
            3, 0, 0, 0, 9, 0, 0, 4, 210, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 167, 87, 133, 81, 186, 232, 154, 150, 195,
            54, 91, 147, 73, 58, 210, 212, 235, 203, 174, 151, 0, 1, 15, 44, 114, 111, 117, 116, 101, 114, 95, 97, 112,
            112, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            106, 128, 155, 54, 202, 240, 212, 106, 147, 94, 231, 104, 53, 6, 94, 197, 168, 179, 206, 167, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 3, 232,
        ];
        let message_decoded = decode_hyperlane_message(&message).unwrap();
        //println!("Decoded: {:?}", message_decoded);

        let _message_body_decoded = decode_token_message_body(&message_decoded.body).unwrap();
        //println!("Body decoded: {:?}", _message_body_decoded);
    }
}
