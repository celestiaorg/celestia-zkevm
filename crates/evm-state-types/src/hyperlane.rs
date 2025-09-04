use alloy_primitives::{U256, keccak256};
use anyhow::{Result, anyhow};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct HyperlaneMessage {
    pub version: u8,
    pub nonce: u32,
    pub origin: u32,
    pub sender: [u8; 32],
    pub destination: u32,
    pub recipient: [u8; 32],
    pub body: Vec<u8>,
}

impl HyperlaneMessage {
    pub fn id(&self) -> String {
        hex::encode(keccak256(
            encode_hyperlane_message(self).expect("failed to encode message"),
        ))
    }
}

pub fn decode_hyperlane_message(message: &[u8]) -> Result<HyperlaneMessage> {
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
    let body = message[BODY_OFFSET..].to_vec();

    Ok(HyperlaneMessage {
        version,
        nonce,
        origin,
        sender,
        destination,
        recipient,
        body,
    })
}

pub fn encode_hyperlane_message(message: &HyperlaneMessage) -> Result<Vec<u8>> {
    let mut encoded = Vec::new();
    encoded.extend_from_slice(&message.version.to_be_bytes());
    encoded.extend_from_slice(&message.nonce.to_be_bytes());
    encoded.extend_from_slice(&message.origin.to_be_bytes());
    encoded.extend_from_slice(&message.sender);
    encoded.extend_from_slice(&message.destination.to_be_bytes());
    encoded.extend_from_slice(&message.recipient);
    encoded.extend_from_slice(&message.body);
    Ok(encoded)
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
        let message = hex::decode("0300000009000004d2000000000000000000000000a7578551bae89a96c3365b93493ad2d4ebcbae9700010f2c726f757465725f617070000000000000000000000000000100000000000000000000000000000000000000006a809b36caf0d46a935ee76835065ec5a8b3cea700000000000000000000000000000000000000000000000000000000000003e8").unwrap();
        let message_decoded = decode_hyperlane_message(&message).unwrap();
        println!("Decoded: {:?}", message_decoded);

        let _message_body_decoded = decode_token_message_body(&message_decoded.body).unwrap();
        println!("Body decoded: {:?}", _message_body_decoded);
    }
}
